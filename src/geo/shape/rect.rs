use crate::geo::geometry::Geometry;
use crate::types::Rect;

/// Compute the union bounding box of multiple geometries.
/// Returns (0, 0, 0, 0) if the list is empty.
pub fn get_combined_rect(geometries: &[Geometry]) -> Rect {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for geo in geometries {
        let Rect(gx0, gy0, gx1, gy1) = geo.rect();
        min_x = min_x.min(gx0);
        min_y = min_y.min(gy0);
        max_x = max_x.max(gx1);
        max_y = max_y.max(gy1);
    }

    if min_x.is_infinite() {
        return Rect(0.0, 0.0, 0.0, 0.0);
    }
    Rect(min_x, min_y, max_x, max_y)
}

pub fn do_rects_intersect(bbox1: Rect, bbox2: Rect) -> bool {
    let Rect(ax1, ay1, ax2, ay2) = bbox1;
    let Rect(bx1, by1, bx2, by2) = bbox2;

    !(ax2 < bx1 || ax1 > bx2 || ay2 < by1 || ay1 > by2)
}
