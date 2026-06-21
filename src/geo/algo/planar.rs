//! Planar (XY-plane) algorithm boundary.
//!
//! All functions re-exported here are **strictly 2D** — they operate only on
//! the XY plane and do not model or carry Z coordinates.  They accept
//! [`Point`] / [`Polygon`] (2D) types.
//!
//! 3D callers must use the projection helpers (also re-exported here) to
//! explicitly project data to XY before calling planar routines and lift
//! results back afterward.
//!
//! [`Point`]: crate::types::Point
//! [`Polygon`]: crate::types::Polygon

// ── Boolean operations (Clipper2) ─────────────────────────────────────

pub use crate::geo::shape::polygon::{
    get_polygons_difference, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_intersection,
    get_polygons_union, offset_polygon,
};

// ── Clipping (2D-only cores) ─────────────────────────────────────────

pub use crate::geo::algo::clipping::{
    clip_line_segment_with_polygons_2d, clip_line_segment_with_rect_2d,
    subtract_polygons_from_line_segment_2d,
};

// ── Minkowski sums (2D) ──────────────────────────────────────────────

pub use crate::geo::algo::minkowski2d::{
    convolve_point_sequences, convolve_two_segments, get_inner_fit_polygon,
    get_no_fit_polygon, get_polygon_minkowski_sum_convex,
};

// ── Projection helpers (3D ↔ 2D boundary crossing) ──────────────────

pub use crate::geo::algo::project::{
    is_planar_in_z, lift_points_to_xy_plane, project_point_to_xy,
    project_points_to_xy,
};
