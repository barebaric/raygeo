//! Point: Point operations.
//!
//! This module provides basic point manipulation functions.

use glam::DMat4;

use crate::geo::types::{Point, Point3D};

/// Computes the midpoint between two 3D points.
pub fn get_midpoint_3d(a: Point3D, b: Point3D) -> Point3D {
    (a + b) / 2.0
}

/// Apply an affine transformation matrix to a 3D point.
/// Returns the transformed point `(x, y, z)`.
pub fn transform_point_3d(matrix: DMat4, p: Point3D) -> Point3D {
    matrix.transform_point3(p)
}

/// Check if two points (as 3-element arrays) are equal within a tolerance.
pub fn are_points_equal_3d(
    p1: &[f64; 3],
    p2: &[f64; 3],
    tolerance: f64,
) -> bool {
    for i in 0..3 {
        if (p1[i] - p2[i]).abs() > tolerance {
            return false;
        }
    }
    true
}

/// Compute the get_circumcenter of three 3D points.
///
/// Returns the center of the unique circle passing through all three points.
/// Returns `None` if the points are collinear (degenerate).
pub fn get_circumcenter_3d(
    a: Point3D,
    b: Point3D,
    c: Point3D,
) -> Option<Point3D> {
    let ab = b - a;
    let ac = c - a;
    let ab2 = ab.length_squared();
    let ac2 = ac.length_squared();
    let ab_ac = ab.dot(ac);
    let denom = 2.0 * (ab2 * ac2 - ab_ac * ab_ac);
    if denom.abs() < 1e-24 {
        return None;
    }
    let alpha = (ab2 * ac2 - ab_ac * ac2) / denom;
    let beta = (ab2 * ac2 - ab2 * ab_ac) / denom;
    Some(a + ab * alpha + ac * beta)
}

/// Rotate a 2D point around the origin by the given angle (radians).
pub fn rotate_point(point: Point, angle: f64) -> Point {
    let c = angle.cos();
    let s = angle.sin();
    Point::new(c * point.x - s * point.y, s * point.x + c * point.y)
}

/// Compute the get_circumcenter and radius of three 2D points.
///
/// Returns `(center, radius)`. Returns a zero center and negative radius
/// if the points are collinear (degenerate).
pub fn get_circumcenter(a: Point, b: Point, c: Point) -> (Point, f64) {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-30 {
        return (Point::new(0.0, 0.0), -1.0);
    }
    let a2 = a.x * a.x + a.y * a.y;
    let b2 = b.x * b.x + b.y * b.y;
    let c2 = c.x * c.x + c.y * c.y;
    let ux = (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d;
    let uy = (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d;
    let center = Point::new(ux, uy);
    let r = ((center.x - a.x).powi(2) + (center.y - a.y).powi(2)).sqrt();
    (center, r)
}
