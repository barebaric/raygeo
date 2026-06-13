//! Point: Point operations.
//!
//! This module provides basic point manipulation functions.

use crate::types::Point3D;

/// Computes the midpoint between two 3D points.
pub fn midpoint(a: Point3D, b: Point3D) -> Point3D {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, (a.2 + b.2) / 2.0)
}

/// Apply an affine transformation matrix to a 3D point.
/// Returns the transformed point `(x, y, z)`.
pub fn transform_point(
    matrix: &[[f64; 4]; 4],
    x: f64,
    y: f64,
    z: f64,
) -> Point3D {
    let px =
        matrix[0][0] * x + matrix[0][1] * y + matrix[0][2] * z + matrix[0][3];
    let py =
        matrix[1][0] * x + matrix[1][1] * y + matrix[1][2] * z + matrix[1][3];
    let pz =
        matrix[2][0] * x + matrix[2][1] * y + matrix[2][2] * z + matrix[2][3];
    (px, py, pz)
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
