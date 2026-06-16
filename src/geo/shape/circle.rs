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
    let d_sq = dx * dx + dy * dy;
    let d = d_sq.sqrt();

    if d < 1e-9 || d > r1 + r2 || d < (r1 - r2).abs() {
        return vec![];
    }

    let a = (r1 * r1 - r2 * r2 + d_sq) / (2.0 * d);
    let h_sq = (r1 * r1 - a * a).max(0.0);
    let h = h_sq.sqrt();

    let x2 = c1.x + a * dx / d;
    let y2 = c1.y + a * dy / d;

    if h < 1e-9 {
        return vec![Point::new(x2, y2)];
    }

    let x3_1 = x2 + h * dy / d;
    let y3_1 = y2 - h * dx / d;
    let x3_2 = x2 - h * dy / d;
    let y3_2 = y2 + h * dx / d;

    vec![Point::new(x3_1, y3_1), Point::new(x3_2, y3_2)]
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
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < 1e-9 {
        return None;
    }

    let scale = radius / dist;
    Some(Point::new(center.x + dx * scale, center.y + dy * scale))
}

/// Tests if a circle is completely contained within an axis-aligned rectangle.
pub fn is_circle_inside_rect(center: Point, radius: f64, rect: Rect) -> bool {
    let cx = center.x;
    let cy = center.y;
    let Rect(rx1, ry1, rx2, ry2) = rect;
    (cx - radius) >= rx1
        && (cy - radius) >= ry1
        && (cx + radius) <= rx2
        && (cy + radius) <= ry2
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
    let Rect(rx1, ry1, rx2, ry2) = rect;

    // Fully contained circles don't intersect
    if is_circle_inside_rect(center, radius, rect) {
        return false;
    }

    // Check if circle's closest point to rect is within radius
    let closest_x = rx1.max(cx.min(rx2));
    let closest_y = ry1.max(cy.min(ry2));
    let dist_sq_closest = (closest_x - cx).powi(2) + (closest_y - cy).powi(2);
    if dist_sq_closest > radius * radius {
        return false;
    }

    // Check if circle's farthest point from rect center is outside radius
    let dx_far = (rx1 - cx).abs().max((rx2 - cx).abs());
    let dy_far = (ry1 - cy).abs().max((ry2 - cy).abs());
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

    let a = dx * dx + dy * dy;
    if a < 1e-20 {
        return vec![];
    }

    let b = 2.0 * (fx * dx + fy * dy);
    let c = fx * fx + fy * fy - radius * radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return vec![];
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    let mut results = Vec::new();

    if sqrt_disc < 1e-10 {
        if (0.0..=1.0).contains(&t1) {
            results.push(Point::new(p1.x + t1 * dx, p1.y + t1 * dy));
        }
    } else {
        if (0.0..=1.0).contains(&t1) {
            results.push(Point::new(p1.x + t1 * dx, p1.y + t1 * dy));
        }
        if (0.0..=1.0).contains(&t2) {
            results.push(Point::new(p1.x + t2 * dx, p1.y + t2 * dy));
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
