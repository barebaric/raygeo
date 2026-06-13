//! Geo: Geometry, shapes, and algorithms.
//!
//! This module provides the core geometric types and operations including
//! the `Geometry` struct, shape primitives, and algorithms.

pub mod algo;
pub mod geometry;
pub mod math;
pub mod query;
pub mod shape;

pub use algo::{
    apply_overcut, are_points_collinear, are_segments_equal, build_hierarchy,
    calculate_input_scale, check_intersection_from_array,
    check_self_intersection_from_array, clip_line_segment_with_polygons,
    clip_line_segment_with_rect, close_all_contours,
    close_geometry_gaps_from_array, compute_gaussian_kernel,
    compute_segment_delta, compute_t_range, convert_arc_to_beziers_from_array,
    convert_arcs_to_beziers, convolve_point_sequences, convolve_two_segments,
    create_arc_cmd, create_line_cmd, does_enclose, filter_to_external_contours,
    find_external_contours, fit_arcs, fit_circle_to_3_points,
    fit_circle_to_points, fit_curves, fit_points_recursive,
    fit_points_with_primitives, flatten_to_points, get_area_from_array,
    get_concave_hull, get_enclosing_hull, get_hulls_from_image,
    get_inner_fit_polygon, get_no_fit_polygon,
    get_outward_normal_at_from_array, get_path_winding_order_from_array,
    get_point_at_from_array, get_polygon_minkowski_sum_convex,
    get_polyline_arc_deviation, get_polyline_line_deviation, get_segment_key,
    get_subpath_area_from_array, get_subpath_vertices_from_array,
    get_tangent_at_from_array, get_valid_contours_data, group_solids_and_holes,
    grow_geometry, is_closed, linearize_data, linearize_geometry,
    normalize_winding_orders, optimize_path_from_array, partial_segment,
    project_circle_center_to_bisector, project_t_along_segment,
    remove_duplicate_segments, remove_duplicates, remove_inner_edges,
    resample_polyline, reverse_contour, segment_length, simplify_data,
    simplify_polyline, slice_scanline_data, smooth_circularly, smooth_polyline,
    smooth_sub_segment, solve_quadratic, split_inner_and_outer_contours,
    split_into_components, split_into_contours,
    subtract_polygons_from_line_segment, ContourHierarchy, ContourInfo,
    SegmentDelta,
};
pub use geometry::Geometry;
pub use math::{
    apply_affine_transform_to_array, map_geometry_to_frame, mat4_mul,
};
pub use query::{
    extract_overcut_rows, find_closest_point_on_path_from_array,
    get_bounding_rect_from_array, get_positions_at_distances_from_array,
    get_total_distance_from_array,
};
pub use shape::{
    are_points_equal, clean_polygon, clip_bezier_with_rect,
    compute_cubic_bezier_bounds_1d, convert_cubic_bezier_to_quadratic,
    do_rects_intersect, does_arc_intersect_circle, does_arc_intersect_rect,
    does_circle_intersect_rect, does_line_segment_intersect_circle,
    does_line_segment_intersect_rect, does_rect_contain_rect,
    does_rect_intersect_rect, evaluate_cubic, flatten_bezier, flip_polygon,
    flip_polygons, get_angle_at_vertex, get_arc_angles, get_arc_bounds,
    get_arc_closest_point, get_arc_direction, get_arc_length, get_arc_midpoint,
    get_bezier_bounds, get_bezier_closest_point, get_bezier_flatness_sq,
    get_bezier_length, get_bezier_point_at, get_bezier_rect_intersections,
    get_circle_circle_intersections, get_combined_rect,
    get_line_circle_intersections, get_line_closest_point,
    get_line_line_intersection, get_line_segment_closest_point,
    get_line_segment_intersection, get_line_segment_length,
    get_line_segment_polygon_intersections, get_perpendicular_dist_sq,
    get_point_line_distance, get_polygon_area, get_polygon_bounds,
    get_polygon_centroid, get_polygon_convex_hull, get_polygon_edges,
    get_polygon_group_bounds, get_polygon_perimeter, get_polygon_signed_area,
    get_polygons_difference, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_intersection,
    get_polygons_union, int_get_polygon_bounds, is_almost_equal,
    is_angle_between, is_arc_clockwise, is_arc_inside_polygons,
    is_bezier_inside_polygons, is_circle_inside_rect, is_point_in_polygon,
    is_point_inside_rect, is_point_on_segment, is_polygon_clockwise,
    is_polygon_convex, line_segment_intersects_circle, linearize_arc,
    linearize_bezier, linearize_bezier_adaptive, linearize_bezier_from_params,
    linearize_bezier_segment, midpoint, normalize_angle, normalize_polygons,
    offset_polygon, path_to_polygon, paths_to_polygons, point_line_distance,
    polygon_to_path, polygons_to_paths, project_point_onto_circle,
    rotate_polygon, rotate_polygons, scale_polygon, split_bezier,
    transform_point, translate_bounds, translate_polygon, translate_polygons,
    ClipperPath, ClipperPaths, GeoScale,
};
