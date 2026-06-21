//! Line: Line segment operations.
//!
//! This module provides functions for working with lines and line segments.

use std::f64::consts::PI;

use crate::types::{Point, Point3D, Polygon, Rect};

/// Computes the Euclidean length of a line segment.
pub fn get_line_segment_length(p1: Point, p2: Point) -> f64 {
    p1.distance(p2)
}

/// Checks if a point lies on a line segment using dot product projection
/// and cross-product collinearity test.
pub fn is_point_on_segment(pt: Point, p1: Point, p2: Point) -> bool {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let len_sq = Point::new(dx, dy).length_squared();

    // Collinearity test: cross product magnitude squared
    let cross = (pt - p1).perp_dot(Point::new(dx, dy));
    if cross.abs() > 1e-9 * len_sq.max(1.0) {
        return false;
    }

    let dot1 = (pt - p1).dot(Point::new(dx, dy));
    if dot1 < 0.0 {
        return false;
    }
    let dot2 = (pt - p2).dot(Point::new(-dx, -dy));
    if dot2 < 0.0 {
        return false;
    }
    true
}

/// Computes the intersection of two infinite lines.
/// Returns None if lines are parallel (denominator = 0).
pub fn get_line_line_intersection(
    p1: Point,
    p2: Point,
    p3: Point,
    p4: Point,
) -> Option<Point> {
    let x1 = p1.x;
    let y1 = p1.y;
    let x2 = p2.x;
    let y2 = p2.y;
    let x3 = p3.x;
    let y3 = p3.y;
    let x4 = p4.x;
    let y4 = p4.y;

    let denom =
        Point::new(x2 - x1, y2 - y1).perp_dot(Point::new(x4 - x3, y4 - y3));
    if denom == 0.0 {
        return None;
    }

    let ua = Point::new(x4 - x3, y4 - y3).perp_dot(p1 - p3) / denom;
    Some(p1 + (p2 - p1) * ua)
}

/// Computes the intersection of two line segments.
/// Returns None if segments don't intersect (even if their infinite lines do).
/// Uses parameter t for first segment [0,1] and u for second segment [0,1].
pub fn get_line_segment_intersection(
    p1: Point,
    p2: Point,
    p3: Point,
    p4: Point,
) -> Option<Point> {
    let x1 = p1.x;
    let y1 = p1.y;
    let x2 = p2.x;
    let y2 = p2.y;
    let x3 = p3.x;
    let y3 = p3.y;
    let x4 = p4.x;
    let y4 = p4.y;

    let den =
        Point::new(x1 - x2, y1 - y2).perp_dot(Point::new(x3 - x4, y3 - y4));
    if den.abs() < 1e-9 {
        return None;
    }

    let t_num =
        Point::new(x1 - x3, y1 - y3).perp_dot(Point::new(x3 - x4, y3 - y4));
    let u_num =
        -Point::new(x1 - x2, y1 - y2).perp_dot(Point::new(x1 - x3, y1 - y3));

    let t = t_num / den;
    let u = u_num / den;

    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(p1 + (p2 - p1) * t)
    } else {
        None
    }
}

/// Finds the closest point on an infinite line to a given point.
/// Uses projection to find the point, returns first endpoint if line is degenerate.
pub fn get_line_closest_point(p1: Point, p2: Point, x: f64, y: f64) -> Point {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let px = x - p1.x;
    let py = y - p1.y;

    let len_sq = Point::new(dx, dy).length_squared();
    if len_sq < 1e-12 {
        return p1;
    }

    let t = Point::new(px, py).dot(Point::new(dx, dy)) / len_sq;
    p1 + Point::new(dx, dy) * t
}

/// Finds the closest point on a line segment to a given point.
/// Returns (t_parameter, closest_point, distance_squared).
/// t is in range [0, 1] representing position along the segment.
pub fn get_line_segment_closest_point(
    p1: Point,
    p2: Point,
    x: f64,
    y: f64,
) -> (f64, Point, f64) {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let len_sq = Point::new(dx, dy).length_squared();

    let t = if len_sq < 1e-12 {
        0.0
    } else {
        (Point::new(x, y) - p1).dot(Point::new(dx, dy)) / len_sq
    };

    let t = t.clamp(0.0, 1.0);

    let closest = p1 + Point::new(dx, dy) * t;
    let dist_sq = Point::new(x, y).distance_squared(closest);

    (t, closest, dist_sq)
}

/// Perpendicular distance from a point to a line segment.
pub fn get_point_line_distance(
    point: Point,
    line_start: Point,
    line_end: Point,
) -> f64 {
    let line_vec =
        Point::new(line_end.x - line_start.x, line_end.y - line_start.y);
    let line_len = line_vec.length();
    if line_len < 1e-6 {
        return (point - line_start).length();
    }
    let line_unit = line_vec.normalize();
    let point_vec = point - line_start;
    let mut proj_len = point_vec.dot(line_unit);
    proj_len = proj_len.max(0.0).min(line_len);
    let closest = line_start + line_unit * proj_len;
    (point - closest).length()
}

/// Finds all intersection parameters along a line segment with multiple polygons.
/// Returns sorted list of t values [0, 1] where segment intersects any region.
pub fn get_line_segment_polygon_intersections(
    p1_2d: Point,
    p2_2d: Point,
    regions: &[Polygon],
) -> Vec<f64> {
    let mut cuts = Vec::new();
    get_line_segment_polygon_intersections_into(
        p1_2d, p2_2d, regions, &mut cuts,
    );
    cuts
}

/// Writes intersection parameters into a caller-provided buffer to reuse allocations.
/// The buffer is cleared and filled with sorted t values [0, 1].
pub fn get_line_segment_polygon_intersections_into(
    p1_2d: Point,
    p2_2d: Point,
    regions: &[Polygon],
    out: &mut Vec<f64>,
) {
    out.clear();
    out.push(0.0);
    out.push(1.0);

    for region in regions {
        for i in 0..region.len() {
            let p3 = region[i];
            let p4 = region[(i + 1) % region.len()];
            if let Some(intersection) =
                get_line_segment_intersection(p1_2d, p2_2d, p3, p4)
            {
                let ix = intersection.x;
                let iy = intersection.y;
                let seg_dx = p2_2d.x - p1_2d.x;
                let seg_dy = p2_2d.y - p1_2d.y;

                let t = if seg_dx.abs() > seg_dy.abs() {
                    if seg_dx != 0.0 {
                        (ix - p1_2d.x) / seg_dx
                    } else {
                        0.0
                    }
                } else if seg_dy != 0.0 {
                    (iy - p1_2d.y) / seg_dy
                } else {
                    0.0
                };
                let t_clamped = t.clamp(0.0, 1.0);
                if !out.contains(&t_clamped) {
                    out.push(t_clamped);
                }
            }
        }
    }

    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
}

/// Tests if a 2D point is inside an axis-aligned rectangle.
pub fn is_point_inside_rect(point: Point, rect: Rect) -> bool {
    let x = point.x;
    let y = point.y;
    x >= rect.min.x && x <= rect.max.x && y >= rect.min.y && y <= rect.max.y
}

/// Tests if rect_b is completely contained within rect_a.
pub fn does_rect_contain_rect(rect_a: Rect, rect_b: Rect) -> bool {
    rect_b.min.x >= rect_a.min.x
        && rect_b.min.y >= rect_a.min.y
        && rect_b.max.x <= rect_a.max.x
        && rect_b.max.y <= rect_a.max.y
}

/// Tests if two rectangles intersect.
pub fn does_rect_intersect_rect(rect_a: Rect, rect_b: Rect) -> bool {
    !(rect_a.max.x < rect_b.min.x
        || rect_a.min.x > rect_b.max.x
        || rect_a.max.y < rect_b.min.y
        || rect_a.min.y > rect_b.max.y)
}

/// Tests if a line segment intersects an axis-aligned rectangle.
/// Checks if either endpoint is inside the rect or if segment crosses any edge.
pub fn does_line_segment_intersect_rect(
    p1: Point,
    p2: Point,
    rect: Rect,
) -> bool {
    if p1.x >= rect.min.x
        && p1.x <= rect.max.x
        && p1.y >= rect.min.y
        && p1.y <= rect.max.y
    {
        return true;
    }
    if p2.x >= rect.min.x
        && p2.x <= rect.max.x
        && p2.y >= rect.min.y
        && p2.y <= rect.max.y
    {
        return true;
    }

    let intersections = [
        get_line_segment_intersection(
            p1,
            p2,
            Point::new(rect.min.x, rect.min.y),
            Point::new(rect.max.x, rect.min.y),
        ),
        get_line_segment_intersection(
            p1,
            p2,
            Point::new(rect.max.x, rect.min.y),
            Point::new(rect.max.x, rect.max.y),
        ),
        get_line_segment_intersection(
            p1,
            p2,
            Point::new(rect.max.x, rect.max.y),
            Point::new(rect.min.x, rect.max.y),
        ),
        get_line_segment_intersection(
            p1,
            p2,
            Point::new(rect.min.x, rect.max.y),
            Point::new(rect.min.x, rect.min.y),
        ),
    ];

    intersections.iter().any(|x| x.is_some())
}

/// Tests if a line segment intersects a circle by checking closest point distance.
pub fn does_line_segment_intersect_circle(
    p1: Point,
    p2: Point,
    center: Point,
    radius: f64,
) -> bool {
    let (_, _, dist_sq) =
        get_line_segment_closest_point(p1, p2, center.x, center.y);
    dist_sq <= radius * radius
}

/// Computes the interior angle at a vertex formed by three points.
/// Returns the angle in radians between 0 and PI.
pub fn get_angle_at_vertex(p0: Point, p1: Point, p2: Point) -> f64 {
    let v1x = p0.x - p1.x;
    let v1y = p0.y - p1.y;
    let v2x = p2.x - p1.x;
    let v2y = p2.y - p1.y;

    let mag_v1 = Point::new(v1x, v1y).length();
    let mag_v2 = Point::new(v2x, v2y).length();
    let mag_prod = mag_v1 * mag_v2;

    if mag_prod < 1e-9 {
        return PI;
    }

    let dot = Point::new(v1x, v1y).dot(Point::new(v2x, v2y));
    let cos_theta = (-1.0_f64).max(1.0_f64).min(dot / mag_prod);

    cos_theta.acos()
}

/// Return `true` when the line segment `(a, b)` crosses the interior of
/// `polygon` (an intersection strictly between the endpoints — touching
/// a vertex or grazing an edge at the endpoints is not a crossing).
pub fn does_line_cross_polygon(a: Point, b: Point, polygon: &Polygon) -> bool {
    let ts = get_line_segment_polygon_intersections(
        a,
        b,
        std::slice::from_ref(polygon),
    );
    ts.iter().any(|&t| t > 1e-12 && t < 1.0 - 1e-12)
}

/// Generate `n` linearly interpolated 3D points from `from` to `to` at
/// height `z`.  Returns `n` points spanning `t ∈ [1/n, 1]` — the start
/// point `from` is **not** included, the end point `to` *is* included.
///
/// When `n == 0` an empty vec is returned.
pub fn interpolated_segment_3d(
    from: Point,
    to: Point,
    z: f64,
    n: usize,
) -> Vec<Point3D> {
    let mut out = Vec::with_capacity(n);
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let x = from.x + (to.x - from.x) * t;
        let y = from.y + (to.y - from.y) * t;
        out.push(Point3D::new(x, y, z));
    }
    out
}
