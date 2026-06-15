//! Projection helpers for 2D/3D boundary crossing.
//!
//! Functions in this module make 3D→2D and 2D→3D transitions explicit,
//! so callers of planar-only algorithms must intentionally project before
//! passing data to Clipper2, nesting, or other XY-plane routines.
//!
//! # Example
//!
//! ```rust
//! use raygeo::types::{Point3D, project_points_to_xy, lift_points_to_xy_plane};
//!
//! let pts_3d = vec![Point3D(1.0, 2.0, 5.0), Point3D(3.0, 4.0, 5.0)];
//! let pts_2d = project_points_to_xy(&pts_3d);
//! // feed pts_2d to a planar-only Clipper2 function …
//! let result_3d = lift_points_to_xy_plane(&pts_2d, 5.0);
//! ```

/// Re-exported from [`crate::types::project_point_to_xy`].
pub use crate::types::project_point_to_xy;

/// Re-exported from [`crate::types::project_points_to_xy`].
pub use crate::types::project_points_to_xy;

/// Re-exported from [`crate::types::lift_points_to_xy_plane`].
pub use crate::types::lift_points_to_xy_plane;

/// Re-exported from [`crate::types::is_planar_in_z`].
pub use crate::types::is_planar_in_z;
