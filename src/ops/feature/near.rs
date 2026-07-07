use crate::geo::shape::polygon::{
    get_polygons_closest_point, get_polygons_group_difference,
    get_polygons_group_intersection, get_signed_boundary_distance,
    offset_polygon, JoinStyle,
};
use crate::types::{Point, Polygon};

/// Find a plunge point near the given position within the cleared area.
///
/// Returns the valid tool placement point *closest* to `near` such that
/// the full tool disk (of `tool_radius`) fits inside the union of
/// `cleared_polygons`, fits inside `boundary`, and does not overlap any
/// `island`, all within `search_radius` of `near`.  Returns `None` if
/// no such point exists.
///
/// The valid-centre region is constructed geometrically:
/// 1. Each cleared polygon and the boundary are eroded by `tool_radius`
///    (so a tool centred anywhere inside the result fits inside the
///    original region).
/// 2. The eroded cleared polygons are intersected with the eroded
///    boundary, then each island is dilated by `tool_radius` and
///    subtracted — yielding the set of all legal tool centres.
/// 3. [`get_polygons_closest_point`] finds the nearest point of that
///    region to `near` (when `near` is outside it).
///
/// When `near` itself is valid it is returned immediately.
pub fn find_plunge_point(
    near: Point,
    cleared_polygons: &[Polygon],
    boundary: &Polygon,
    islands: &[Polygon],
    tool_radius: f64,
    search_radius: f64,
) -> Option<Point> {
    if cleared_polygons.is_empty() {
        return None;
    }

    // Erode each cleared polygon by tool_radius (negative offset).
    let mut valid: Vec<Polygon> = Vec::new();
    for cleared in cleared_polygons {
        for eroded in offset_polygon(cleared, -tool_radius, JoinStyle::Miter) {
            if eroded.len() >= 3 {
                valid.push(eroded);
            }
        }
    }
    if valid.is_empty() {
        return None;
    }

    // Intersect with the boundary eroded by tool_radius.
    let eroded_boundary =
        offset_polygon(boundary, -tool_radius, JoinStyle::Miter);
    if !eroded_boundary.is_empty() {
        valid = get_polygons_group_intersection(&valid, &eroded_boundary);
    }
    if valid.is_empty() {
        return None;
    }

    // Subtract each island dilated by tool_radius.
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

    // If `near` lies inside the valid region, return it directly.
    // `get_signed_boundary_distance` is negative only when the point is
    // inside a CCW outer polygon and not inside any CW hole (island).
    if get_signed_boundary_distance(near, &valid) < 0.0 {
        return Some(near);
    }

    // Otherwise find the closest point on the valid region's boundary.
    let (.., closest, _) = get_polygons_closest_point(&valid, near)?;
    if closest.distance(near) > search_radius {
        return None;
    }
    Some(closest)
}
