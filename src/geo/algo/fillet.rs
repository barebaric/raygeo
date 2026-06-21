//! Pure-geometry fillet operations.
//!
//! Domain-neutral utilities for creating circular fillet arcs,
//! appending them to polylines, and trimming to safe spans.
//! No CNC or machining concepts — terms like "pocket", "island",
//! and "tool" belong in higher layers.

use std::f64::consts::FRAC_PI_2;

use crate::geo::shape::arc::get_polyline_turn_sign;
use crate::geo::shape::line::get_segment_segment_distance;
use crate::geo::shape::polygon::does_path_sweep_intersect_polygon;
use crate::types::Point;

/// Create a circular fillet polyline tangent to `dir` at `p`.
///
/// `side` selects the offset side (+1 = left of `dir`, -1 = right).
/// When `reverse` is true the arc curls back opposite to `dir`.
/// Returns `(center, polyline)` — the arc centre and the fillet
/// vertices starting at `p` and ending at the far tangent point.
pub fn create_fillet_polyline(
    p: Point,
    dir: Point,
    radius: f64,
    sweep_angle: f64,
    side: f64,
    reverse: bool,
) -> (Point, Vec<Point>) {
    let d = if dir.length_squared() > 0.0 {
        dir / dir.length()
    } else {
        Point::new(1.0, 0.0)
    };
    let n = Point::new(-d.y, d.x);
    let c = p + side * n * radius;

    let a1 = (p - c).y.atan2((p - c).x);
    let sweep = if reverse {
        -side * sweep_angle
    } else {
        side * sweep_angle
    };
    let n_arc =
        (sweep.abs() * 4.0 / FRAC_PI_2).ceil().clamp(4.0, 64.0) as usize;

    let mut arc = vec![p];
    for j in 1..=n_arc {
        let t = j as f64 / n_arc as f64;
        let a = a1 + sweep * t;
        arc.push(c + Point::new(radius * a.cos(), radius * a.sin()));
    }
    (c, arc)
}

/// Append fillet arcs to both ends of an open polyline.
///
/// A reversed fillet is added at the start and a forward fillet at
/// the end, producing a smooth rounded path.  Returns the full
/// polyline with fillets in order.
pub fn append_end_fillets(
    polyline: &[Point],
    radius: f64,
    sweep_angle: f64,
    side: f64,
) -> Vec<Point> {
    if polyline.len() < 2 {
        return polyline.to_vec();
    }
    let start_dir = polyline[1] - polyline[0];
    let last = polyline.len() - 1;
    let end_dir = polyline[last] - polyline[last - 1];

    let (_, start_arc) = create_fillet_polyline(
        polyline[0],
        start_dir,
        radius,
        sweep_angle,
        side,
        true,
    );
    let (_, end_arc) = create_fillet_polyline(
        polyline[last],
        end_dir,
        radius,
        sweep_angle,
        side,
        false,
    );

    let mut path =
        Vec::with_capacity(start_arc.len() + polyline.len() + end_arc.len());
    path.extend(start_arc.iter().rev().copied());
    path.extend(polyline.iter().skip(1).copied());
    path.extend(end_arc.iter().skip(1).copied());
    path
}

/// Find the longest sub-span of `polyline` whose end fillets do not
/// collide with `outer_boundary` or `inner_obstacles`.
///
/// Each end is tested independently: only the fillet that extends
/// perpendicular at each trim point can collide.  Walks inward from
/// each end, then binary-searches the crossing edge for sub-vertex
/// precision.  `margin` adds extra clearance beyond tangency
/// (`0.0` allows the sweep to touch the boundary).
///
/// Returns `(enter, exit)` — the first and last points of the safe
/// sub-span, or `None` when no usable safe span remains.
pub fn trim_to_safe_fillet_span(
    polyline: &[Point],
    outer_boundary: &[Point],
    inner_obstacles: &[Vec<Point>],
    radius: f64,
    margin: f64,
) -> Option<(Point, Point)> {
    let n = polyline.len();
    if n < 3 || radius <= 0.0 {
        return None;
    }
    let radius_eff = radius + margin;
    let side = get_polyline_turn_sign(polyline);
    let last = n - 1;

    fn fillet_collides(
        farc: &[Point],
        outer_boundary: &[Point],
        inner_obstacles: &[Vec<Point>],
        radius_eff: f64,
    ) -> bool {
        if farc.len() < 2 {
            return false;
        }
        if !inner_obstacles.is_empty()
            && does_path_sweep_intersect_polygon(
                farc,
                radius_eff,
                inner_obstacles,
            )
        {
            return true;
        }
        let pn = outer_boundary.len();
        if pn >= 3 {
            for w in farc.windows(2) {
                let a = w[0];
                let b = w[1];
                for j in 0..pn {
                    let c = outer_boundary[j];
                    let d = outer_boundary[(j + 1) % pn];
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
        let dir0 = polyline[1] - polyline[0];
        let (_, f0) = create_fillet_polyline(
            polyline[0],
            dir0,
            radius,
            FRAC_PI_2,
            side,
            true,
        );
        if !fillet_collides(&f0, outer_boundary, inner_obstacles, radius_eff) {
            return Some(polyline[0]);
        }
        for lo in 1..last {
            let dir = polyline[lo + 1] - polyline[lo];
            let (_, f) = create_fillet_polyline(
                polyline[lo],
                dir,
                radius,
                FRAC_PI_2,
                side,
                true,
            );
            if !fillet_collides(&f, outer_boundary, inner_obstacles, radius_eff)
            {
                // Binary-search edge (lo-1, lo) for the crossing.
                let a = polyline[lo - 1];
                let b = polyline[lo];
                let ab = b - a;
                let mut lo_t = 0.0;
                let mut hi_t = 1.0;
                for _ in 0..24 {
                    let mid_t = (lo_t + hi_t) / 2.0;
                    let p = a + ab * mid_t;
                    let dir_p = b - p;
                    let (_, f) = create_fillet_polyline(
                        p, dir_p, radius, FRAC_PI_2, side, true,
                    );
                    if !fillet_collides(
                        &f,
                        outer_boundary,
                        inner_obstacles,
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
        let dir_last = polyline[last] - polyline[last - 1];
        let (_, f0) = create_fillet_polyline(
            polyline[last],
            dir_last,
            radius,
            FRAC_PI_2,
            side,
            false,
        );
        if !fillet_collides(&f0, outer_boundary, inner_obstacles, radius_eff) {
            return Some(polyline[last]);
        }
        for hi in (1..last).rev() {
            let dir = polyline[hi] - polyline[hi - 1];
            let (_, f) = create_fillet_polyline(
                polyline[hi],
                dir,
                radius,
                FRAC_PI_2,
                side,
                false,
            );
            if !fillet_collides(&f, outer_boundary, inner_obstacles, radius_eff)
            {
                // Binary-search edge (hi, hi+1) for the crossing.
                let a = polyline[hi];
                let b = polyline[hi + 1];
                let ab = b - a;
                let mut lo_t = 0.0;
                let mut hi_t = 1.0;
                for _ in 0..24 {
                    let mid_t = (lo_t + hi_t) / 2.0;
                    let p = a + ab * mid_t;
                    let dir_p = p - polyline[hi - 1];
                    let (_, f) = create_fillet_polyline(
                        p, dir_p, radius, FRAC_PI_2, side, false,
                    );
                    if !fillet_collides(
                        &f,
                        outer_boundary,
                        inner_obstacles,
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
    if (exit - enter).length_squared() < radius * radius {
        return None;
    }
    Some((enter, exit))
}
