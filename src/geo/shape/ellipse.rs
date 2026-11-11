//! Ellipse: parametric ellipse queries.
//!
//! The ellipse is defined by its center, the two semi-axis lengths
//! and a rotation.  Point queries rotate into the ellipse's local
//! frame internally.

use std::f64::consts::PI;

use crate::geo::types::Point;

/// Point on the ellipse at parameter `t`:
/// `center + R(rotation) * (rx * cos(t), ry * sin(t))`.
pub fn get_ellipse_point_at(
    center: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    t: f64,
) -> Point {
    let (cos_a, sin_a) = (rotation.cos(), rotation.sin());
    let x = rx * t.cos();
    let y = ry * t.sin();
    Point::new(
        center.x + x * cos_a - y * sin_a,
        center.y + x * sin_a + y * cos_a,
    )
}

/// Samples the ellipse uniformly in parameter over a full turn,
/// including both the start (t = 0) and the closing point
/// (t = 2*pi).
pub fn get_ellipse_samples(
    center: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    segments: usize,
) -> Vec<Point> {
    if segments == 0 {
        return Vec::new();
    }
    (0..=segments)
        .map(|i| {
            let t = 2.0 * PI * i as f64 / segments as f64;
            get_ellipse_point_at(center, rx, ry, rotation, t)
        })
        .collect()
}

/// Rotates `point - center` into the ellipse's local frame.
fn to_local(point: Point, center: Point, rotation: f64) -> Point {
    let (cos_a, sin_a) = ((-rotation).cos(), (-rotation).sin());
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(dx * cos_a - dy * sin_a, dx * sin_a + dy * cos_a)
}

/// Checks whether a point lies on the ellipse's circumference within
/// `tolerance` (interpreted in world units along the radial
/// direction, scaled by the smaller semi-axis).
pub fn is_point_on_ellipse(
    point: Point,
    center: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
    tolerance: f64,
) -> bool {
    if rx < 1e-9 || ry < 1e-9 {
        return false;
    }
    let local = to_local(point, center, rotation);
    let dist = ((local.x / rx).powi(2) + (local.y / ry).powi(2)).sqrt();
    (dist - 1.0).abs() < tolerance / rx.min(ry)
}

/// Checks whether a point lies inside or on the ellipse.
pub fn is_point_inside_ellipse(
    point: Point,
    center: Point,
    rx: f64,
    ry: f64,
    rotation: f64,
) -> bool {
    if rx <= 1e-9 || ry <= 1e-9 {
        return false;
    }
    let local = to_local(point, center, rotation);
    (local.x / rx).powi(2) + (local.y / ry).powi(2) <= 1.0
}

/// Area enclosed by the ellipse.
pub fn get_ellipse_area(rx: f64, ry: f64) -> f64 {
    PI * rx * ry
}
