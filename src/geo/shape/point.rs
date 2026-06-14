//! Point: Point operations.
//!
//! This module provides basic point manipulation functions.

use glam::{DMat4, DVec3};

use crate::types::Point3D;

/// Computes the midpoint between two 3D points.
pub fn midpoint(a: Point3D, b: Point3D) -> Point3D {
    (a + b) / 2.0
}

/// Apply an affine transformation matrix to a 3D point.
/// Returns the transformed point `(x, y, z)`.
pub fn transform_point(matrix: DMat4, p: Point3D) -> Point3D {
    let r = matrix.transform_point3(DVec3::new(p.x(), p.y(), p.z()));
    Point3D(r.x, r.y, r.z)
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
