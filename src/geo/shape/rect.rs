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
        let r = geo.rect();
        min_x = min_x.min(r.min.x);
        min_y = min_y.min(r.min.y);
        max_x = max_x.max(r.max.x);
        max_y = max_y.max(r.max.y);
    }

    if min_x.is_infinite() {
        return Rect::default();
    }
    Rect::new(min_x, min_y, max_x, max_y)
}

pub fn do_rects_intersect(bbox1: Rect, bbox2: Rect) -> bool {
    !(bbox1.max.x < bbox2.min.x
        || bbox1.min.x > bbox2.max.x
        || bbox1.max.y < bbox2.min.y
        || bbox1.min.y > bbox2.max.y)
}
