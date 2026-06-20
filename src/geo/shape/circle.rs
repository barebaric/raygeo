//! Circle: Circle geometry operations.
//!
//! This module provides functions for working with circles including:
//! - Circle-circle intersection
//! - Point projection onto circle
//! - Circle-rectangle containment and intersection
//! - Line segment intersection with circles

use crate::geo::shape::line::get_line_segment_closest_point;
use crate::types::{Point, Rect};

/// Computes the intersection points between two circles.
/// Uses the geometry of intersecting circles to find 0, 1, or 2 points.
pub fn get_circle_circle_intersections(
    c1: Point,
    r1: f64,
    c2: Point,
    r2: f64,
) -> Vec<Point> {
    let dx = c2.x - c1.x;
    let dy = c2.y - c1.y;
    let d_sq = Point::new(dx, dy).length_squared();
    let d = d_sq.sqrt();

    if d < 1e-9 || d > r1 + r2 || d < (r1 - r2).abs() {
        return vec![];
    }

    let a = (r1 * r1 - r2 * r2 + d_sq) / (2.0 * d);
    let h_sq = (r1 * r1 - a * a).max(0.0);
    let h = h_sq.sqrt();

    let mid = c1 + Point::new(dx, dy) * (a / d);

    if h < 1e-9 {
        return vec![mid];
    }

    let offset = Point::new(h * dy / d, -h * dx / d);

    vec![mid + offset, mid - offset]
}

/// Projects a point onto a circle's circumference.
/// Returns None if the point is at the circle's center (direction undefined).
pub fn project_point_onto_circle(
    point: Point,
    center: Point,
    radius: f64,
) -> Option<Point> {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let dist = Point::new(dx, dy).length();

    if dist < 1e-9 {
        return None;
    }

    let scale = radius / dist;
    Some(center + Point::new(dx, dy) * scale)
}

/// Tests if a circle is completely contained within an axis-aligned rectangle.
pub fn is_circle_inside_rect(center: Point, radius: f64, rect: Rect) -> bool {
    let cx = center.x;
    let cy = center.y;
    (cx - radius) >= rect.min.x
        && (cy - radius) >= rect.min.y
        && (cx + radius) <= rect.max.x
        && (cy + radius) <= rect.max.y
}

/// Tests if a circle intersects an axis-aligned rectangle.
/// Uses multiple tests: containment check, closest point check, farthest point check.
pub fn does_circle_intersect_rect(
    center: Point,
    radius: f64,
    rect: Rect,
) -> bool {
    let cx = center.x;
    let cy = center.y;
    // Fully contained circles don't intersect
    if is_circle_inside_rect(center, radius, rect) {
        return false;
    }

    // Check if circle's closest point to rect is within radius
    let closest_x = rect.min.x.max(cx.min(rect.max.x));
    let closest_y = rect.min.y.max(cy.min(rect.max.y));
    let dist_sq_closest =
        Point::new(closest_x, closest_y).distance_squared(center);
    if dist_sq_closest > radius * radius {
        return false;
    }

    // Check if circle's farthest point from rect center is outside radius
    let dx_far = (rect.min.x - cx).abs().max((rect.max.x - cx).abs());
    let dy_far = (rect.min.y - cy).abs().max((rect.max.y - cy).abs());
    let dist_sq_farthest = dx_far * dx_far + dy_far * dy_far;
    if dist_sq_farthest < radius * radius {
        return false;
    }

    true
}

/// Computes intersection points between a line segment and a circle.
///
/// Returns 0, 1, or 2 intersection points where the segment from `p1`
/// to `p2` crosses the circle defined by `center` and `radius`.
/// Only intersections with t in [0, 1] along the segment are returned.
pub fn get_line_circle_intersections(
    p1: Point,
    p2: Point,
    center: Point,
    radius: f64,
) -> Vec<Point> {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let fx = p1.x - center.x;
    let fy = p1.y - center.y;

    let a = Point::new(dx, dy).length_squared();
    if a < 1e-20 {
        return vec![];
    }

    let b = 2.0 * Point::new(fx, fy).dot(Point::new(dx, dy));
    let c = Point::new(fx, fy).length_squared() - radius * radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return vec![];
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    let mut results = Vec::new();
    let dir = Point::new(dx, dy);

    if sqrt_disc < 1e-10 {
        if (0.0..=1.0).contains(&t1) {
            results.push(p1 + dir * t1);
        }
    } else {
        if (0.0..=1.0).contains(&t1) {
            results.push(p1 + dir * t1);
        }
        if (0.0..=1.0).contains(&t2) {
            results.push(p1 + dir * t2);
        }
    }

    results
}

/// Tests if a line segment intersects a circle by checking closest point distance.
pub fn line_segment_intersects_circle(
    p1: Point,
    p2: Point,
    center: Point,
    radius: f64,
) -> bool {
    let (_, _, dist_sq) =
        get_line_segment_closest_point(p1, p2, center.x, center.y);
    dist_sq <= radius * radius
}
