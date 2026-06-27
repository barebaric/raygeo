use prof_macros::prof;

use crate::geo::shape::polygon::{
    get_polygon_heading_at, get_polygon_signed_area,
    get_polygons_closest_point, get_polygons_group_intersection,
    is_point_in_polygon,
};
use crate::ops::cut::ClearedArea;
use crate::ops::cut::ToolPose;
use crate::types::Point;

/// Set to `true` to enable verbose search debug logging.
const SEARCH_DEBUG: bool = false;

macro_rules! dbg_log {
    ($($arg:tt)*) => {
        if SEARCH_DEBUG {
            eprintln!($($arg)*);
        }
    };
}

/// Compute the inward offset position from a frontier vertex.
///
/// During normal cutting the tool centre sits at distance `radius -
/// advance` inside the cleared area from the boundary.  Placing the
/// tool directly on the frontier gives ~50% engagement instead of
/// the target, causing the solver to waste iterations steering back
/// to the correct depth.
fn offset_inward(pt: Point, normal: Point, radius: f64, advance: f64) -> Point {
    let offset = (radius - advance).max(0.0);
    pt - normal * offset
}

/// Walk along the cleared-area frontier, clipped to the tool-centre
/// envelope, returning the first vertex whose outward cut-area probe
/// satisfies `accept`.
///
/// `forward` is relative to the polygon's natural storage order (not
/// to a global CCW/CW convention).  Use
/// [`search_frontier_engagement`] which chooses the direction from the
/// start heading.
///
/// The returned position is offset inward (into the cleared area)
/// by `radius - advance` so the tool starts at the correct
/// engagement depth rather than directly on the boundary.
#[allow(clippy::too_many_arguments)]
#[prof]
fn walk_frontier(
    cleared: &ClearedArea,
    start_pos: Point,
    radius: f64,
    step_length: f64,
    advance: f64,
    forward: bool,
    skip_closest: bool,
    mut accept: impl FnMut(f64) -> bool,
) -> Option<ToolPose> {
    let frontier = cleared.frontier(0.001);
    if frontier.is_empty() {
        return None;
    }

    let polys = {
        let envelope = cleared.envelope(radius);
        if envelope.is_empty() {
            frontier
        } else {
            get_polygons_group_intersection(&frontier, &envelope)
        }
    };
    if polys.is_empty() {
        return None;
    }

    let (closest_poly_idx, _t, _closest_pt, _d2) =
        get_polygons_closest_point(&polys, start_pos)?;
    let poly = &polys[closest_poly_idx];
    let n = poly.len();
    if n < 3 {
        return None;
    }

    // Normalise to CCW so "forward" always means increasing index.
    let is_ccw = get_polygon_signed_area(poly) > 0.0;
    let actual_forward = if is_ccw { forward } else { !forward };

    let start_idx = poly
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(start_pos)
                .partial_cmp(&b.distance_squared(start_pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)?;

    let first = if skip_closest { 1 } else { 0 };
    for offset in first..n {
        let idx = if actual_forward {
            (start_idx + offset) % n
        } else {
            (start_idx + n - offset) % n
        };
        let pt = poly[idx];

        let normal_heading = get_polygon_heading_at(poly, pt);
        let normal = Point::new(normal_heading.cos(), normal_heading.sin());

        // Compute the travel direction (tangent) at this vertex.
        let prev_idx = if actual_forward {
            (idx + n - 1) % n
        } else {
            (idx + 1) % n
        };
        let prev_pt = poly[prev_idx];
        let heading = (pt.y - prev_pt.y).atan2(pt.x - prev_pt.x);
        let tangent = Point::new(heading.cos(), heading.sin());

        // Probe from the offset position along the travel direction,
        // measuring the actual cut area the tool would experience on
        // its first step.  This ensures the reengagement point has
        // approximately the target engagement, not just "any" material.
        let offset_pos = offset_inward(pt, normal, radius, advance);
        let probe = offset_pos + tangent * step_length;
        let area = cleared.cut_area(offset_pos, probe, radius);
        if accept(area) {
            dbg_log!(
                "  RESUME_SEARCH  frontier_pt=({:.3},{:.3})  \
                 normal=({:.3},{:.3})  offset_pos=({:.3},{:.3})  \
                 inward={:.3}  probe_area={:.4}  heading={:.4}",
                pt.x,
                pt.y,
                normal.x,
                normal.y,
                offset_pos.x,
                offset_pos.y,
                (radius - advance).max(0.0),
                area,
                heading,
            );
            return Some(ToolPose {
                pos: offset_pos,
                heading,
            });
        }
    }
    None
}

/// Walk the frontier from `start`, using the heading direction to
/// decide whether to walk forward (CCW) or backward (CW).
///
/// Returns the first vertex whose outward cut-area probe by
/// `step_length` falls in `[min_cut_area, max_cut_area]`.
/// The returned position is offset inward by `radius - advance`.
///
/// Set `max_cut_area` to `f64::MAX` when only a lower bound matters.
#[prof]
pub fn search_frontier_engagement(
    cleared: &ClearedArea,
    start: ToolPose,
    radius: f64,
    step_length: f64,
    advance: f64,
    min_cut_area: f64,
    max_cut_area: f64,
) -> Option<ToolPose> {
    let frontier = cleared.frontier(0.001);
    if frontier.is_empty() {
        return None;
    }

    let polys = {
        let envelope = cleared.envelope(radius);
        if envelope.is_empty() {
            frontier
        } else {
            get_polygons_group_intersection(&frontier, &envelope)
        }
    };
    if polys.is_empty() {
        return None;
    }

    let (closest_poly_idx, _t, _closest_pt, _d2) =
        get_polygons_closest_point(&polys, start.pos)?;
    let poly = &polys[closest_poly_idx];
    let n = poly.len();
    if n < 3 {
        return None;
    }
    let start_idx = poly
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(start.pos)
                .partial_cmp(&b.distance_squared(start.pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)?;

    // Determine direction from the heading: compare with the
    // polygon-order tangent at the nearest vertex.
    let heading_vec = Point::new(start.heading.cos(), start.heading.sin());
    let tangent = poly[(start_idx + 1) % n] - poly[start_idx];
    let forward = heading_vec.x * tangent.x + heading_vec.y * tangent.y >= 0.0;

    walk_frontier(
        cleared,
        start.pos,
        radius,
        step_length,
        advance,
        forward,
        true,
        |a| a >= min_cut_area && a <= max_cut_area,
    )
}

/// Walk backward from `start` (opposite to the heading direction)
/// along the cleared-area frontier (clipped to the tool-centre
/// envelope) until engagement **drops**.
///
/// 1. Walk opposite to `start.heading` along the envelope-clipped
///    frontier.
/// 2. Skip vertices without engagement.
/// 3. When a vertex *with* engagement is found, remember it but
///    **keep walking** in the same direction.
/// 4. When engagement **drops below threshold** again, that vertex
///    is the disengagement point — return it with the heading
///    *flipped* (the forward/travel direction at the previous
///    engaged vertex plus π).
///
/// The returned position is offset inward by `radius - advance`.
#[prof]
pub fn search_reengagement(
    cleared: &ClearedArea,
    start: ToolPose,
    radius: f64,
    step_length: f64,
    advance: f64,
    min_cut_area: f64,
) -> Option<ToolPose> {
    let frontier = cleared.frontier(0.1);
    if frontier.is_empty() {
        return None;
    }
    let polys = {
        let envelope = cleared.envelope(radius);
        if envelope.is_empty() {
            frontier
        } else {
            get_polygons_group_intersection(&frontier, &envelope)
        }
    };
    if polys.is_empty() {
        return None;
    }
    let (closest_poly_idx, _t, _closest_pt, _d2) =
        get_polygons_closest_point(&polys, start.pos)?;
    let poly = &polys[closest_poly_idx];
    let n = poly.len();
    if n < 3 {
        return None;
    }

    let start_idx = poly
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(start.pos)
                .partial_cmp(&b.distance_squared(start.pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)?;

    // Walk opposite to the heading direction.
    let backward_vec = Point::new(-start.heading.cos(), -start.heading.sin());
    let ccw_tangent = poly[(start_idx + 1) % n] - poly[start_idx];
    let cw_tangent = poly[(start_idx + n - 1) % n] - poly[start_idx];
    let dot_ccw =
        backward_vec.x * ccw_tangent.x + backward_vec.y * ccw_tangent.y;
    let dot_cw = backward_vec.x * cw_tangent.x + backward_vec.y * cw_tangent.y;
    let walk_ccw = dot_ccw >= dot_cw;

    let envelope = cleared.envelope(radius);

    let mut last_engaged: Option<(Point, f64)> = None;

    for offset in 1..n {
        let idx = if walk_ccw {
            (start_idx + offset) % n
        } else {
            (start_idx + n - offset) % n
        };
        let pt = poly[idx];

        let normal_heading = get_polygon_heading_at(poly, pt);
        let normal = Point::new(normal_heading.cos(), normal_heading.sin());
        let probe = pt + normal * step_length;

        // Probes outside the tool-centre envelope cut zero material.
        let in_bounds = envelope.is_empty()
            || envelope.iter().any(|env| is_point_in_polygon(probe, env));
        let area = if in_bounds {
            cleared.cut_area(pt, probe, radius)
        } else {
            0.0
        };

        if area >= min_cut_area {
            let prev_idx = if walk_ccw {
                (idx + n - 1) % n
            } else {
                (idx + 1) % n
            };
            let prev_pt = poly[prev_idx];
            let travel_heading = (pt.y - prev_pt.y).atan2(pt.x - prev_pt.x);
            last_engaged = Some((pt, travel_heading));
        } else if let Some((last_pt, travel_heading)) = last_engaged {
            // We passed through an engaged region and now engagement
            // dropped — return the disengagement point with flipped
            // heading (the forward direction, opposite to travel).
            let flipped = travel_heading + std::f64::consts::PI;
            let normal_h = get_polygon_heading_at(poly, last_pt);
            let n_vec = Point::new(normal_h.cos(), normal_h.sin());
            let offset_pos = offset_inward(last_pt, n_vec, radius, advance);
            return Some(ToolPose {
                pos: offset_pos,
                heading: flipped,
            });
        }
    }

    // Entered engaged region but never left — return last engaged vertex.
    last_engaged.map(|(pos, _)| {
        let normal_h = get_polygon_heading_at(poly, pos);
        let n_vec = Point::new(normal_h.cos(), normal_h.sin());
        let offset_pos = offset_inward(pos, n_vec, radius, advance);
        ToolPose {
            pos: offset_pos,
            heading: 0.0,
        }
    })
}
