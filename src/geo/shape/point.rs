//! Point: Point operations.
//!
//! This module provides basic point manipulation functions.

use glam::DMat4;

use crate::types::Point3D;

/// Computes the midpoint between two 3D points.
pub fn midpoint(a: Point3D, b: Point3D) -> Point3D {
    (a + b) / 2.0
}

/// Apply an affine transformation matrix to a 3D point.
/// Returns the transformed point `(x, y, z)`.
pub fn transform_point(matrix: DMat4, p: Point3D) -> Point3D {
    matrix.transform_point3(p)
}

/// Check if two points (as 3-element arrays) are equal within a tolerance.
pub fn are_points_equal(p1: &[f64; 3], p2: &[f64; 3], tolerance: f64) -> bool {
    for i in 0..3 {
        if (p1[i] - p2[i]).abs() > tolerance {
            return false;
        }
    }
    true
}

/// Compute the circumcenter of three 3D points.
///
/// Returns the center of the unique circle passing through all three points.
/// Returns `None` if the points are collinear (degenerate).
pub fn circumcenter(a: Point3D, b: Point3D, c: Point3D) -> Option<Point3D> {
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
