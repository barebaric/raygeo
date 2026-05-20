//! Point: Point operations.
//!
//! This module provides basic point manipulation functions.

use crate::types::Point3D;

/// Computes the midpoint between two 3D points.
pub fn midpoint(a: Point3D, b: Point3D) -> Point3D {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, (a.2 + b.2) / 2.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midpoint() {
        let a: Point3D = (0.0, 0.0, 0.0);
        let b: Point3D = (2.0, 4.0, 6.0);
        let result = midpoint(a, b);
        assert_eq!(result, (1.0, 2.0, 3.0));
    }

    #[test]
    fn test_are_points_equal_exact() {
        let p1 = [1.0, 2.0, 3.0];
        let p2 = [1.0, 2.0, 3.0];
        assert!(are_points_equal(&p1, &p2, 1e-6));
    }

    #[test]
    fn test_are_points_equal_within_tolerance() {
        let p1 = [1.0, 2.0, 3.0];
        let p2 = [1.000009, 2.000009, 3.000009];
        assert!(are_points_equal(&p1, &p2, 1e-5));
    }

    #[test]
    fn test_are_points_equal_outside_tolerance() {
        let p1 = [1.0, 2.0, 3.0];
        let p2 = [1.1, 2.0, 3.0];
        assert!(!are_points_equal(&p1, &p2, 1e-6));
    }

    #[test]
    fn test_are_points_equal_partial_difference() {
        let p1 = [1.0, 2.0, 3.0];
        let p2 = [1.0, 2.0, 3.001];
        assert!(!are_points_equal(&p1, &p2, 1e-6));
    }
}
