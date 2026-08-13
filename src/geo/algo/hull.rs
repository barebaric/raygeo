use crate::geo::algo::intersect::check_self_intersection_from_array;
use crate::geo::shape::polygon::get_polygon_convex_hull;
use crate::geo::types::Point;

/// Spacing between band particles, in pixels.
const BAND_SPACING: f64 = 1.0;

/// Maximum number of shrink iterations before the result is returned.
const MAX_SHRINK_ITERATIONS: usize = 200;

/// Gravity increment consumed per shrink pass.
const GRAVITY_STEP: f64 = 0.05;

/// Number of bisections used to find the largest pull scale at which
/// the band still forms a simple loop.
const SELF_INTERSECTION_BISECTIONS: usize = 12;

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

pub fn find_external_contours(
    image: &[u8],
    width: usize,
    height: usize,
) -> Vec<Vec<Point>> {
    let mut visited = vec![false; width * height];
    let mut contours: Vec<Vec<Point>> = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if image[idx] == 0 || visited[idx] {
                continue;
            }
            if x == 0 || image[idx - 1] == 0 {
                let contour =
                    trace_contour(image, width, height, &mut visited, x, y);
                if contour.len() >= 3 {
                    contours.push(contour);
                }
            }
        }
    }
    contours
}

fn trace_contour(
    image: &[u8],
    width: usize,
    height: usize,
    visited: &mut [bool],
    start_x: usize,
    start_y: usize,
) -> Vec<Point> {
    let mut contour: Vec<Point> = Vec::new();
    let dirs: [(i32, i32); 8] = [
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
        (0, -1),
        (1, -1),
    ];

    let mut cx = start_x as i32;
    let mut cy = start_y as i32;
    contour.push(Point::new(cx as f64, cy as f64));

    let mut dir_idx = 7;
    let max_steps = width * height * 2;

    for _ in 0..max_steps {
        let search_start = (dir_idx + 6) % 8;
        let mut found = false;

        for i in 0..8 {
            let d = (search_start + i) % 8;
            let nx = cx + dirs[d].0;
            let ny = cy + dirs[d].1;
            if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                let nidx = ny as usize * width + nx as usize;
                if image[nidx] != 0 {
                    cx = nx;
                    cy = ny;
                    dir_idx = d;
                    visited[nidx] = true;
                    contour.push(Point::new(cx as f64, cy as f64));
                    found = true;
                    break;
                }
            }
        }

        if !found {
            break;
        }

        if cx == start_x as i32 && cy == start_y as i32 {
            break;
        }
    }

    contour
}

fn convex_hull_points(points: &[Point]) -> Vec<Point> {
    let hull = get_polygon_convex_hull(&points.to_vec());
    if hull.len() < 3 {
        return hull;
    }
    hull
}

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

    let hull = convex_hull_points(&all_points);
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
        let hull = convex_hull_points(contour);
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
    let effective_gravity = gravity.clamp(0.0, 1.0);
    if effective_gravity < 1e-6 {
        return get_enclosing_hull(image, width, height);
    }

    let contours = find_external_contours(image, width, height);
    if contours.is_empty() {
        return None;
    }

    let all_points: Vec<Point> = contours.iter().flatten().copied().collect();
    if all_points.len() < 3 {
        return get_enclosing_hull(image, width, height);
    }

    let hull_vertices = convex_hull_points(&all_points);
    let num_hull = hull_vertices.len();
    if num_hull < 3 {
        return get_enclosing_hull(image, width, height);
    }

    // Walk along the hull, sampling at small distances.
    let mut band = sample_band(&hull_vertices);

    // Integrate the shrink over the gravity in small increments, so
    // the result changes continuously with the requested gravity.
    // Whenever a step's pull would intersect a shape, it is scaled
    // back to just touch it and only the applied fraction of the step
    // is consumed from the gravity budget.
    let mut applied = 0.0f64;
    for _ in 0..MAX_SHRINK_ITERATIONS {
        if applied >= effective_gravity - 1e-9 {
            break;
        }
        let step = GRAVITY_STEP.min(effective_gravity - applied);
        let (next, scale) = shrink_band(
            &band,
            image,
            width,
            height,
            step,
            allow_self_intersections,
        );
        band = next;
        applied += step * scale;
        if scale < 1e-3 {
            break;
        }
    }

    // When self-intersections are prevented, smooth the stretches
    // where the band is aligned with itself (the pinch corridors).
    if !allow_self_intersections {
        smooth_aligned_sections(&mut band, width, height);
    }

    Some(band)
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
    let mut cells: Vec<Vec<usize>> = vec![Vec::new(); width * height];
    for (i, &point) in band.iter().enumerate() {
        let x = point.x.round() as i64;
        let y = point.y.round() as i64;
        if x >= 0 && y >= 0 && x < width as i64 && y < height as i64 {
            cells[y as usize * width + x as usize].push(i);
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
                for &j in &cells[ny as usize * width + nx as usize] {
                    if circular_distance(i, j, num) > SELF_TOUCH_MARGIN {
                        found = true;
                        break;
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
            (lerp_point(&strand_a, fraction) + lerp_point(&strand_b, fraction))
                * 0.5,
        );
    }

    // Smooth the centerline so the wobble of either strand is ironed
    // out.
    for _ in 0..CORRIDOR_SMOOTHING_PASSES {
        centerline = moving_average(&centerline, CORRIDOR_SMOOTHING_RADIUS);
    }

    // Both strands now follow the centerline exactly.
    for k in 0..len_a {
        let fraction = k as f64 / (len_a - 1).max(1) as f64;
        snapped[a.0 + k] = lerp_point(&centerline, fraction);
    }
    for k in 0..len_b {
        let fraction = if b_reversed {
            (len_b - 1 - k) as f64 / (len_b - 1).max(1) as f64
        } else {
            k as f64 / (len_b - 1).max(1) as f64
        };
        snapped[b.0 + k] = lerp_point(&centerline, fraction);
    }
}

/// Linear interpolation along a point sequence.
fn lerp_point(points: &[Point], fraction: f64) -> Point {
    let scaled = fraction * (points.len() - 1) as f64;
    let lower = scaled.floor() as usize;
    let upper = scaled.ceil() as usize;
    if lower == upper {
        return points[lower];
    }
    let t = scaled - lower as f64;
    points[lower] + (points[upper] - points[lower]) * t
}

/// Moving average of a point sequence, clamped at the ends.
fn moving_average(points: &[Point], radius: usize) -> Vec<Point> {
    let n = points.len();
    let mut smoothed = points.to_vec();
    for (i, entry) in smoothed.iter_mut().enumerate() {
        let mut sum = Point::ZERO;
        let mut count = 0.0;
        for o in -(radius as i64)..=(radius as i64) {
            let j = i as i64 + o;
            if j >= 0 && j < n as i64 {
                sum += points[j as usize];
                count += 1.0;
            }
        }
        *entry = sum / count;
    }
    smoothed
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

/// Samples a polygon outline every `BAND_SPACING` pixels.
fn sample_band(outline: &[Point]) -> Vec<Point> {
    let mut samples = Vec::new();
    for i in 0..outline.len() {
        let p0 = outline[i];
        let p2 = outline[(i + 1) % outline.len()];
        let edge = p2 - p0;
        let length = edge.length();
        let steps = ((length / BAND_SPACING).round() as usize).max(1);
        for j in 0..steps {
            let t = j as f64 / steps as f64;
            samples.push(Point::new(p0.x + edge.x * t, p0.y + edge.y * t));
        }
    }
    samples
}

/// One shrink pass: pulls every band point inward along the inward
/// normal, following a sine profile over its free section. Returns the
/// pulled band and the scale the pull had to be reduced to so that no
/// point crosses into a shape.
fn shrink_band(
    band: &[Point],
    image: &[u8],
    width: usize,
    height: usize,
    gravity: f64,
    allow_self_intersections: bool,
) -> (Vec<Point>, f64) {
    let num = band.len();
    if num < 3 {
        return (band.to_vec(), 1.0);
    }

    // The inward normal of each band edge, pointing to the interior
    // side given the band's winding.
    let inward_left = polygon_signed_area(band) >= 0.0;
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

    // A point touches the shape when the content is at or right next
    // to its position. A point also touches the band itself when
    // another, non-adjacent part of the band shares its pixel: such a
    // pinch point then acts like a shape touch, splitting the band
    // into new sections that continue to be pulled.
    let mut touch = vec![false; num];
    let mut self_cell = vec![-1i64; width * height];
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
            let previous = self_cell[cell];
            if previous >= 0 {
                let distance = circular_distance(i, previous as usize, num);
                if distance > SELF_TOUCH_MARGIN {
                    touching = true;
                    touch[previous as usize] = true;
                }
            }
            self_cell[cell] = i as i64;
        }
        touch[i] = touching;
    }

    // The distance along the band to the touches bounding each free
    // section, measured both ways around the closed band.
    let (forward, backward) = nearest_touch_distances(&touch);

    // The desired inward pull per point, following a sine profile: no
    // pull at the touches, a smooth maximum halfway between them.
    let depths: Vec<f64> = forward
        .iter()
        .zip(&backward)
        .map(|(&forward, &backward)| {
            let length = forward + backward;
            if length > 0.0 {
                (forward.min(backward) / length * std::f64::consts::PI).sin()
                    * length
                    * 0.5
                    * gravity
            } else {
                0.0
            }
        })
        .collect();

    // Limit the pull so that no point crosses into a shape: march
    // along the pull direction and find the first content pixel.
    let mut scale = 1.0f64;
    for i in 0..num {
        let depth = depths[i];
        if depth < 1e-9 {
            continue;
        }
        let origin = band[i];
        let normal = normals[i];
        for step in 1..=(depth.ceil() as usize) {
            let probe = origin + normal * (step as f64);
            let x = probe.x.round() as i64;
            let y = probe.y.round() as i64;
            if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
                break;
            }
            if image[y as usize * width + x as usize] != 0 {
                scale = scale.min(step as f64 / depth);
                break;
            }
        }
    }

    // When self-intersections are not allowed, additionally limit the
    // pull to the largest scale at which the band still forms a simple
    // loop.
    if !allow_self_intersections && scale > 1e-9 {
        let candidate = pulled_band(band, &normals, &depths, scale);
        if band_self_intersects(&candidate) {
            let mut low = 0.0f64;
            let mut high = scale;
            for _ in 0..SELF_INTERSECTION_BISECTIONS {
                let middle = 0.5 * (low + high);
                let candidate = pulled_band(band, &normals, &depths, middle);
                if band_self_intersects(&candidate) {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            scale = low;
        }
    }

    let mut pulled = pulled_band(band, &normals, &depths, scale);
    if !allow_self_intersections {
        align_pinched_pairs(&mut pulled, width, height);
    }
    (pulled, scale)
}

/// Pulls the non-adjacent band points that lie within a couple of
/// pixels of each other onto the earlier one, so a pinch resolves into
/// a clean shared line. The fork points where the strands part are
/// merged into shared vertices, and the moved points are then relaxed
/// so the strands leave the pinch smoothly instead of zigzagging.
fn align_pinched_pairs(band: &mut [Point], width: usize, height: usize) {
    let num = band.len();
    let mut moved: Vec<f64> = vec![0.0; num];
    let mut cells: Vec<Option<usize>> = vec![None; width * height];
    for i in 0..num {
        let x = band[i].x.round() as i64;
        let y = band[i].y.round() as i64;
        let mut nearest: Option<usize> = None;
        let mut best = f64::INFINITY;
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let nx = x + dx;
                let ny = y + dy;
                if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64
                {
                    continue;
                }
                let cell = ny as usize * width + nx as usize;
                if let Some(j) = cells[cell] {
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
            if cells[cell].is_none() {
                cells[cell] = Some(i);
            }
        }
    }

    // Keep the snapped band as the fallback in case the relaxation
    // below folds it over.
    let snapped = band.to_vec();

    // Relax the points that moved far and their neighbours, so the
    // kinks they introduced at the pinch forks smooth out. The
    // straight shared stretch is left untouched.
    for _ in 0..4 {
        let relaxed: Vec<Point> = (0..num)
            .map(|i| {
                let affected = moved[i] > 1.0
                    || moved[(i + num - 1) % num] > 1.0
                    || moved[(i + 1) % num] > 1.0;
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

    if band_self_intersects(band) {
        band.copy_from_slice(&snapped);
    }
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
fn band_self_intersects(points: &[Point]) -> bool {
    let mut geometry = crate::geo::geometry::Geometry::new();
    geometry.move_to(points[0].x, points[0].y, 0.0);
    for point in &points[1..] {
        geometry.line_to(point.x, point.y, 0.0);
    }
    geometry.line_to(points[0].x, points[0].y, 0.0);
    check_self_intersection_from_array(&geometry.data, false)
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

/// Signed area of a closed polygon, positive when counter-clockwise.
fn polygon_signed_area(points: &[Point]) -> f64 {
    let mut sum = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        sum += a.x * b.y - b.x * a.y;
    }
    0.5 * sum
}
