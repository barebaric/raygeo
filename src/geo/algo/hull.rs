use crate::constants::EPSILON_INTERSECT;
use crate::geo::algo::trace::find_external_contours;
use crate::geo::shape::line::get_line_segment_intersection;
use crate::geo::shape::point::{
    get_point_at_fraction, get_points_moving_average,
};
use crate::geo::shape::polygon::{
    get_polygon_convex_hull, get_polygon_signed_area, resample_polygon,
};
use crate::geo::types::Point;

/// Spacing between band particles, in pixels.
const BAND_SPACING: f64 = 1.0;

/// Maximum number of shrink iterations before the result is returned.
const MAX_SHRINK_ITERATIONS: usize = 1500;

/// Gravity increment consumed per shrink pass.
const GRAVITY_STEP: f64 = 0.05;

/// Weight of the gravity parameter: the number of shrink passes is
/// scaled by this factor, so a free band point traverses up to
/// `PULL_WEIGHT * free_section / 2` pixels per unit of gravity. The
/// per-pass pull stays small, keeping the pinch handling stable.
const PULL_WEIGHT: f64 = 6.0;

// The effective gravity is the squared clamp of the parameter. The
// first passes dominate the shrink (long free sections pull fast,
// the band then crawls onto the content), so a linear budget makes
// small gravities look drastic and large ones look inert. Squaring
// spreads the slider: gravity 1.0 is unchanged, low values shrink
// almost nothing, and the upper half of the range carries most of
// the visible tightening.

/// Number of bisections used to find the largest pull scale at which
/// the band still forms a simple loop.
const SELF_INTERSECTION_BISECTIONS: usize = 12;

/// Maximum rounds of anchoring fold crossings per shrink pass before
/// falling back to a global scale bisection.
const FOLD_ANCHOR_ROUNDS: usize = 8;

/// Maximum crossings collected per fold-anchoring round.
const FOLD_CROSSINGS_PER_ROUND: usize = 4096;

/// Downsampling factor of the distance field used by the content march.
const DISTANCE_FIELD_SCALE: usize = 4;

/// Maximum pull of any band point (in pixels) below which the band is
/// considered converged and the shrink stops early. The last free
/// points of deep pockets only crawl at fractions of a pixel per
/// pass; stopping earlier would cut the descent short.
const CONVERGED_PROGRESS: f64 = 0.1;

/// Window radius (in band points) over which the pull normals are
/// averaged, damping the pixel staircase of the band outline.
const NORMAL_SMOOTH_RADIUS: usize = 4;

/// Rounds of neighbour averaging applied to the pulled band each
/// pass, damping lateral waves along content edges.
const SMOOTH_ITERATIONS: usize = 2;

/// Minimum index distance along the band for two points sharing a
/// pixel to count as a self-contact.
const SELF_TOUCH_MARGIN: usize = 4;

/// Minimum length of an aligned run to be smoothed.
const ALIGNMENT_MIN_RUN_LENGTH: usize = 4;

/// Number of moving-average passes applied to a rebuilt corridor.
const CORRIDOR_SMOOTHING_PASSES: usize = 2;

/// Moving-average radius used on a rebuilt corridor.
const CORRIDOR_SMOOTHING_RADIUS: usize = 2;

/// Maximum number of points considered on each fork arm when
/// re-spacing the junction.
const FORK_ARM_EXTENT: usize = 8;

pub fn get_enclosing_hull(
    image: &[u8],
    width: usize,
    height: usize,
) -> Option<Vec<Point>> {
    let contours = find_external_contours(image, width, height);
    if contours.is_empty() {
        return None;
    }

    let all_points: Vec<Point> = contours.iter().flatten().copied().collect();
    if all_points.len() < 3 {
        return None;
    }

    let hull = get_polygon_convex_hull(&all_points);
    if hull.len() < 3 {
        None
    } else {
        Some(hull)
    }
}

/// Extracts the convex hull of each foreground region in a binary image.
/// Pixels with value 0 are treated as background; non-zero values are foreground.
/// Returns a list of hulls (each hull is a `Vec<Point>` ordered counter-clockwise).
pub fn get_hulls_from_image(
    image: &[u8],
    width: usize,
    height: usize,
) -> Vec<Vec<Point>> {
    let contours = find_external_contours(image, width, height);
    let mut result = Vec::new();

    for contour in &contours {
        if contour.len() < 3 {
            continue;
        }
        let hull = get_polygon_convex_hull(contour);
        if hull.len() >= 3 {
            result.push(hull);
        }
    }

    result
}
pub fn get_concave_hull(
    image: &[u8],
    width: usize,
    height: usize,
    gravity: f64,
    allow_self_intersections: bool,
) -> Option<Vec<Point>> {
    let trace = std::env::var("RAYGEO_HULL_TRACE").is_ok();
    if trace {
        eprintln!(
            "[hull] image={}x{} gravity={} allow_self_intersections={}",
            width, height, gravity, allow_self_intersections
        );
    }
    if let Ok(path) = std::env::var("RAYGEO_HULL_DUMP") {
        dump_bool_image(&path, image, width, height);
    }

    let effective_gravity = gravity.clamp(0.0, 1.0).powi(2) * PULL_WEIGHT;
    if effective_gravity < 1e-6 {
        return get_enclosing_hull(image, width, height);
    }

    let contours = find_external_contours(image, width, height);
    if contours.is_empty() {
        return None;
    }
    if trace {
        eprintln!(
            "[hull] contours={} effective_gravity={:.3}",
            contours.len(),
            effective_gravity
        );
    }

    let all_points: Vec<Point> = contours.iter().flatten().copied().collect();
    if all_points.len() < 3 {
        return get_enclosing_hull(image, width, height);
    }

    let hull_vertices = get_polygon_convex_hull(&all_points);
    let num_hull = hull_vertices.len();
    if num_hull < 3 {
        return get_enclosing_hull(image, width, height);
    }

    // Walk along the hull, sampling at small distances.
    let mut band = resample_polygon(&hull_vertices, BAND_SPACING);
    if trace {
        eprintln!("[hull] initial band points={}", band.len());
    }

    // Coarse distance field over the content, used to bound the
    // content march so far-away points do not probe pixel by pixel.
    let dt = build_distance_field(image, width, height, DISTANCE_FIELD_SCALE);

    // Integrate the shrink over the gravity in small increments, so
    // the result changes continuously with the requested gravity.
    // Each pass consumes one increment of the budget; a pass with no
    // remaining pull (progress below the threshold) stops the shrink.
    let mut applied = 0.0f64;
    let mut passes = 0usize;
    for _ in 0..MAX_SHRINK_ITERATIONS {
        if applied >= effective_gravity - 1e-9 {
            break;
        }
        let step = GRAVITY_STEP.min(effective_gravity - applied);
        let pass_start = std::time::Instant::now();
        let (next, progress, stats) = shrink_band(
            &band,
            image,
            width,
            height,
            &dt,
            step,
            allow_self_intersections,
        );
        passes += 1;
        if trace {
            let elapsed = pass_start.elapsed().as_secs_f64() * 1000.0;
            eprintln!(
                "[hull] pass={} applied={:.3}/{:.3} step={:.3} progress={:.3} \
                 band={} touches={} free={} max_depth={:.2} clamped={} \
                 anchored={} si_limited={} time={:.1}ms",
                passes,
                applied + step,
                effective_gravity,
                step,
                progress,
                next.len(),
                stats.touches,
                stats.free,
                stats.max_depth,
                stats.clamped,
                stats.anchored,
                stats.si_limited,
                elapsed,
            );
        }
        band = next;
        applied += step;
        if progress < CONVERGED_PROGRESS {
            if trace {
                eprintln!(
                    "[hull] stop: band converged (progress {:.3} < {})",
                    progress, CONVERGED_PROGRESS
                );
            }
            break;
        }
    }
    if trace
        && applied < effective_gravity - 1e-9
        && passes >= MAX_SHRINK_ITERATIONS
    {
        eprintln!(
            "[hull] stop: MAX_SHRINK_ITERATIONS ({}) reached, applied={:.3}/{:.3}",
            MAX_SHRINK_ITERATIONS, applied, effective_gravity
        );
    }
    if trace {
        eprintln!(
            "[hull] done: passes={} applied={:.3}/{:.3} final band points={}",
            passes,
            applied,
            effective_gravity,
            band.len()
        );
    }
    if let Ok(path) = std::env::var("RAYGEO_HULL_DUMP") {
        dump_band(&format!("{}.band.txt", path), &band);
    }

    // When self-intersections are prevented, smooth the stretches
    // where the band is aligned with itself (the pinch corridors).
    if !allow_self_intersections {
        let before = band.clone();
        smooth_aligned_sections(&mut band, width, height);
        // The corridor rebuild can fold the band over at a fork when
        // the pinches are deep; keep the unsmoothed band in that case.
        if band_self_intersects(&band) {
            band = before;
        }
    }

    // The pinch snapping and the elasticity smoothing can leave
    // degenerate out-and-back retraces that plot as figure-eight
    // artifacts at the pinch points; drop them.
    remove_backtracks(&mut band);

    Some(band)
}

/// Removes degenerate out-and-back retraces and consecutive duplicate
/// points from the finished band. These are produced when pinch
/// snapping pulls a strand back onto itself; they are collinear
/// overlaps, invisible to the self-intersection checks, but they plot
/// as small figure-eight loops.
fn remove_backtracks(band: &mut Vec<Point>) {
    loop {
        let n = band.len();
        if n < 3 {
            return;
        }
        let mut keep = Vec::with_capacity(n);
        for i in 0..n {
            let p0 = band[(i + n - 1) % n];
            let p1 = band[i];
            let p2 = band[(i + 1) % n];
            let d02 = p0.distance(p2);
            let through = p0.distance(p1) + p1.distance(p2);
            let duplicate = p0.distance(p1) < 0.5;
            let retrace = d02 < 1.5 && through > d02 + 1.5;
            if !(duplicate || retrace) {
                keep.push(p1);
            }
        }
        if keep.len() == n {
            return;
        }
        *band = keep;
    }
}

/// Smooths the stretches where the band is aligned with itself (the
/// pinch corridors). Each corridor is rebuilt around a shared
/// centerline so both passes follow the same clean line.
fn smooth_aligned_sections(band: &mut [Point], width: usize, height: usize) {
    let num = band.len();
    if num < 8 {
        return;
    }

    // The points close to a non-adjacent part of the band.
    let mut cells: std::collections::HashMap<usize, Vec<usize>> =
        std::collections::HashMap::with_capacity(num);
    for (i, &point) in band.iter().enumerate() {
        let x = point.x.round() as i64;
        let y = point.y.round() as i64;
        if x >= 0 && y >= 0 && x < width as i64 && y < height as i64 {
            cells
                .entry(y as usize * width + x as usize)
                .or_default()
                .push(i);
        }
    }
    let mut aligned = vec![false; num];
    for (i, &point) in band.iter().enumerate() {
        let x = point.x.round() as i64;
        let y = point.y.round() as i64;
        let mut found = false;
        for dy in -2i64..=2 {
            for dx in -2i64..=2 {
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64
                {
                    continue;
                }
                if let Some(cell_indices) =
                    cells.get(&(ny as usize * width + nx as usize))
                {
                    for &j in cell_indices {
                        if circular_distance(i, j, num) > SELF_TOUCH_MARGIN {
                            found = true;
                            break;
                        }
                    }
                }
                if found {
                    break;
                }
            }
            if found {
                break;
            }
        }
        aligned[i] = found;
    }

    // The contiguous runs of aligned points.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < num {
        if aligned[i] {
            let mut end = i;
            while end + 1 < num && aligned[end + 1] {
                end += 1;
            }
            if end - i + 1 >= ALIGNMENT_MIN_RUN_LENGTH {
                runs.push((i, end));
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    if runs.len() < 2 {
        if std::env::var("RAYGEO_HULL_TRACE").is_ok() {
            eprintln!(
                "[hull] smooth_aligned_sections: {} aligned points, {} runs",
                aligned.iter().filter(|&&a| a).count(),
                runs.len(),
            );
        }
        return;
    }

    // Pair each run with the closest other run: together they form the
    // two strands of one corridor.
    let mut paired = vec![false; runs.len()];
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for a in 0..runs.len() {
        if paired[a] {
            continue;
        }
        let mut best: Option<(usize, f64)> = None;
        for b in 0..runs.len() {
            if b == a || paired[b] {
                continue;
            }
            let distance = run_distance(band, runs[a], runs[b]);
            if best.is_none_or(|(_, best)| distance < best) {
                best = Some((b, distance));
            }
        }
        if let Some((b, _)) = best {
            // Only accept the pairing when it is mutual, so the runs
            // of different corridors are not matched.
            let mut best_back: Option<(usize, f64)> = None;
            for c in 0..runs.len() {
                if c == b {
                    continue;
                }
                let distance = run_distance(band, runs[b], runs[c]);
                if best_back.is_none_or(|(_, best)| distance < best) {
                    best_back = Some((c, distance));
                }
            }
            if best_back.map(|(c, _)| c) == Some(a) {
                paired[a] = true;
                paired[b] = true;
                pairs.push((a, b));
            }
        }
    }

    // Snap each corridor's strands onto a shared smoothed centerline,
    // so both passes follow the same clean line symmetrically.
    let mut snapped = band.to_vec();
    for &(a, b) in &pairs {
        rebuild_corridor(&mut snapped, band, runs[a], runs[b]);
    }
    // Re-space the arms around each fork to the corridor's spacing,
    // keeping the fork point itself fixed and shared, so the corridor
    // blends into the departing curves without a jump.
    for &(a, b) in &pairs {
        let spacing = corridor_spacing(band, runs[a]);
        smooth_corridor_forks(&mut snapped, runs[a], runs[b], spacing);
    }
    if std::env::var("RAYGEO_HULL_TRACE").is_ok() {
        eprintln!(
            "[hull] smooth_aligned_sections: {} runs, {} corridor pairs",
            runs.len(),
            pairs.len(),
        );
    }
    band.copy_from_slice(&snapped);
}

/// The average spacing between the points in the middle of a run.
fn corridor_spacing(band: &[Point], run: (usize, usize)) -> f64 {
    let middle = (run.0 + run.1) / 2;
    let mut total = 0.0;
    let mut count = 0.0f64;
    for i in middle.saturating_sub(4)..middle.saturating_add(4).min(run.1) {
        total += (band[i + 1] - band[i]).length();
        count += 1.0;
    }
    total / count.max(1.0)
}

/// Re-spaces the arms around the two forks of a corridor pair.
fn smooth_corridor_forks(
    band: &mut [Point],
    a: (usize, usize),
    b: (usize, usize),
    spacing: f64,
) {
    let num = band.len();
    // The fork where the run a starts and the run b ends: the corridor
    // arm continues forward through the a-run and backward through the
    // b-run.
    resample_one_fork(
        band,
        num,
        a.0,
        b.1,
        (a.0 + 1) % num,
        1,
        (b.1 + num - 1) % num,
        num - 1,
        (a.0 + num - 1) % num,
        num - 1,
        (b.1 + 1) % num,
        1,
        spacing,
    );
    // The fork where the run a ends and the run b starts: the corridor
    // arm continues backward through the a-run and forward through the
    // b-run.
    resample_one_fork(
        band,
        num,
        a.1,
        b.0,
        (a.1 + num - 1) % num,
        num - 1,
        (b.0 + 1) % num,
        1,
        (a.1 + 1) % num,
        1,
        (b.0 + num - 1) % num,
        num - 1,
        spacing,
    );
}

/// Re-spaces the points directly around one fork: the shared fork
/// point is fixed, and the first points of each arm (the corridor and
/// the two departing curves) are placed at even spacing along the
/// arm's own curve.
#[allow(clippy::too_many_arguments)]
fn resample_one_fork(
    band: &mut [Point],
    num: usize,
    fork_a: usize,
    fork_b: usize,
    corridor_a: usize,
    corridor_a_step: usize,
    corridor_b: usize,
    corridor_b_step: usize,
    depart_a: usize,
    depart_a_step: usize,
    depart_b: usize,
    depart_b_step: usize,
    spacing: f64,
) {
    let fork = (band[fork_a] + band[fork_b]) * 0.5;
    band[fork_a] = fork;
    band[fork_b] = fork;

    // The corridor arm, re-spaced along its own curve.
    respace_arm(band, num, fork, corridor_a, corridor_a_step, spacing);
    respace_arm(band, num, fork, corridor_b, corridor_b_step, spacing);

    // The two departing arms, each with its own spacing measured from
    // the untouched points further along the arm.
    for (depart, step) in [(depart_a, depart_a_step), (depart_b, depart_b_step)]
    {
        let mut arm_spacing = spacing;
        let mut total = 0.0;
        let mut count = 0.0f64;
        for k in 3..6 {
            let from = band[(depart + step * k) % num];
            let to = band[(depart + step * (k + 1)) % num];
            total += (to - from).length();
            count += 1.0;
        }
        if count > 0.0 {
            arm_spacing = total / count;
        }
        respace_arm(band, num, fork, depart, step, arm_spacing);
    }
}

/// Places the first points of a fork arm at even spacing along the
/// arm's own polyline, starting from the fork point, so the junction
/// follows the arm's curve instead of a straight ray.
fn respace_arm(
    band: &mut [Point],
    num: usize,
    fork: Point,
    depart: usize,
    step: usize,
    spacing: f64,
) {
    let mut path = Vec::with_capacity(FORK_ARM_EXTENT + 1);
    path.push(fork);
    for k in 0..FORK_ARM_EXTENT {
        path.push(band[(depart + step * k) % num]);
    }
    let mut cumulative = vec![0.0; path.len()];
    for k in 1..path.len() {
        cumulative[k] = cumulative[k - 1] + (path[k] - path[k - 1]).length();
    }
    let mut cursor = 0;
    for k in 1..=FORK_ARM_EXTENT {
        let target = spacing * k as f64;
        if target >= cumulative[path.len() - 1] {
            break;
        }
        while cursor + 1 < cumulative.len() && cumulative[cursor + 1] < target {
            cursor += 1;
        }
        let segment = cumulative[cursor + 1] - cumulative[cursor];
        let t = if segment > 1e-9 {
            (target - cumulative[cursor]) / segment
        } else {
            0.0
        };
        let placed = path[cursor] + (path[cursor + 1] - path[cursor]) * t;
        band[(depart + step * (k - 1)) % num] = placed;
    }
}
/// Rebuilds one pinch corridor: both strands are replaced by a shared
/// centerline through the corridor, smoothed and uniformly resampled,
/// so the two passes of the band follow the exact same clean line.
fn rebuild_corridor(
    snapped: &mut [Point],
    band: &[Point],
    a: (usize, usize),
    b: (usize, usize),
) {
    let strand_a: Vec<Point> = (a.0..=a.1).map(|i| band[i]).collect();
    let mut strand_b: Vec<Point> = (b.0..=b.1).map(|i| band[i]).collect();
    let b_reversed = (strand_b.first().unwrap() - strand_a.first().unwrap())
        .length()
        > (strand_b.last().unwrap() - strand_a.first().unwrap()).length();
    if b_reversed {
        strand_b.reverse();
    }

    let len_a = strand_a.len();
    let len_b = strand_b.len();
    let center_len = len_a.max(len_b);

    // The shared centerline: the midpoint of the points paired by
    // their normalized position along each strand.
    let mut centerline = Vec::with_capacity(center_len);
    for k in 0..center_len {
        let fraction = k as f64 / (center_len - 1).max(1) as f64;
        centerline.push(
            (get_point_at_fraction(&strand_a, fraction)
                + get_point_at_fraction(&strand_b, fraction))
                * 0.5,
        );
    }

    // Smooth the centerline so the wobble of either strand is ironed
    // out.
    for _ in 0..CORRIDOR_SMOOTHING_PASSES {
        centerline =
            get_points_moving_average(&centerline, CORRIDOR_SMOOTHING_RADIUS);
    }

    // Both strands now follow the centerline exactly.
    for k in 0..len_a {
        let fraction = k as f64 / (len_a - 1).max(1) as f64;
        snapped[a.0 + k] = get_point_at_fraction(&centerline, fraction);
    }
    for k in 0..len_b {
        let fraction = if b_reversed {
            (len_b - 1 - k) as f64 / (len_b - 1).max(1) as f64
        } else {
            k as f64 / (len_b - 1).max(1) as f64
        };
        snapped[b.0 + k] = get_point_at_fraction(&centerline, fraction);
    }
}

/// Mean distance between the closest points of two runs.
fn run_distance(band: &[Point], a: (usize, usize), b: (usize, usize)) -> f64 {
    let mut total = 0.0;
    let mut count = 0.0;
    for i in a.0..=a.1 {
        let mut closest = f64::INFINITY;
        for j in b.0..=b.1 {
            closest = closest.min((band[i] - band[j]).length());
        }
        total += closest;
        count += 1.0;
    }
    for j in b.0..=b.1 {
        let mut closest = f64::INFINITY;
        for i in a.0..=a.1 {
            closest = closest.min((band[j] - band[i]).length());
        }
        total += closest;
        count += 1.0;
    }
    total / count
}

/// Per-pass diagnostics, used by the `RAYGEO_HULL_TRACE` instrumentation.
struct ShrinkStats {
    touches: usize,
    free: usize,
    max_depth: f64,
    /// Points whose pull was clamped at the first content pixel.
    clamped: usize,
    /// Extra fold anchors added this pass.
    anchored: usize,
    /// True when the fallback global scale bisection engaged.
    si_limited: bool,
}

/// Writes the boolean image as a PGM (P5) file, for offline analysis.
fn dump_bool_image(path: &str, image: &[u8], width: usize, height: usize) {
    if let Ok(mut file) = std::fs::File::create(path) {
        use std::io::Write;
        let _ = write!(file, "P5\n{} {}\n255\n", width, height);
        let mut bytes = Vec::with_capacity(width * height);
        for &v in image {
            bytes.push(if v != 0 { 255 } else { 0 });
        }
        let _ = file.write_all(&bytes);
        eprintln!("[hull] dumped image to {}", path);
    }
}

/// Writes the band as `x y` lines, for offline analysis.
fn dump_band(path: &str, band: &[Point]) {
    if let Ok(mut file) = std::fs::File::create(path) {
        use std::io::Write;
        for p in band {
            let _ = writeln!(file, "{} {}", p.x, p.y);
        }
        eprintln!("[hull] dumped band to {}", path);
    }
}

/// Coarse distance to the nearest content pixel on a grid downsampled
/// by `scale`. Stores a 3-4 chamfer distance in downsampled cells,
/// which overestimates the Euclidean distance by at most ~3x.
struct DistanceField {
    data: Vec<u16>,
    width: usize,
    height: usize,
    scale: usize,
}

impl DistanceField {
    /// Downsampled chamfer distance to the nearest content pixel, or
    /// 0 when the query point lies outside the image.
    fn get(&self, x: f64, y: f64) -> u16 {
        let gx = (x / self.scale as f64).round() as i64;
        let gy = (y / self.scale as f64).round() as i64;
        if gx < 0
            || gy < 0
            || gx >= self.width as i64
            || gy >= self.height as i64
        {
            return 0;
        }
        self.data[gy as usize * self.width + gx as usize]
    }
}

/// Distance to content in pixels at which the march switches from
/// distance-field jumps to 1px pixel probes.
const MARCH_NEAR_PX: f64 = 24.0;
/// Safety margin subtracted from the jump distance to account for the
/// downsampling and chamfer approximation errors.
const MARCH_MARGIN_PX: f64 = 12.0;

/// Builds the coarse distance field over the boolean image.
fn build_distance_field(
    image: &[u8],
    width: usize,
    height: usize,
    scale: usize,
) -> DistanceField {
    let w = width.div_ceil(scale);
    let h = height.div_ceil(scale);
    const INF: u16 = 30000;
    let mut data = vec![INF; w * h];
    for y in 0..height {
        for x in 0..width {
            if image[y * width + x] != 0 {
                data[(y / scale) * w + (x / scale)] = 0;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut v = data[i];
            if x > 0 {
                v = v.min(data[i - 1] + 3);
            }
            if y > 0 {
                v = v.min(data[i - w] + 3);
            }
            if x > 0 && y > 0 {
                v = v.min(data[i - w - 1] + 4);
            }
            if x + 1 < w && y > 0 {
                v = v.min(data[i - w + 1] + 4);
            }
            data[i] = v;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut v = data[i];
            if x + 1 < w {
                v = v.min(data[i + 1] + 3);
            }
            if y + 1 < h {
                v = v.min(data[i + w] + 3);
            }
            if x + 1 < w && y + 1 < h {
                v = v.min(data[i + w + 1] + 4);
            }
            if x > 0 && y + 1 < h {
                v = v.min(data[i + w - 1] + 4);
            }
            data[i] = v;
        }
    }
    DistanceField {
        data,
        width: w,
        height: h,
        scale,
    }
}

/// One shrink pass: pulls every band point inward along the inward
/// normal, following a sine profile over its free section. Returns the
/// pulled band and the largest remaining pull (the pass progress), so
/// a fully anchored band can stop the shrink.
fn shrink_band(
    band: &[Point],
    image: &[u8],
    width: usize,
    height: usize,
    dt: &DistanceField,
    gravity: f64,
    allow_self_intersections: bool,
) -> (Vec<Point>, f64, ShrinkStats) {
    let num = band.len();
    if num < 3 {
        return (
            band.to_vec(),
            1.0,
            ShrinkStats {
                touches: num,
                free: 0,
                max_depth: 0.0,
                clamped: 0,
                anchored: 0,
                si_limited: false,
            },
        );
    }

    // The inward normal of each band edge, pointing to the interior
    // side given the band's winding.
    let inward_left = get_polygon_signed_area(band) >= 0.0;
    let mut normals = Vec::with_capacity(num);
    for i in 0..num {
        let p0 = band[i];
        let p2 = band[(i + 1) % num];
        let edge = p2 - p0;
        let length = edge.length();
        if length < 1e-9 {
            normals.push(*normals.last().unwrap_or(&Point::new(1.0, 0.0)));
            continue;
        }
        normals.push(if inward_left {
            Point::new(-edge.y, edge.x) / length
        } else {
            Point::new(edge.y, -edge.x) / length
        });
    }

    // Smooth the pull directions: the band is a chain of 1px steps, so
    // its edge normals alternate along diagonals. Pulling each point
    // along its raw edge normal amplifies that staircase into a
    // zigzag, so the normals are averaged over a small window (and
    // re-normalised) first.
    for _ in 0..2 {
        let smoothed: Vec<Point> = (0..num)
            .map(|i| {
                let mut sum = Point::ZERO;
                for k in 0..=NORMAL_SMOOTH_RADIUS {
                    sum += normals[(i + k) % num];
                    sum += normals[(i + num - k) % num];
                }
                let n = sum / ((2 * NORMAL_SMOOTH_RADIUS + 1) as f64);
                let length = n.length();
                if length > 1e-9 {
                    n / length
                } else {
                    normals[i]
                }
            })
            .collect();
        normals = smoothed;
    }

    // A point touches the shape when the content is at or right next
    // to its position. A point also touches the band itself when
    // another, non-adjacent part of the band shares its pixel: such a
    // pinch point then acts like a shape touch, splitting the band
    // into new sections that continue to be pulled.
    let mut touch = vec![false; num];
    let mut self_cell = CellTable::new(num);
    for i in 0..num {
        let sample = band[i];
        let x = sample.x.round() as i64;
        let y = sample.y.round() as i64;
        let mut touching = false;
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0
                    && ny >= 0
                    && nx < width as i64
                    && ny < height as i64
                    && image[ny as usize * width + nx as usize] != 0
                {
                    touching = true;
                }
            }
        }
        if !allow_self_intersections
            && x >= 0
            && y >= 0
            && x < width as i64
            && y < height as i64
        {
            let cell = y as usize * width + x as usize;
            if let Some(previous) = self_cell.get(cell) {
                let distance = circular_distance(i, previous, num);
                if distance > SELF_TOUCH_MARGIN {
                    touching = true;
                    touch[previous] = true;
                }
            }
            self_cell.insert(cell, i);
        }
        touch[i] = touching;
    }

    // Fold anchors start as the shape/self touches and grow wherever
    // the pulled band would cross itself.
    let mut anchors = touch.clone();

    // The distance each point may travel along its normal before
    // reaching content, minus a 1px clearance. It depends only on the
    // band geometry, not on the pull profile, so it is computed once
    // per pass. Far from content the march jumps along the normal
    // using the distance field; near content it falls back to exact
    // 1px pixel probes.
    let clearance: Vec<f64> = (0..num)
        .map(|i| {
            let origin = band[i];
            let normal = normals[i];
            let mut t = 0.0f64;
            loop {
                let probe = origin + normal * t;
                let px = probe.x.round() as i64;
                let py = probe.y.round() as i64;
                if px < 0 || py < 0 || px >= width as i64 || py >= height as i64
                {
                    return f64::INFINITY;
                }
                let d = dt.get(probe.x, probe.y) as f64 * dt.scale as f64;
                if d > MARCH_NEAR_PX {
                    let jump = (d / 3.0 - MARCH_MARGIN_PX).max(1.0);
                    t += jump;
                } else if image[py as usize * width + px as usize] != 0 {
                    return (t - 1.0).max(0.0);
                } else {
                    t += 1.0;
                }
            }
        })
        .collect();

    // The pull profile over the anchors, capped per point by its own
    // content clearance: a point that has reached content stops
    // without throttling the rest of its section, so the band keeps
    // advancing wherever it still can.
    let clamped = std::cell::Cell::new(0usize);
    let pull_depths = |anchors: &[bool]| -> Vec<f64> {
        let (forward, backward) = nearest_touch_distances(anchors);
        let mut count = 0usize;
        let depths: Vec<f64> = forward
            .iter()
            .zip(&backward)
            .zip(&clearance)
            .map(|((&forward, &backward), &limit)| {
                let length = forward + backward;
                if length > 0.0 {
                    let profile = (forward.min(backward) / length
                        * std::f64::consts::PI)
                        .sin()
                        * length
                        * gravity;
                    if profile > limit {
                        count += 1;
                        limit
                    } else {
                        profile
                    }
                } else {
                    0.0
                }
            })
            .collect();
        clamped.set(count);
        depths
    };

    // Pull the band and damp lateral waves with a small amount of
    // elasticity: two rounds of neighbour averaging. Without it, a
    // band hugging a content edge keeps any unevenness it picked up
    // while approaching, because the pull direction of a wavy strand
    // has a tangential component that transports the wave instead of
    // damping it. Points that the averaging would move into content
    // are reverted to their plain pulled position.
    let smooth_pull = |anchors: &[bool]| -> Vec<Point> {
        let depths = pull_depths(anchors);
        let mut band_out = pulled_band(band, &normals, &depths, 1.0);
        if !allow_self_intersections {
            for _ in 0..SMOOTH_ITERATIONS {
                let averaged: Vec<Point> = (0..num)
                    .map(|i| {
                        (band_out[(i + num - 1) % num]
                            + band_out[i] * 2.0
                            + band_out[(i + 1) % num])
                            * 0.25
                    })
                    .collect();
                band_out = averaged;
            }
            for i in 0..num {
                let x = band_out[i].x.round() as i64;
                let y = band_out[i].y.round() as i64;
                if x >= 0
                    && y >= 0
                    && x < width as i64
                    && y < height as i64
                    && image[y as usize * width + x as usize] != 0
                {
                    band_out[i] = band[i] + normals[i] * depths[i];
                }
            }
        }
        band_out
    };

    let mut depths = pull_depths(&anchors);

    // When self-intersections are not allowed, the pulled band must
    // stay a simple loop. Instead of scaling the whole band back
    // when folds would form, the points that would cross are turned
    // into anchors: a closed pinch then acts like a touch, and the
    // sections around it keep pulling. Only the zone around the
    // points that actually pulled can fold, so the scan is confined
    // to it.
    let mut anchored = 0usize;
    let mut si_limited = false;
    let candidate = if allow_self_intersections {
        pulled_band(band, &normals, &depths, 1.0)
    } else {
        let mut candidate = smooth_pull(&anchors);
        // Transversal self-touches: the two strands at the shared
        // point are snapped together (the later one onto the earlier
        // one) so the corridor closes into a pinch instead of the
        // strands exchanging sides.
        let mut point_fixes: Vec<(usize, usize)> = Vec::new();
        let mut rounds = 0usize;
        let mut clean = false;
        while rounds < FOLD_ANCHOR_ROUNDS {
            // The zone the pull could have folded: the bounding box of
            // the points that moved, padded by the largest movement.
            let mut zmin = Point::new(f64::INFINITY, f64::INFINITY);
            let mut zmax = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
            let mut max_d = 0.0f64;
            for i in 0..num {
                let d = depths[i];
                if d > 0.5 {
                    zmin = zmin.min(band[i]);
                    zmax = zmax.max(band[i]);
                    zmin = zmin.min(band[(i + 1) % num]);
                    zmax = zmax.max(band[(i + 1) % num]);
                    zmin = zmin.min(band[(i + num - 1) % num]);
                    zmax = zmax.max(band[(i + num - 1) % num]);
                    max_d = max_d.max(d);
                }
            }
            let crossings = if zmin.x <= zmax.x {
                let pad = Point::new(max_d + 8.0, max_d + 8.0);
                zone_band_crossings(
                    &candidate,
                    zmin - pad,
                    zmax + pad,
                    FOLD_CROSSINGS_PER_ROUND,
                )
            } else {
                Vec::new()
            };
            if crossings.is_empty() {
                clean = true;
                break;
            }
            let mut fresh = 0usize;
            for (i, j, _pt) in &crossings {
                // A crossing of two points at (nearly) the same
                // position is a transversal self-touch: snap the
                // strands together instead of anchoring them.
                if candidate[*i].distance(candidate[*j]) < 3.0 {
                    let fix = if *i < *j { (*i, *j) } else { (*j, *i) };
                    if !point_fixes.contains(&fix) {
                        point_fixes.push(fix);
                        fresh += 1;
                    }
                    continue;
                }
                for k in [*i, (*i + 1) % num, *j, (*j + 1) % num] {
                    if !anchors[k] {
                        anchors[k] = true;
                        fresh += 1;
                    }
                }
            }
            anchored += fresh;
            rounds += 1;
            if std::env::var("RAYGEO_HULL_TRACE").is_ok() {
                eprintln!(
                    "[hull] fold round {}: {} crossings, {} fixes",
                    rounds,
                    crossings.len(),
                    fresh,
                );
            }
            if fresh == 0 {
                break;
            }
            depths = pull_depths(&anchors);
            candidate = smooth_pull(&anchors);
            for (a, b) in &point_fixes {
                let p = candidate[*a];
                candidate[*b] = p;
            }
        }
        if clean {
            candidate
        } else {
            // The folds persist even with their points anchored: fall
            // back to a global scale bisection so the band stays a
            // simple loop.
            si_limited = true;
            let mut low = 0.0f64;
            let mut high = 1.0f64;
            for _ in 0..SELF_INTERSECTION_BISECTIONS {
                let middle = 0.5 * (low + high);
                let trial = pulled_band(band, &normals, &depths, middle);
                if band_self_intersects(&trial) {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            pulled_band(band, &normals, &depths, low)
        }
    };

    let max_depth = depths.iter().cloned().fold(0.0, f64::max);
    let free = depths.iter().filter(|&&d| d > 1e-9).count();
    let touches = num - free;
    let progress = max_depth;

    let mut pulled = candidate;
    if !allow_self_intersections {
        align_pinched_pairs(&mut pulled, width, height);
    }
    (
        pulled,
        progress,
        ShrinkStats {
            touches,
            free,
            max_depth,
            clamped: clamped.get(),
            anchored,
            si_limited,
        },
    )
}

/// Packs integer pixel coordinates into a 32-bit cell key for the
/// `CellTable` (whose slots hold the key in the high 32 bits).
fn cell_key(x: i64, y: i64) -> usize {
    let xw = (x + 32768) as u64 & 0xFFFF;
    let yw = (y + 32768) as u64 & 0xFFFF;
    ((xw << 16) | yw) as usize
}

/// Small open-addressing table mapping a cell index to a band index,
/// used for the per-pixel neighbour lookups during pinch alignment and
/// touch detection. Much faster than the hashed collections for this
/// pattern of many lookups on a stable key space.
struct CellTable {
    cells: Vec<u64>,
    cap_mask: usize,
}

impl CellTable {
    fn new(n: usize) -> Self {
        let cap = (n.max(8) * 2).next_power_of_two();
        Self {
            cells: vec![u64::MAX; cap],
            cap_mask: cap - 1,
        }
    }

    fn get(&self, key: usize) -> Option<usize> {
        // The high bits of the product give a good spread even when
        // the keys form an arithmetic progression (band points 1px
        // apart map to consecutive cell indices).
        let mut h =
            ((key as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) as usize;
        loop {
            let slot = self.cells[h & self.cap_mask];
            if slot == u64::MAX {
                return None;
            }
            if (slot >> 32) as usize == key {
                return Some((slot & 0xFFFF_FFFF) as usize);
            }
            h += 1;
        }
    }

    fn insert(&mut self, key: usize, index: usize) {
        let mut h =
            ((key as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) as usize;
        loop {
            let slot = &mut self.cells[h & self.cap_mask];
            if *slot == u64::MAX {
                *slot = ((key as u64) << 32) | index as u64;
                return;
            }
            if (*slot >> 32) as usize == key {
                // Keep the first index for the cell, like
                // `HashMap::entry().or_insert()`.
                return;
            }
            h += 1;
        }
    }
}

/// Pulls the non-adjacent band points that lie within a couple of
/// pixels of each other onto the earlier one, so a pinch resolves into
/// a clean shared line. The fork points where the strands part are
/// merged into shared vertices, and the moved points are then relaxed
/// so the strands leave the pinch smoothly instead of zigzagging.
fn align_pinched_pairs(band: &mut [Point], width: usize, height: usize) {
    let num = band.len();
    let mut moved: Vec<f64> = vec![0.0; num];
    let mut cells = CellTable::new(num);
    for i in 0..num {
        let x = band[i].x.round() as i64;
        let y = band[i].y.round() as i64;
        let mut nearest: Option<usize> = None;
        let mut best = f64::INFINITY;
        // Deduplicate the neighbouring cells so each is only visited
        // once (up to 9 distinct cells, but often fewer).
        let mut seen = [usize::MAX; 9];
        let mut seen_count = 0usize;
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64
                {
                    continue;
                }
                let cell = ny as usize * width + nx as usize;
                if seen[..seen_count].contains(&cell) {
                    continue;
                }
                seen[seen_count] = cell;
                seen_count += 1;
                if let Some(j) = cells.get(cell) {
                    if circular_distance(i, j, num) > SELF_TOUCH_MARGIN {
                        let distance = band[i].distance_squared(band[j]);
                        if distance < best {
                            best = distance;
                            nearest = Some(j);
                        }
                    }
                }
            }
        }
        if let Some(j) = nearest {
            moved[i] = best.sqrt();
            band[i] = band[j];
        }
        let mx = band[i].x.round() as i64;
        let my = band[i].y.round() as i64;
        if mx >= 0 && my >= 0 && mx < width as i64 && my < height as i64 {
            let cell = my as usize * width + mx as usize;
            cells.insert(cell, i);
        }
    }

    // Keep the input band as the fallback: the caller guarantees it
    // is a simple loop (the fold anchoring), so reverting to it keeps
    // the band simple even when the snapping itself crossed strands.
    let input = band.to_vec();

    // Relax the points that were snapped and their neighbours, so the
    // kinks the snapping introduced at the pinch forks smooth out.
    // The straight shared stretch is left untouched.
    for _ in 0..4 {
        let relaxed: Vec<Point> = (0..num)
            .map(|i| {
                let affected = moved[i] > 0.1
                    || moved[(i + num - 1) % num] > 0.1
                    || moved[(i + 1) % num] > 0.1;
                if affected {
                    (band[(i + num - 1) % num]
                        + band[i] * 2.0
                        + band[(i + 1) % num])
                        * 0.25
                } else {
                    band[i]
                }
            })
            .collect();
        band.copy_from_slice(&relaxed);
    }

    // The snapping and the relaxation can fold the band over (strands
    // snap across each other at an angle, crossing at the shared
    // point). When the band is no longer a simple loop, revert to the
    // input: a pinch left unaligned for one pass is preferable to a
    // self-intersecting outline. Nothing changed when no point was
    // snapped, so the check can be skipped.
    if moved.iter().any(|&m| m > 0.1) && first_band_crossing(band).is_some() {
        band.copy_from_slice(&input);
    }
}

/// Detects crossings where two non-adjacent parts of the band pass
/// through the same point with alternating directions. Such a
/// transversal self-touch is invisible to the segment-intersection
/// tests (every involved segment pair shares an endpoint), but it is
/// still a genuine self-crossing: the two strands exchange sides at
/// the shared point instead of merely touching.
fn find_point_crossings(
    points: &[Point],
    zone_min: Point,
    zone_max: Point,
    limit: usize,
) -> Vec<(usize, usize, Point)> {
    let n = points.len();
    let mut result = Vec::new();
    if n < 4 {
        return result;
    }
    let mut cells = CellTable::new(n);
    for i in 0..n {
        let p = points[i];
        if p.x < zone_min.x
            || p.x > zone_max.x
            || p.y < zone_min.y
            || p.y > zone_max.y
        {
            continue;
        }
        let x = p.x.round() as i64;
        let y = p.y.round() as i64;
        let mut hit: Option<usize> = None;
        'neighbours: for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let key = cell_key(x + dx, y + dy);
                if let Some(j) = cells.get(key) {
                    if circular_distance(i, j, n) > SELF_TOUCH_MARGIN {
                        hit = Some(j);
                        break 'neighbours;
                    }
                }
            }
        }
        let Some(j) = hit else {
            let key = cell_key(x, y);
            cells.insert(key, i);
            continue;
        };
        // Two visits at (nearly) the same point: they cross when the
        // four outgoing directions alternate around the point (visit
        // i, visit j, visit i, visit j). A clean pinch fork has each
        // visit's directions anti-parallel to the other's, so the
        // directions come grouped (i, i, j, j) and are not a crossing.
        let q = points[j];
        let in_i = p - points[(i + n - 1) % n];
        let out_i = points[(i + 1) % n] - p;
        let in_j = q - points[(j + n - 1) % n];
        let out_j = points[(j + 1) % n] - q;
        if in_i.length() < 0.5
            || out_i.length() < 0.5
            || in_j.length() < 0.5
            || out_j.length() < 0.5
        {
            let key = cell_key(x, y);
            cells.insert(key, i);
            continue;
        }
        let mut rays = [
            (0u8, in_i.y.atan2(in_i.x)),
            (0u8, out_i.y.atan2(out_i.x)),
            (1u8, in_j.y.atan2(in_j.x)),
            (1u8, out_j.y.atan2(out_j.x)),
        ];
        rays.sort_by(|a, b| a.1.total_cmp(&b.1));
        let crossing = rays[0].0 == rays[2].0
            && rays[1].0 == rays[3].0
            && rays[0].0 != rays[1].0;
        if crossing {
            result.push((i, j, p));
            if result.len() >= limit {
                return result;
            }
        }
        let key = cell_key(x, y);
        cells.insert(key, i);
    }
    result
}

/// Collects up to `limit` self-crossings of the closed polyline whose
/// intersection points lie inside the given zone. The caller must
/// guarantee the polyline is simple outside the zone.
fn zone_band_crossings(
    points: &[Point],
    zone_min: Point,
    zone_max: Point,
    limit: usize,
) -> Vec<(usize, usize, Point)> {
    let n = points.len();
    let mut result = Vec::new();
    if n < 2 {
        return result;
    }
    // The zone, padded so segments crossing its edge are included.
    let pad = 8.0;
    let min_x = zone_min.x - pad;
    let max_x = zone_max.x + pad;
    let min_y = zone_min.y - pad;
    let max_y = zone_max.y + pad;
    const CELL_SIZE: f64 = 8.0;
    let inv_cell = 1.0 / CELL_SIZE;
    let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> =
        std::collections::HashMap::new();
    for i in 0..n {
        let p1 = points[i];
        let p2 = points[(i + 1) % n];
        let seg_min_x = p1.x.min(p2.x);
        let seg_max_x = p1.x.max(p2.x);
        let seg_min_y = p1.y.min(p2.y);
        let seg_max_y = p1.y.max(p2.y);
        if seg_max_x < min_x
            || seg_min_x > max_x
            || seg_max_y < min_y
            || seg_min_y > max_y
        {
            continue;
        }
        let x0 = (seg_min_x * inv_cell).floor() as i64;
        let x1 = (seg_max_x * inv_cell).floor() as i64;
        let y0 = (seg_min_y * inv_cell).floor() as i64;
        let y1 = (seg_max_y * inv_cell).floor() as i64;
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                grid.entry((cx, cy)).or_default().push(i);
            }
        }
    }
    for cell_indices in grid.values() {
        for a in 0..cell_indices.len() {
            for b in a + 1..cell_indices.len() {
                let (i, j) = (cell_indices[a], cell_indices[b]);
                let p1 = points[i];
                let p2 = points[(i + 1) % n];
                let q1 = points[j];
                let q2 = points[(j + 1) % n];
                // Adjacent segments share a vertex; the wrapping pair
                // is still checked below for folds at the closure.
                if i.abs_diff(j) <= 1 {
                    continue;
                }
                let Some(pt) = get_line_segment_intersection(p1, p2, q1, q2)
                else {
                    continue;
                };
                let at_end1 = pt.distance_squared(p1) < EPSILON_INTERSECT
                    || pt.distance_squared(p2) < EPSILON_INTERSECT;
                let at_end2 = pt.distance_squared(q1) < EPSILON_INTERSECT
                    || pt.distance_squared(q2) < EPSILON_INTERSECT;
                if at_end1 || at_end2 {
                    continue;
                }
                result.push((i, j, pt));
                if result.len() >= limit {
                    return result;
                }
            }
        }
    }
    // Transversal self-touches at shared points: the segments all
    // share an endpoint there, so the scan above cannot see them.
    for (i, j, pt) in find_point_crossings(
        points,
        Point::new(min_x, min_y),
        Point::new(max_x, max_y),
        limit.saturating_sub(result.len()),
    ) {
        result.push((i, j, pt));
        if result.len() >= limit {
            return result;
        }
    }
    result
}

/// The band pulled inward by the given depths, uniformly scaled.
fn pulled_band(
    band: &[Point],
    normals: &[Point],
    depths: &[f64],
    scale: f64,
) -> Vec<Point> {
    band.iter()
        .zip(normals)
        .zip(depths)
        .map(|((&sample, &normal), &depth)| sample + normal * (depth * scale))
        .collect()
}

/// Circular distance between two indices along a closed loop.
fn circular_distance(a: usize, b: usize, length: usize) -> usize {
    let straight = a.abs_diff(b);
    straight.min(length - straight)
}

/// Whether a closed point loop crosses itself.
///
/// Mirrors the semantics of `check_self_intersection_from_array`
/// applied to the polyline built from `points` (with
/// `fail_on_t_junction = false`): the same per-pair tests are
/// performed, but the intermediate geometry and command arrays are
/// not constructed. Since every command of such a polyline is a line,
/// each one linearizes to exactly one segment, and the resulting
/// boolean is identical.
///
/// Candidate pairs are collected with a uniform pixel grid instead of
/// an R-tree. Because the cell size is a power of two, cell-index
/// division is exact, so any two overlapping bounding boxes are
/// guaranteed to share at least one cell: the grid therefore yields a
/// superset of the bbox-intersecting pairs an R-tree query would
/// visit. Extra pairs cannot produce a different result because the
/// per-pair test only reports an intersection when the segments'
/// bounding boxes overlap.
fn band_self_intersects(points: &[Point]) -> bool {
    first_band_crossing(points).is_some()
}

/// Finds the first genuine self-crossing of the closed polyline: two
/// non-adjacent segments that intersect away from their endpoints, or
/// an adjacent pair whose intersection lies away from their shared
/// vertex.
fn first_band_crossing(points: &[Point]) -> Option<(usize, usize, Point)> {
    collect_band_crossings(points, 1).pop()
}

/// Collects up to `limit` genuine self-crossings of the closed
/// polyline: two non-adjacent segments that intersect away from their
/// endpoints, or an adjacent pair whose intersection lies away from
/// their shared vertex. Returns the segment indices and the
/// intersection point of each crossing.
fn collect_band_crossings(
    points: &[Point],
    limit: usize,
) -> Vec<(usize, usize, Point)> {
    let n = points.len();
    let mut result = Vec::new();
    if n < 2 {
        return result;
    }

    // One segment per polyline edge, indexed like the commands of the
    // equivalent geometry (move at index 0, line commands from 1 on):
    // command index c corresponds to the segment from points[c - 1]
    // to points[c % n].
    struct Segment {
        index: usize,
        p1: Point,
        p2: Point,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
    }
    let mut segments = Vec::with_capacity(n);
    for i in 0..n {
        let p1 = points[i];
        let p2 = points[(i + 1) % n];
        segments.push(Segment {
            index: i,
            p1,
            p2,
            min_x: p1.x.min(p2.x),
            max_x: p1.x.max(p2.x),
            min_y: p1.y.min(p2.y),
            max_y: p1.y.max(p2.y),
        });
    }

    // A uniform grid over the loop's extent: segments are short, so
    // each cell holds only a handful of them and the per-cell pair
    // scan is cheap.
    const CELL_SIZE: f64 = 8.0;
    let inv_cell = 1.0 / CELL_SIZE;
    let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> =
        std::collections::HashMap::with_capacity(n);
    for (i, seg) in segments.iter().enumerate() {
        let x0 = (seg.min_x * inv_cell).floor() as i64;
        let x1 = (seg.max_x * inv_cell).floor() as i64;
        let y0 = (seg.min_y * inv_cell).floor() as i64;
        let y1 = (seg.max_y * inv_cell).floor() as i64;
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                grid.entry((cx, cy)).or_default().push(i);
            }
        }
    }

    for cell_indices in grid.values() {
        for a in 0..cell_indices.len() {
            for b in a + 1..cell_indices.len() {
                let (i, j) = (cell_indices[a], cell_indices[b]);
                // The original R-tree query only ever visits pairs
                // with the smaller command index first.
                let (seg1, seg2) = if i < j {
                    (&segments[i], &segments[j])
                } else {
                    (&segments[j], &segments[i])
                };
                // Bounding boxes must overlap for the segments to
                // intersect; this mirrors the envelope filter of the
                // original query.
                if seg1.max_x < seg2.min_x
                    || seg2.max_x < seg1.min_x
                    || seg1.max_y < seg2.min_y
                    || seg2.max_y < seg1.min_y
                {
                    continue;
                }
                let Some(pt) = get_line_segment_intersection(
                    seg1.p1, seg1.p2, seg2.p1, seg2.p2,
                ) else {
                    continue;
                };
                if seg2.index == seg1.index + 1 {
                    // The adjacent pair shares the vertex at the end
                    // of the first command. Any intersection away from
                    // that vertex means the band folded over itself.
                    if pt.distance_squared(seg1.p2) < EPSILON_INTERSECT {
                        continue;
                    }
                    result.push((seg1.index, seg2.index, pt));
                    if result.len() >= limit {
                        return result;
                    }
                    continue;
                }
                let at_end1 = pt.distance_squared(seg1.p1) < EPSILON_INTERSECT
                    || pt.distance_squared(seg1.p2) < EPSILON_INTERSECT;
                let at_end2 = pt.distance_squared(seg2.p1) < EPSILON_INTERSECT
                    || pt.distance_squared(seg2.p2) < EPSILON_INTERSECT;
                if at_end1 || at_end2 {
                    continue;
                }
                result.push((seg1.index, seg2.index, pt));
                if result.len() >= limit {
                    return result;
                }
            }
        }
    }
    // Transversal self-touches at shared points: the segments all
    // share an endpoint there, so the scan above cannot see them.
    let infinite = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (i, j, pt) in find_point_crossings(
        points,
        infinite,
        Point::new(f64::INFINITY, f64::INFINITY),
        limit.saturating_sub(result.len()),
    ) {
        result.push((i, j, pt));
        if result.len() >= limit {
            return result;
        }
    }
    result
}

/// Arc length in samples from each band point to the nearest touching
/// points, measured forward and backward around the closed band.
fn nearest_touch_distances(touch: &[bool]) -> (Vec<f64>, Vec<f64>) {
    let n = touch.len();
    let mut forward = vec![f64::INFINITY; n];
    let mut backward = vec![f64::INFINITY; n];
    let mut last = f64::INFINITY;
    for i in 0..2 * n {
        let idx = i % n;
        if touch[idx] {
            last = 0.0;
        } else {
            last += 1.0;
        }
        forward[idx] = forward[idx].min(last);
    }
    let mut last = f64::INFINITY;
    for i in (0..2 * n).rev() {
        let idx = i % n;
        if touch[idx] {
            last = 0.0;
        } else {
            last += 1.0;
        }
        backward[idx] = backward[idx].min(last);
    }
    (forward, backward)
}
