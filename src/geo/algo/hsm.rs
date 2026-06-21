//! HSM (High-Speed Machining) adaptive clearing strategies.

use crate::geo::algo::cleared_area::ClearedArea;
use crate::geo::algo::helix::{generate_helix, HelixDirection, HelixOptions};
use crate::geo::algo::intersect::get_ray_polygon_intersection;
use crate::geo::algo::medial_axis::{
    compute_medial_axis, mat_path, MedialAxis,
};
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::ramp::{generate_ramp, RampOptions, RampStyle};
use crate::geo::algo::smooth::smooth_path;
use crate::geo::algo::spiral::{generate_spiral, SpiralOptions};
use crate::geo::shape::arc::get_polyline_turn_sign;
use crate::geo::shape::line::{
    does_line_cross_polygon, get_segment_segment_distance,
    longest_line_through_point,
};
use crate::geo::shape::polygon::{
    does_path_sweep_intersect_polygon, get_circle_polygon, get_polygon_area,
    get_polygon_bounds, get_polygon_centroid, get_polygon_closest_point,
    get_polygons_group_difference, get_segment_swept_polygon, trim_polyline_at,
};
use crate::types::{Point, Point3D, Polygon};

/// Options for [`adaptive_entry`].
#[derive(Clone, Debug)]
pub struct AdaptiveEntryOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub tool_radius: f64,
    pub step_over: f64,
    pub safe_z: f64,
    pub target_z: f64,
    pub plunge_pitch: f64,
    pub safe_margin: f64,
    pub angular_step: f64,
}

/// Return type of [`adaptive_entry`].
#[derive(Clone, Debug)]
pub struct AdaptiveEntryResult {
    pub toolpath: Vec<Point3D>,
    pub cleared_polygons: Vec<Polygon>,
}

/// Fast central clearing entry.
///
/// Given a pocket boundary (with optional islands), finds the optimal entry
/// pole and generates either:
///
/// - **Helix → Spiral** (wide area, `r_max > 1.5 × tool_radius`): a helical
///   plunge to depth followed by a flat Archimedean spiral.
/// - **ZigZag Ramp** (tight slot, `r_max ≤ 1.5 × tool_radius`): a trochoidal
///   ramp along the longest axis of the slot.
///
/// The result includes the toolpath and the swept polygons (tool-disk or
/// segment-swept) that should be added to the `ClearedArea`.
pub fn adaptive_entry(opts: &AdaptiveEntryOptions) -> AdaptiveEntryResult {
    let (entry_pt, r_max) =
        find_largest_circle(&opts.pocket_boundary, &opts.islands, 0.1)
            .unwrap_or_else(|| {
                let c = get_polygon_centroid(&opts.pocket_boundary);
                (c, 0.0)
            });

    let mut toolpath = Vec::new();

    if r_max > opts.tool_radius * 1.5 {
        let helix_r = (opts.tool_radius * 0.8).min(r_max * 0.5);

        if opts.target_z < opts.safe_z {
            let hp = generate_helix(&HelixOptions {
                center: entry_pt,
                start_radius: helix_r,
                end_radius: helix_r,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                pitch: opts.plunge_pitch,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
                min_revolutions: None,
            });
            toolpath.extend(hp);
        }

        let spiral_max_r =
            (r_max - opts.tool_radius - opts.safe_margin).max(helix_r + 0.01);
        let radial_dist = spiral_max_r - helix_r;

        if radial_dist > 0.0 && opts.step_over > 0.0 {
            let n_revs = radial_dist / opts.step_over;

            let sp = generate_spiral(&SpiralOptions {
                center: entry_pt,
                z: opts.target_z,
                start_radius: helix_r,
                end_radius: spiral_max_r,
                revolutions: n_revs,
                direction: HelixDirection::Cw,
                angular_step: opts.angular_step,
            });
            toolpath.extend(sp);
        }

        let disk_r = spiral_max_r + opts.tool_radius;
        let cleared_polygons = vec![get_circle_polygon(entry_pt, disk_r, 64)];

        AdaptiveEntryResult {
            toolpath,
            cleared_polygons,
        }
    } else {
        let bbox = get_polygon_bounds(&opts.pocket_boundary);
        let (start, end) = longest_line_through_point(entry_pt, bbox);

        if opts.target_z < opts.safe_z {
            let rp = generate_ramp(&RampOptions {
                start,
                end,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                max_ramp_angle_deg: 45.0,
                style: RampStyle::ZigZag,
                lateral_amplitude: opts.tool_radius * 0.8,
            });
            toolpath.extend(rp);
        }

        let cleared_polygons =
            get_segment_swept_polygon(start, end, opts.tool_radius);

        AdaptiveEntryResult {
            toolpath,
            cleared_polygons,
        }
    }
}

const MAX_WAVEFRONT_ITERATIONS: usize = 1000;

/// Options for [`adaptive_wavefronts`] and [`adaptive_peeling`].
#[derive(Clone, Debug)]
pub struct AdaptiveWavefrontOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub tool_radius: f64,
    pub step_over: f64,
    pub z: f64,
    pub area_tolerance: f64,
    /// Retract / safe Z height for return loops in D-cut passes.
    /// When `safe_z == z` no lift is applied.
    pub safe_z: f64,
    /// Extra clearance (beyond tangency) kept between the tool sweep
    /// (arc + end fillets) and the pocket wall / islands when trimming
    /// cutting arcs.  `0.0` lets the sweep touch the wall; larger values
    /// leave a sliver of safety margin.
    pub wall_margin: f64,
    /// Gaussian smoothing amount (0–200) applied to MAT-routed travel
    /// segments.  `0` disables smoothing (shortcut only).  See
    /// [`smooth_path`].
    pub travel_smoothing: i32,
    /// Optional pre-computed Medial Axis.  If `None` the peeler will
    /// compute it internally.  After the call this field holds the
    /// (computed or provided) MAT, which can be extracted for use in
    /// `link_filleted_arcs`.
    pub mat: Option<MedialAxis>,
}

/// Return type of [`adaptive_wavefronts`].
#[derive(Clone, Debug)]
pub struct AdaptiveWavefrontResult {
    pub toolpaths: Vec<Vec<Point3D>>,
    pub iterations: usize,
}

/// Inside-out adaptive wavefronts.
///
/// Starting from the cleared area, each iteration expands the frontier
/// (outer boundary) outward by `step_over`, clips to the valid tool
/// area, traces the wavefront, and updates the cleared state.
///
/// Each iteration produces one toolpath — a sequence of 3D points at
/// height `z` tracing the boundaries of all wavefront fragments.
pub fn adaptive_wavefronts(
    cleared: &mut ClearedArea,
    opts: &AdaptiveWavefrontOptions,
) -> AdaptiveWavefrontResult {
    let (valid_tool_area, valid_total_area) = compute_inset_region(
        &opts.pocket_boundary,
        opts.tool_radius,
        &opts.islands,
    );

    let mut toolpaths = Vec::new();

    for _ in 0..MAX_WAVEFRONT_ITERATIONS {
        let _iter_prof = crate::prof::prof_guard("wf_iteration");

        // Compute the "bites" — the new material reachable by expanding
        // the current frontier outward by step_over, clipped to the
        // valid-tool-area and with already-cleared portions removed.
        let bounded = cleared.bites(opts.step_over, &valid_tool_area, 0.01);
        if bounded.is_empty() {
            break;
        }

        // Add newly reached material to the cleared state.
        let _prof = crate::prof::prof_guard("wf_ring");
        let new_ring = cleared.incorporate(&bounded);
        drop(_prof);
        if new_ring.is_empty() {
            continue;
        }

        // Trace new ring into toolpath with NaN separators between
        // disjoint fragments so matplotlib doesn't draw spurious
        // connecting lines.
        let _prof = crate::prof::prof_guard("wf_trace");
        toolpaths.push(trace_ring(&new_ring, opts.z));
        drop(_prof);

        let ring_area: f64 = new_ring.iter().map(get_polygon_area).sum();
        if ring_area < opts.area_tolerance
            || cleared.total_area() >= valid_total_area - 0.1
        {
            break;
        }
    }

    let n = toolpaths.len();
    AdaptiveWavefrontResult {
        toolpaths,
        iterations: n,
    }
}

/// Generate a single D-cut pass from a bite polygon.
///
/// The bite is a crescent between the cleared frontier (inner arc) and
/// the expanded boundary (outer arc).  The D-cut traces the **outer arc**
/// at cutting depth `z` (the belly of the D), then returns **straight
/// through cleared space** at `safe_z` from the end of the cut back to
/// the start (the back of the D).
///
/// The first return point is offset by `tool_radius` along the return
/// direction, and a circular fillet arc of radius `tool_radius` is
/// inserted at the D-junction to smooth the corner between the cutting
/// arc and the return line.
///
/// If the direct return would cross an island the function falls back
/// to the inner arc (cleared boundary) which naturally avoids obstacles.
/// Extract the cutting-arc (outer) vertices from a bite polygon.
///
/// The bite is a crescent between the cleared frontier and the expanded
/// boundary.  The cutting arc is the longest contiguous run of bite
/// vertices that lie *outside* all cleared fragments.
///
/// Returns `(arc_vertices, cut_start, cut_len)` where `arc_vertices` is
/// the contiguous slice of `bite` forming the outer arc, `cut_start`
/// is the index into `bite`, and `cut_len` is the number of vertices
/// in the arc.  Returns `None` when the bite is degenerate (no outer
/// arc found).
pub fn find_cutting_arc(
    bite: &Polygon,
    cleared_fragments: &[Polygon],
) -> Option<(Vec<Point>, usize, usize)> {
    let n = bite.len();
    if n < 3 {
        return None;
    }

    let is_outer: Vec<bool> = bite
        .iter()
        .map(|p| {
            !cleared_fragments.iter().any(|frag| {
                let d2 = get_polygon_closest_point(frag, p.x, p.y)
                    .map(|(_, _, d2)| d2)
                    .unwrap_or(f64::MAX);
                d2 < 1e-2
            })
        })
        .collect();

    let extended: Vec<bool> = is_outer
        .iter()
        .copied()
        .chain(is_outer.iter().copied())
        .collect();

    let mut cut_start = 0usize;
    let mut cut_len = 0usize;
    {
        let mut cs: Option<usize> = None;
        let mut cl = 0usize;
        for (i, &val) in extended.iter().enumerate() {
            if val {
                if cs.is_none() {
                    cs = Some(i);
                    cl = 1;
                } else {
                    cl += 1;
                }
                if cl > cut_len {
                    cut_start = cs.unwrap();
                    cut_len = cl;
                }
            } else {
                cs = None;
                cl = 0;
            }
        }
    }

    if cut_len < 3 {
        return None;
    }

    // Interior angle at `curr` formed by edges `prev→curr` and `curr→next`.
    fn interior_angle(curr: Point, prev: Point, next: Point) -> f64 {
        let v1 = prev - curr;
        let v2 = next - curr;
        let d2 = v1.length_squared() * v2.length_squared();
        if d2 < 1e-18 {
            return 0.0;
        }
        (v1.dot(v2) / d2.sqrt()).clamp(-1.0, 1.0).acos()
    }

    // Trim vertices from the ends where the interior angle changes
    // abruptly — these are the transition vertices at the tips where the
    // outer arc meets the inner arc.  We compare each candidate vertex
    // against its inward neighbour: if the angle drops sharply (≥ 25°)
    // the vertex is a tip transition and gets trimmed.  This avoids
    // trimming gradual curves that have steadily tightening angles.
    const DERIV_THRESHOLD: f64 = 0.436_332_312_998_582_4; // 25° in radians
    let mut trimmed = true;
    while trimmed && cut_len > 3 {
        trimmed = false;
        let first = (cut_start + 1) % n;
        let b = bite[first];
        let c = bite[(first + 1) % n];
        let d = bite[(first + 2) % n];
        let angle_curr = interior_angle(b, bite[(first + n - 1) % n], c);
        let angle_next = interior_angle(c, b, d);
        if angle_curr + DERIV_THRESHOLD < angle_next {
            cut_start = (cut_start + 1) % n;
            cut_len -= 1;
            trimmed = true;
        }
        let last = (cut_start + cut_len - 2) % n;
        let a = bite[(last + n - 1) % n];
        let b = bite[last];
        let c = bite[(last + 1) % n];
        let a_prev = bite[(last + n - 2) % n];
        let angle_curr = interior_angle(b, a, c);
        let angle_prev = interior_angle(a, a_prev, b);
        if angle_curr + DERIV_THRESHOLD < angle_prev {
            cut_len -= 1;
            trimmed = true;
        }
    }

    if cut_len < 3 {
        return None;
    }

    let vertices: Vec<Point> =
        (0..cut_len).map(|i| bite[(cut_start + i) % n]).collect();
    Some((vertices, cut_start, cut_len))
}

/// Generate a 90° quarter-circle fillet arc of `radius`, tangent to
/// `edge_dir` at `p`.  `sign` selects the offset side (±1).
/// When `reverse` is true the arc goes opposite to `edge_dir` at `p`
/// so that when reversed in the assembly it matches `edge_dir`.
pub(crate) fn quarter_fillet(
    p: Point,
    edge_dir: Point,
    radius: f64,
    sign: f64,
    reverse: bool,
) -> (Point, Vec<Point>) {
    let d = if edge_dir.length_squared() > 0.0 {
        edge_dir / edge_dir.length()
    } else {
        Point::new(1.0, 0.0)
    };
    let n = Point::new(-d.y, d.x);
    let c = p + sign * n * radius;
    let e = if reverse {
        c - sign * d * radius
    } else {
        c + sign * d * radius
    };

    let a1 = (p - c).y.atan2((p - c).x);
    let sweep = if reverse {
        -sign * std::f64::consts::FRAC_PI_2
    } else {
        sign * std::f64::consts::FRAC_PI_2
    };
    let n_arc = (sweep.abs() * 4.0).ceil().clamp(4.0, 64.0) as usize;

    let mut arc = vec![p];
    for j in 1..n_arc {
        let t = j as f64 / n_arc as f64;
        let a = a1 + sweep * t;
        arc.push(c + Point::new(radius * a.cos(), radius * a.sin()));
    }
    arc.push(e);
    (c, arc)
}

/// Round both ends of a cutting arc so that the tool sweep (arc + end
/// fillets of `tool_radius`) stays inside the pocket and clear of
/// islands.
///
/// The arc is trimmed via [`find_safe_sweep_end`] to the longest
/// sub-arc whose fillet path does not collide with `pocket_boundary`
/// or `islands`.  A 90° quarter-circle fillet of `tool_radius` is then
/// appended at each end — tangent to the arc and curling toward the
/// cleared (concave) side — so the path transitions smoothly into the
/// return move.
pub fn fillet_arc_ends(
    arc: &[Point],
    pocket_boundary: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    wall_margin: f64,
) -> Vec<Point> {
    if arc.len() < 3 || tool_radius <= 0.0 {
        return arc.to_vec();
    }

    let Some((enter, exit)) = find_safe_sweep_end(
        arc,
        pocket_boundary,
        islands,
        tool_radius,
        wall_margin,
    ) else {
        return arc.to_vec();
    };

    let trimmed = trim_polyline_at(arc, enter, exit);
    if trimmed.len() < 3 {
        return trimmed;
    }

    let sign = get_polyline_turn_sign(arc);
    build_fillet_candidate(&trimmed, tool_radius, sign)
}

/// Link a sequence of filleted cutting arcs into a single continuous
/// 3-D polyline.
///
/// Consecutive arcs are joined by a straight segment at `safe_z`.
/// When the straight segment would cross (or pass within `safe_margin`
/// of) any polygon in `uncleared`, the connection uses `mat` (the
/// Medial Axis) to route around obstacles.  Falls back to a direct
/// line when MAT routing is unavailable or no path exists.
#[allow(clippy::too_many_arguments)]
pub fn link_filleted_arcs(
    arcs: &[Vec<Point>],
    uncleared: &[Polygon],
    z: f64,
    safe_z: f64,
    mat: Option<&MedialAxis>,
    preserve_order: bool,
    safe_margin: f64,
    smoothing_amount: i32,
) -> Vec<Point3D> {
    let mut result: Vec<Point3D> = Vec::new();

    let order: Vec<usize> = if preserve_order || arcs.is_empty() {
        (0..arcs.len()).collect()
    } else {
        // Reorder by nearest-neighbour starting from the longest arc.
        let mut used = vec![false; arcs.len()];
        let mut o = Vec::with_capacity(arcs.len());
        let start_idx = (0..arcs.len())
            .max_by(|&i, &j| arcs[i].len().cmp(&arcs[j].len()))
            .unwrap_or(0);
        o.push(start_idx);
        used[start_idx] = true;
        while o.len() < arcs.len() {
            let last_end = *arcs[*o.last().unwrap()].last().unwrap();
            let mut best = None;
            let mut best_d2 = f64::MAX;
            for (i, arc) in arcs.iter().enumerate() {
                if used[i] || arc.len() < 2 {
                    continue;
                }
                let d2 = (arc[0] - last_end).length_squared();
                if d2 < best_d2 {
                    best_d2 = d2;
                    best = Some(i);
                }
            }
            if let Some(i) = best {
                o.push(i);
                used[i] = true;
            } else {
                break;
            }
        }
        o
    };

    for &oi in &order {
        let arc = &arcs[oi];
        if arc.len() < 2 {
            continue;
        }
        if result.is_empty() {
            for p in arc.iter() {
                result.push(Point3D::new(p.x, p.y, z));
            }
        } else {
            let last: Point = {
                let p = *result.last().unwrap();
                Point::new(p.x, p.y)
            };
            let first: Point = arc[0];

            // Build connection at safe_z.
            // Dilate uncleared by safe_margin so the tool doesn't
            // clip obstacles even on a near-miss.
            let blocked = if safe_margin > 0.0 {
                let margin2 = safe_margin * safe_margin;
                uncleared.iter().any(|poly| {
                    // Quick interior crossing test first (cheaper).
                    if does_line_cross_polygon(last, first, poly) {
                        return true;
                    }
                    for i in 0..poly.len() {
                        let a = poly[i];
                        let b = poly[(i + 1) % poly.len()];
                        let d = get_segment_segment_distance(last, first, a, b);
                        if d * d < margin2 {
                            return true;
                        }
                    }
                    false
                })
            } else {
                uncleared
                    .iter()
                    .any(|poly| does_line_cross_polygon(last, first, poly))
            };

            let link: Vec<Point> = if blocked {
                let mat_link = mat
                    .and_then(|ma| mat_path(ma, last, first))
                    .unwrap_or_else(|| vec![last, first]);
                // mat_path returns only MAT node positions — prepend the
                // previous arc end and append the next arc start so the
                // travel path connects end-to-start without gaps.
                if mat_link.len() < 2 {
                    vec![last, first]
                } else {
                    let mut full = Vec::with_capacity(mat_link.len() + 2);
                    full.push(last);
                    full.extend(mat_link);
                    if (full.last().unwrap() - first).length_squared() > 1e-12 {
                        full.push(first);
                    }
                    smooth_path(&full, uncleared, safe_margin, smoothing_amount)
                }
            } else {
                vec![last, first]
            };

            // Push the full link at safe_z.
            for wp in &link {
                result.push(Point3D::new(wp.x, wp.y, safe_z));
            }

            // Skip the first arc point if it duplicates the last link point.
            let skip_start =
                (arc[0] - *link.last().unwrap()).length_squared() < 1e-12;
            for (i, p) in arc.iter().enumerate() {
                if i == 0 && skip_start {
                    continue;
                }
                result.push(Point3D::new(p.x, p.y, z));
            }
        }
    }
    result
}

/// Build the full candidate path (start fillet reversed + sub-arc + end
/// fillet) for a given sub-arc slice.  Used both by the sweep-safety
/// search inside [`find_safe_sweep_end`] and by [`fillet_arc_ends`] for
/// the final assembly, ensuring the tested path equals the emitted one.
fn build_fillet_candidate(sub: &[Point], radius: f64, sign: f64) -> Vec<Point> {
    if sub.len() < 2 {
        return sub.to_vec();
    }
    let start_dir = sub[1] - sub[0];
    let last = sub.len() - 1;
    let end_dir = sub[last] - sub[last - 1];

    let (_, start_arc) = quarter_fillet(sub[0], start_dir, radius, sign, true);
    let (_, end_arc) = quarter_fillet(sub[last], end_dir, radius, sign, false);

    let mut path =
        Vec::with_capacity(start_arc.len() + sub.len() + end_arc.len());
    path.extend(start_arc.iter().rev().copied());
    path.extend(sub.iter().skip(1).copied());
    path.extend(end_arc.iter().skip(1).copied());
    path
}

/// Find the longest sub-arc of `arc` whose end fillets (quarter-circles
/// of `tool_radius`) do not collide with the pocket boundary or islands.
///
/// Returns `(enter, exit)` — the first and last points of the safe
/// sub-arc.  Each end is tested **independently**: the arc centre is
/// always inside `valid_tool_area` (so the arc sweep is safe by
/// construction); only the fillet that extends perpendicular at each
/// trim point can collide.  Walks inward from each end, then
/// binary-searches the crossing edge for sub-vertex precision.
/// `wall_margin` adds extra clearance beyond tangency (`0.0` allows
/// the sweep to touch the wall).
///
/// Returns `None` when no safe sub-arc of usable length remains.
pub fn find_safe_sweep_end(
    arc: &[Point],
    pocket_boundary: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    wall_margin: f64,
) -> Option<(Point, Point)> {
    let n = arc.len();
    if n < 3 || tool_radius <= 0.0 {
        return None;
    }
    let radius_eff = tool_radius + wall_margin;
    let sign = get_polyline_turn_sign(arc);
    let last = n - 1;

    /// Test whether a single fillet arc's disk-sweep collides with any
    /// obstacle.  `farc` is the polyline returned by [`quarter_fillet`].
    fn fillet_collides(
        farc: &[Point],
        pocket_boundary: &Polygon,
        islands: &[Polygon],
        radius_eff: f64,
    ) -> bool {
        if farc.len() < 2 {
            return false;
        }
        if !islands.is_empty()
            && does_path_sweep_intersect_polygon(farc, radius_eff, islands)
        {
            return true;
        }
        let pn = pocket_boundary.len();
        if pn >= 3 {
            for w in farc.windows(2) {
                let a = w[0];
                let b = w[1];
                for j in 0..pn {
                    let c = pocket_boundary[j];
                    let d = pocket_boundary[(j + 1) % pn];
                    if get_segment_segment_distance(a, b, c, d) < radius_eff {
                        return true;
                    }
                }
            }
        }
        false
    }

    // --- Enter search: first point from the start whose START fillet
    //     does not collide. ---
    let find_enter = || -> Option<Point> {
        let dir0 = arc[1] - arc[0];
        let (_, f0) = quarter_fillet(arc[0], dir0, tool_radius, sign, true);
        if !fillet_collides(&f0, pocket_boundary, islands, radius_eff) {
            return Some(arc[0]);
        }
        for lo in 1..last {
            let dir = arc[lo + 1] - arc[lo];
            let (_, f) = quarter_fillet(arc[lo], dir, tool_radius, sign, true);
            if !fillet_collides(&f, pocket_boundary, islands, radius_eff) {
                // Binary-search edge (lo-1, lo) for the crossing.
                let a = arc[lo - 1];
                let b = arc[lo];
                let ab = b - a;
                let mut lo_t = 0.0;
                let mut hi_t = 1.0;
                for _ in 0..24 {
                    let mid_t = (lo_t + hi_t) / 2.0;
                    let p = a + ab * mid_t;
                    let dir_p = b - p;
                    let (_, f) =
                        quarter_fillet(p, dir_p, tool_radius, sign, true);
                    if !fillet_collides(
                        &f,
                        pocket_boundary,
                        islands,
                        radius_eff,
                    ) {
                        hi_t = mid_t;
                    } else {
                        lo_t = mid_t;
                    }
                }
                return Some(a + ab * hi_t);
            }
        }
        None
    };

    // --- Exit search: last point from the end whose END fillet does
    //     not collide. ---
    let find_exit = || -> Option<Point> {
        let dir_last = arc[last] - arc[last - 1];
        let (_, f0) =
            quarter_fillet(arc[last], dir_last, tool_radius, sign, false);
        if !fillet_collides(&f0, pocket_boundary, islands, radius_eff) {
            return Some(arc[last]);
        }
        for hi in (1..last).rev() {
            let dir = arc[hi] - arc[hi - 1];
            let (_, f) = quarter_fillet(arc[hi], dir, tool_radius, sign, false);
            if !fillet_collides(&f, pocket_boundary, islands, radius_eff) {
                // Binary-search edge (hi, hi+1) for the crossing.
                let a = arc[hi];
                let b = arc[hi + 1];
                let ab = b - a;
                let mut lo_t = 0.0;
                let mut hi_t = 1.0;
                for _ in 0..24 {
                    let mid_t = (lo_t + hi_t) / 2.0;
                    let p = a + ab * mid_t;
                    let dir_p = p - arc[hi - 1];
                    let (_, f) =
                        quarter_fillet(p, dir_p, tool_radius, sign, false);
                    if !fillet_collides(
                        &f,
                        pocket_boundary,
                        islands,
                        radius_eff,
                    ) {
                        lo_t = mid_t;
                    } else {
                        hi_t = mid_t;
                    }
                }
                return Some(a + ab * lo_t);
            }
        }
        None
    };

    let enter = find_enter()?;
    let exit = find_exit()?;
    if (exit - enter).length_squared() < tool_radius * tool_radius {
        return None;
    }
    Some((enter, exit))
}

/// Fillet each raw cutting arc (from `find_cutting_arc`) then link them
/// into a single continuous path with MAT routing.
fn link_cutting_arcs(
    cut_arcs: Vec<Vec<Point>>,
    uncleared: &[Polygon],
    opts: &AdaptiveWavefrontOptions,
) -> Vec<Point3D> {
    if cut_arcs.is_empty() {
        return Vec::new();
    }

    // Minimum span to consider an arc meaningful — tiny cleanup arcs
    // produce ugly cluster-jumps in the linked path.
    let min_span = opts.step_over;

    let filleted: Vec<Vec<Point>> = cut_arcs
        .iter()
        .filter_map(|arc| {
            if arc.len() < 3 {
                return None;
            }
            let xs = arc.iter().map(|p| p.x);
            let ys = arc.iter().map(|p| p.y);
            let (xmin, xmax) = xs
                .clone()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .zip(xs.max_by(|a, b| a.partial_cmp(b).unwrap()))
                .unwrap_or((0.0, 0.0));
            let (ymin, ymax) = ys
                .clone()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .zip(ys.max_by(|a, b| a.partial_cmp(b).unwrap()))
                .unwrap_or((0.0, 0.0));
            let span = (xmax - xmin).max(ymax - ymin);
            if span < min_span {
                return None;
            }
            let fa = fillet_arc_ends(
                arc,
                &opts.pocket_boundary,
                &opts.islands,
                opts.tool_radius,
                opts.wall_margin,
            );
            if fa.len() >= 3 {
                Some(fa)
            } else {
                None
            }
        })
        .collect();

    if filleted.is_empty() {
        return Vec::new();
    }

    link_filleted_arcs(
        &filleted,
        uncleared,
        opts.z,
        opts.safe_z,
        opts.mat.as_ref(),
        true,
        opts.tool_radius,
        opts.travel_smoothing,
    )
}

/// Inside-out adaptive peeling (D-biting).
///
/// Whereas [`adaptive_wavefronts`] traces the centreline of each
/// wavefront ring, this function implements the true HSM D-cut
/// strategy:
///
/// 1. Expand the cleared boundary outward by `step_over`.
/// 2. Clip to the valid tool area and subtract cleared space to
///    obtain "bites" — crescents of uncut material.
/// 3. For each bite, extract its *outer* (cutting) edge — the arc that
///    lies outside the cleared area — and trace it as an open D-cut pass.
/// 4. The bite is then added to the cleared area.
/// 5. Repeat until no material remains.
///
/// Returns a single continuous toolpath where each D-cut's cutting arc
/// (at `z`) is connected to the next via a travel segment at `safe_z`.
/// The Medial Axis is used to route travel around obstacles.
pub fn adaptive_peeling(
    cleared: &mut ClearedArea,
    opts: &mut AdaptiveWavefrontOptions,
) -> Vec<Point3D> {
    let (valid_tool_area, valid_total_area) = compute_inset_region(
        &opts.pocket_boundary,
        opts.tool_radius,
        &opts.islands,
    );

    // Compute Medial Axis once to guide clearing directions.
    if opts.mat.is_none() {
        let holes: Vec<Vec<Point>> =
            opts.islands.iter().map(|h| h.to_vec()).collect();
        opts.mat = compute_medial_axis(
            &opts.pocket_boundary,
            &holes,
            opts.tool_radius,
            opts.step_over * 0.5,
        )
        .ok();
    }

    // Collect branch endpoints projected to the outer boundary.
    let centre = get_polygon_centroid(&opts.pocket_boundary);
    let mut targets: Vec<Point> = Vec::new();
    if let Some(ref ma) = opts.mat {
        let mut branches: Vec<usize> = (0..ma.branches.len()).collect();
        branches.sort_by(|&a, &b| {
            let ca = ma.branches[a]
                .clearances
                .iter()
                .cloned()
                .fold(f64::MIN, f64::max);
            let cb = ma.branches[b]
                .clearances
                .iter()
                .cloned()
                .fold(f64::MIN, f64::max);
            cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
        });
        for &bi in &branches {
            let branch = &ma.branches[bi];
            // Pick the point with maximum clearance on each branch.
            let mut best_idx = 0usize;
            let mut best_cl = f64::MIN;
            for (j, &cl) in branch.clearances.iter().enumerate() {
                if cl > best_cl {
                    best_cl = cl;
                    best_idx = j;
                }
            }
            let mat_pt = branch.points[best_idx];
            let dir = (mat_pt - centre).normalize();
            let boundary_pt = get_ray_polygon_intersection(
                centre,
                dir,
                &opts.pocket_boundary,
            )
            .unwrap_or(mat_pt);
            targets.push(boundary_pt);
        }
    }

    let mut cut_arcs: Vec<Vec<Point>> = Vec::new();

    // Directional phase: clear toward each projected boundary point.
    let max_angle = std::f64::consts::FRAC_PI_3; // 60°
    for &target in &targets {
        for _ in 0..MAX_WAVEFRONT_ITERATIONS {
            let bites = cleared.bite_in_direction(
                opts.step_over,
                &valid_tool_area,
                0.01,
                target,
                max_angle,
            );
            if bites.is_empty() {
                break;
            }
            for bite in &bites {
                if let Some((ref arc, _, _)) =
                    find_cutting_arc(bite, cleared.fragments())
                {
                    if arc.len() >= 3 {
                        cut_arcs.push(arc.clone());
                    }
                }
            }
            cleared.incorporate(&bites);
            if cleared.total_area() >= valid_total_area - 0.1 {
                let mut uncleared = opts.islands.clone();
                uncleared.extend(get_polygons_group_difference(
                    &valid_tool_area,
                    cleared.fragments(),
                ));
                return link_cutting_arcs(cut_arcs, &uncleared, opts);
            }
        }
    }

    // Fallback isotropic phase for any remaining material.
    for _ in 0..MAX_WAVEFRONT_ITERATIONS {
        let bites = cleared.bites(opts.step_over, &valid_tool_area, 0.01);
        if bites.is_empty() {
            break;
        }
        for bite in &bites {
            if let Some((ref arc, _, _)) =
                find_cutting_arc(bite, cleared.fragments())
            {
                if arc.len() >= 3 {
                    cut_arcs.push(arc.clone());
                }
            }
        }
        cleared.incorporate(&bites);
        if cleared.total_area() >= valid_total_area - 0.1 {
            break;
        }
    }

    let mut uncleared = opts.islands.clone();
    uncleared.extend(get_polygons_group_difference(
        &valid_tool_area,
        cleared.fragments(),
    ));
    link_cutting_arcs(cut_arcs, &uncleared, opts)
}

/// Trace a set of polygon fragments into a single toolpath with NaN
/// separators between fragments (so downstream renderers don't draw
/// spurious connecting lines).
fn trace_ring(ring: &[Polygon], z: f64) -> Vec<Point3D> {
    let mut path = Vec::new();
    for poly in ring {
        if !path.is_empty() {
            path.push(Point3D::new(f64::NAN, f64::NAN, z));
        }
        for p in poly {
            path.push(Point3D::new(p.x, p.y, z));
        }
    }
    path
}
