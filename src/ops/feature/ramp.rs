/// Find the longest straight carrier segment suitable for ramp entry.
///
/// A "ramp carrier" is a straight line segment within the valid
/// tool-centre region (the boundary eroded by `tool_radius`, minus
/// each island dilated by `tool_radius`) that is long enough for a
/// ramp descent of one pass (`Δz = tool_radius`) at the given maximum
/// ramp angle.
///
/// # Algorithm
///
/// Sweeps axis-aligned lines across the valid tool-centre region at
/// evenly spaced perpendicular offsets, clipping each against the
/// region, and returns the longest sub-segment that is at least `L_min`
/// long.
///
/// 1. Erode the boundary by `tool_radius` (Miter join). If the eroded
///    region is empty → `None`.
/// 2. Dilate each island by `tool_radius` and subtract from the eroded
///    region using [`get_polygons_group_difference`]. If empty → `None`.
/// 3. Compute the axis-aligned bounding box of the valid region.
/// 4. Sweep horizontal and vertical lines across the full perpendicular
///    extent of the region (sampled at intervals of `2 × tool_radius`,
///    clamped to 1…32 samples).
/// 5. Clip each sweep line with [`clip_line_segment_with_polygons_2d`]
///    and keep sub-segments whose length ≥ `L_min`.
/// 6. Return the longest qualifying carrier, oriented so the start has
///    the smaller coordinate on the dominant axis.
use crate::geo::algo::clipping::clip_line_segment_with_polygons_2d;
use crate::geo::shape::polygon::{
    get_polygon_group_bounds, get_polygons_group_difference, offset_polygon,
    JoinStyle,
};
use crate::geo::types::{Point, Polygon, Rect};

/// Minimum horizontal ramp length for a single-pass descent.
fn min_ramp_length(tool_radius: f64, max_ramp_angle_deg: f64) -> f64 {
    let dz = tool_radius;
    let angle_rad = max_ramp_angle_deg.to_radians();
    let horizontal = dz / angle_rad.tan();
    tool_radius.max(horizontal)
}

/// Sweep axis-aligned lines across the region and return the longest
/// sub-segment that is at least `l_min` long.
fn sweep_axis(
    bounds: &Rect,
    horizontal: bool,
    valid: &[Polygon],
    l_min: f64,
    tool_radius: f64,
) -> Option<(Point, Point)> {
    let (perp_min, sweep_min, sweep_max, perp_extent) = if horizontal {
        (
            bounds.min.y,
            bounds.min.x,
            bounds.max.x,
            bounds.max.y - bounds.min.y,
        )
    } else {
        (
            bounds.min.x,
            bounds.min.y,
            bounds.max.y,
            bounds.max.x - bounds.min.x,
        )
    };

    let n_samples =
        ((perp_extent / (tool_radius * 2.0)).ceil() as usize).clamp(1, 32);

    let mut best: Option<(f64, Point, Point)> = None;

    for i in 0..n_samples {
        let offset =
            perp_min + (i as f64 + 0.5) * perp_extent / n_samples as f64;
        let (sweep_start, sweep_end) = if horizontal {
            (Point::new(sweep_min, offset), Point::new(sweep_max, offset))
        } else {
            (Point::new(offset, sweep_min), Point::new(offset, sweep_max))
        };

        let sub_segments =
            clip_line_segment_with_polygons_2d(sweep_start, sweep_end, valid);
        for (a, b) in sub_segments {
            let len = a.distance(b);
            if len >= l_min {
                match best {
                    Some((best_len, _, _)) if len > best_len => {
                        best = Some((len, a, b));
                    }
                    None => {
                        best = Some((len, a, b));
                    }
                    _ => {}
                }
            }
        }
    }

    best.map(|(_, a, b)| (a, b))
}

/// Find the longest straight carrier segment suitable for ramp entry.
///
/// Returns `Some((start, end))` where both points are valid tool
/// centres inside the region and the segment length is at least
/// `L_min`.  Returns `None` if no qualifying carrier exists.
pub fn find_ramp_carrier(
    boundary: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    max_ramp_angle_deg: f64,
) -> Option<(Point, Point)> {
    // 1. Erode boundary by tool_radius.
    let mut valid = offset_polygon(boundary, -tool_radius, JoinStyle::Miter);
    if valid.is_empty() {
        return None;
    }

    // 2. Subtract dilated islands.
    for island in islands {
        let dilated = offset_polygon(island, tool_radius, JoinStyle::Miter);
        if dilated.is_empty() {
            continue;
        }
        valid = get_polygons_group_difference(&valid, &dilated);
        if valid.is_empty() {
            return None;
        }
    }

    let l_min = min_ramp_length(tool_radius, max_ramp_angle_deg);

    // 3. AABB of the valid region.
    let bounds = get_polygon_group_bounds(&valid);
    let w = bounds.max.x - bounds.min.x;
    let h = bounds.max.y - bounds.min.y;

    // 4. Sweep both axes.
    let x_carrier = sweep_axis(&bounds, true, &valid, l_min, tool_radius);
    let y_carrier = sweep_axis(&bounds, false, &valid, l_min, tool_radius);

    // 5. Pick the best carrier (ties broken by preferring the longer
    //    AABB axis: x if w >= h).
    let primary_is_x = w >= h;
    let result = match (x_carrier, y_carrier) {
        (Some((a1, b1)), Some((a2, b2))) => {
            let l1 = a1.distance(b1);
            let l2 = a2.distance(b2);
            if l1 > l2 || (l1 == l2 && primary_is_x) {
                (a1, b1)
            } else {
                (a2, b2)
            }
        }
        (Some(pair), None) => pair,
        (None, Some(pair)) => pair,
        (None, None) => return None,
    };

    // 6. Orient so start has the smaller coordinate on the dominant axis.
    let (start, end) = result;
    let oriented = if primary_is_x {
        if start.x <= end.x {
            (start, end)
        } else {
            (end, start)
        }
    } else if start.y <= end.y {
        (start, end)
    } else {
        (end, start)
    };
    Some(oriented)
}
