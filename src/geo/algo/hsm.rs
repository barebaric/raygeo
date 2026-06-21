//! HSM cutting-arc geometry primitives.
//!
//! Pure geometric helpers for adaptive clearing: cutting-arc extraction,
//! arc filleting, and safe-sweep detection. Motion assembly (entry
//! strategy, wavefront expansion, peeling, arc linking) lives in
//! [`crate::ops::assembly::hsm`].

use crate::geo::shape::arc::get_polyline_turn_sign;
use crate::geo::shape::line::{
    get_interior_angle, get_segment_segment_distance,
};
use crate::geo::shape::polygon::{
    does_path_sweep_intersect_polygon, get_polygon_closest_point,
    trim_polyline_at,
};
use crate::types::{Point, Polygon};

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
        let angle_curr = get_interior_angle(bite[(first + n - 1) % n], b, c);
        let angle_next = get_interior_angle(b, c, d);
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
        let angle_curr = get_interior_angle(a, b, c);
        let angle_prev = get_interior_angle(a_prev, a, b);
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
