use crate::types::Rect;

pub fn do_rects_intersect(bbox1: Rect, bbox2: Rect) -> bool {
    let (ax1, ay1, ax2, ay2) = bbox1;
    let (bx1, by1, bx2, by2) = bbox2;

    !(ax2 < bx1 || ax1 > bx2 || ay2 < by1 || ay1 > by2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_do_rects_intersect() {
        let bbox1: Rect = (0.0, 0.0, 5.0, 5.0);
        let bbox2: Rect = (3.0, 3.0, 8.0, 8.0);
        assert!(do_rects_intersect(bbox1, bbox2));

        let bbox3: Rect = (10.0, 10.0, 15.0, 15.0);
        assert!(!do_rects_intersect(bbox1, bbox3));
    }
}
