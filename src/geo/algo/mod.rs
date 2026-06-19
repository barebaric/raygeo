//! Algo: Complex mathematical operations.
//!
//! This module provides advanced geometric algorithms including clipping,
//! curve fitting, 2D Minkowski sums, simplification, and smoothing.

pub mod analysis;
pub mod cleanup;
pub mod cleared_area;
pub mod clipping;
pub mod cylindrical;
pub mod fitting;
pub mod helix;
pub mod hsm;
pub mod hull;
pub mod interp;
pub mod intersect;
pub mod minkowski2d;
pub mod nest2d;
pub mod offset;
pub mod overcut;
pub mod pde_mesh;
pub mod pde_spiral;
pub mod planar;
pub mod polylabel;
pub mod project;
pub mod ramp;
pub mod simplify;
pub mod smooth;
pub mod spatial_grid2d;
pub mod spiral;
pub mod topology;
pub mod trochoid;

pub use analysis::{
    does_enclose, get_area_from_array, get_outward_normal_at_from_array,
    get_path_winding_order_from_array, get_point_at_from_array,
    get_subpath_area_from_array, get_subpath_vertices_from_array,
    get_tangent_at_from_array, is_closed, partial_segment, remove_duplicates,
    segment_length,
};
pub use cleanup::{
    are_segments_equal, close_geometry_gaps_from_array, get_segment_key,
    remove_duplicate_segments,
};
pub use cleared_area::ClearedArea;
pub use clipping::{
    clip_line_segment_with_polygons, clip_line_segment_with_polygons_2d,
    clip_line_segment_with_rect, clip_line_segment_with_rect_2d,
    subtract_polygons_from_line_segment,
    subtract_polygons_from_line_segment_2d,
};
pub use fitting::{
    are_points_collinear, convert_arc_to_beziers_from_array,
    convert_arcs_to_beziers, fit_arcs, fit_circle_to_3_points,
    fit_circle_to_points, fit_curves, fit_points_recursive,
    fit_points_with_primitives, flatten_to_points, get_polyline_arc_deviation,
    get_polyline_line_deviation, linearize_data, linearize_geometry,
    optimize_path_from_array, project_circle_center_to_bisector,
};
pub use helix::{generate_helix, HelixDirection, HelixOptions};
pub use hsm::{
    adaptive_entry, adaptive_wavefronts, AdaptiveEntryOptions,
    AdaptiveEntryResult, AdaptiveWavefrontOptions, AdaptiveWavefrontResult,
};
pub use hull::{
    find_external_contours, get_concave_hull, get_enclosing_hull,
    get_hulls_from_image,
};
pub use interp::{
    barycentric_interpolate, barycentric_weights, compute_segment_delta,
    compute_t_range, project_t_along_segment, slice_scanline_data,
    solve_quadratic, SegmentDelta,
};
pub use intersect::{
    check_intersection_from_array, check_self_intersection_from_array,
    ray_line_intersection,
};
pub use minkowski2d::{
    calculate_input_scale, convolve_point_sequences, convolve_two_segments,
    get_inner_fit_polygon, get_no_fit_polygon,
    get_polygon_minkowski_sum_convex,
};
pub use offset::{
    concentric_offsets, find_deepest_cores, grow_geometry,
    grow_geometry_on_plane, offset_contour_group,
};
pub use overcut::apply_overcut;
pub use pde_spiral::trace_spiral;
pub use polylabel::{find_largest_circle, polylabel};
pub use project::{
    is_planar_in_z, lift_points_to_xy_plane, project_point_to_xy,
    project_points_to_xy,
};
pub use ramp::{generate_ramp, RampOptions, RampStyle};
pub use simplify::{simplify_data, simplify_polyline};
pub use smooth::{
    compute_gaussian_kernel, resample_polyline, smooth_circularly,
    smooth_polyline, smooth_sub_segment,
};
pub use spiral::{generate_spiral, SpiralOptions};
pub use topology::{
    build_hierarchy, close_all_contours, filter_to_external_contours,
    get_valid_contours_data, group_solids_and_holes, normalize_winding_orders,
    remove_inner_edges, reverse_contour, split_inner_and_outer_contours,
    split_into_components, split_into_contours, ContourHierarchy, ContourInfo,
};
pub use trochoid::{trochoid_along, TrochoidOptions};
