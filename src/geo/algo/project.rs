//! Projection helpers for 2D/3D boundary crossing.
//!
//! Functions in this module make 3D→2D and 2D→3D transitions explicit,
//! so callers of planar-only algorithms must intentionally project before
//! passing data to Clipper2, nesting, or other XY-plane routines.
//!
//! # Example
//!
//! ```rust
//! use raygeo::geo::algo::project::{
//!     project_points_to_xy, lift_points_to_xy_plane,
//! };
//! use raygeo::geo::types::Point3D;
//!
//! let pts_3d = vec![Point3D::new(1.0, 2.0, 5.0), Point3D::new(3.0, 4.0, 5.0)];
//! let pts_2d = project_points_to_xy(&pts_3d);
//! // feed pts_2d to a planar-only Clipper2 function …
//! let result_3d = lift_points_to_xy_plane(&pts_2d, 5.0);
//! ```

use crate::geo::types::{Point, Point3D};

/// Project a 3D point to the XY plane, dropping Z.
pub fn project_point_to_xy(p: Point3D) -> Point {
    Point::new(p.x, p.y)
}

/// Project a slice of 3D points to the XY plane, dropping Z.
pub fn project_points_to_xy(points: &[Point3D]) -> Vec<Point> {
    points.iter().map(|p| Point::new(p.x, p.y)).collect()
}

/// Lift 2D points to the XY plane at a given Z height.
pub fn lift_points_to_xy_plane(points: &[Point], z: f64) -> Vec<Point3D> {
    points.iter().map(|p| Point3D::new(p.x, p.y, z)).collect()
}

/// Check whether all points share the same Z (within tolerance).
/// Returns `Some(z)` if planar in Z, `None` otherwise.
pub fn is_planar_in_z(points: &[Point3D], tol: f64) -> Option<f64> {
    let z0 = points.first()?.z;
    if points.iter().all(|p| (p.z - z0).abs() <= tol) {
        Some(z0)
    } else {
        None
    }
}
