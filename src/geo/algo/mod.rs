//! Algo: Complex mathematical operations.
//!
//! This module provides advanced geometric algorithms including clipping,
//! curve fitting, 2D Minkowski sums, simplification, and smoothing.

pub mod analysis;
pub mod astar;
pub mod cleanup;
pub mod clipping;
pub mod cylindrical;
pub mod engagement;
pub mod fillet;
pub mod fitting;
pub mod helix;
pub mod hull;
pub mod interp;
pub mod intersect;
pub mod medial_axis;
pub mod minkowski2d;
pub mod nest2d;
pub mod offset;
pub mod ordering;
pub mod overcut;
pub mod planar;
pub mod polylabel;
pub mod project;
pub mod ramp;
pub mod rootfind;
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
pub use clipping::{
    clip_line_segment_with_polygons, clip_line_segment_with_polygons_2d,
    clip_line_segment_with_rect, clip_line_segment_with_rect_2d,
    subtract_polygons_from_line_segment,
    subtract_polygons_from_line_segment_2d,
};
pub use engagement::{
    circle_polygon_intersection_area, compute_engagement, disk_segment_area,
    Engagement,
};
pub use fillet::{
    append_end_fillets, create_fillet_polyline, trim_to_safe_fillet_span,
    try_fillet_one_end,
};
pub use fitting::{
    arc_between_two_points, are_points_collinear,
    convert_arc_to_beziers_from_array, convert_arcs_to_beziers, fit_arcs,
    fit_circle_to_3_points, fit_circle_to_points, fit_curves,
    fit_points_recursive, fit_points_with_primitives, flatten_to_points_3d,
    generate_linking_arc, get_polyline_arc_deviation,
    get_polyline_line_deviation, linearize_data, linearize_geometry,
    optimize_path_from_array, project_circle_center_to_bisector,
};
pub use helix::{generate_helix_3d, HelixDirection, HelixOptions};
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
    get_ray_line_intersection, get_ray_polygon_intersection,
};
pub use medial_axis::{MaBranch, MaNode, MedialAxis};
pub use minkowski2d::{
    calculate_input_scale, convolve_point_sequences, convolve_two_segments,
    get_inner_fit_polygon, get_no_fit_polygon,
    get_polygon_minkowski_sum_convex,
};
pub use offset::{
    compute_inset_region, concentric_offsets, find_deepest_cores,
    grow_geometry, grow_geometry_on_plane, offset_contour_group,
};
pub use ordering::order_nearest_neighbor;
pub use overcut::apply_overcut;
pub use polylabel::{find_largest_circle, polylabel};
pub use project::{
    is_planar_in_z, lift_points_to_xy_plane, project_point_to_xy,
    project_points_to_xy,
};
pub use ramp::{generate_ramp_3d, RampOptions, RampStyle};
pub use rootfind::{
    bisect, bisect_tracked, bracket_grid, illinois, illinois_tracked, secant,
    secant_tracked, RootStatus,
};
pub use simplify::{simplify_data, simplify_polyline, simplify_polyline_3d};
pub use smooth::{
    blend_tangent, build_smoothed_path, chaikin_corner_cut,
    compute_gaussian_kernel, smooth_circularly, smooth_path,
    smooth_polyline_3d, smooth_sub_segment,
};
pub use spiral::{generate_spiral_3d, SpiralOptions};
pub use topology::{
    build_hierarchy, close_all_contours, filter_to_external_contours,
    get_valid_contours_data, group_solids_and_holes, normalize_winding_orders,
    remove_inner_edges, reverse_contour, split_inner_and_outer_contours,
    split_into_components, split_into_contours, ContourHierarchy, ContourInfo,
};
pub use trochoid::{trochoid_along_3d, TrochoidOptions};
