//! Shape: Base geometric entities.
//!
//! This module provides fundamental geometric shapes and their operations.

pub mod arc;
pub mod bezier;
pub mod circle;
pub mod line;
pub mod point;
pub mod polygon;
pub mod polygon3d;
pub mod rect;

pub use arc::{
    does_arc_intersect_circle, does_arc_intersect_rect, get_arc_angles,
    get_arc_bounds, get_arc_closest_point, get_arc_direction, get_arc_length,
    get_arc_midpoint, is_angle_between, is_arc_clockwise,
    is_arc_inside_polygons, linearize_arc, normalize_angle,
};
pub use bezier::{
    clip_bezier_with_rect, compute_cubic_bezier_bounds_1d,
    convert_cubic_bezier_to_quadratic, evaluate_cubic, flatten_bezier,
    get_bezier_bounds, get_bezier_closest_point, get_bezier_flatness_sq,
    get_bezier_length, get_bezier_point_at, get_bezier_rect_intersections,
    get_perpendicular_dist_sq, is_bezier_flat, is_bezier_inside_polygons,
    linearize_bezier, linearize_bezier_adaptive, linearize_bezier_from_params,
    linearize_bezier_segment, split_bezier,
};
pub use circle::{
    does_circle_intersect_rect, get_circle_circle_intersections,
    get_line_circle_intersections, is_circle_inside_rect,
    line_segment_intersects_circle, project_point_onto_circle,
};
pub use line::{
    does_line_segment_intersect_circle, does_line_segment_intersect_rect,
    does_rect_contain_rect, does_rect_intersect_rect, get_angle_at_vertex,
    get_line_closest_point, get_line_line_intersection,
    get_line_segment_closest_point, get_line_segment_intersection,
    get_line_segment_length, get_line_segment_polygon_intersections,
    get_line_segment_polygon_intersections_into, get_point_line_distance,
    is_point_inside_rect, is_point_on_segment,
};
pub use point::{are_points_equal, midpoint, transform_point};
pub use polygon::{
    apply_minimum_curvature, clean_polygon, flip_polygon, flip_polygons,
    get_circle_polygon, get_polygon_area, get_polygon_bounds,
    get_polygon_centroid, get_polygon_closest_point, get_polygon_convex_hull,
    get_polygon_edges, get_polygon_group_bounds, get_polygon_perimeter,
    get_polygon_signed_area, get_polygons_difference,
    get_polygons_group_difference, get_polygons_group_intersection,
    get_polygons_intersection, get_polygons_union, get_segment_swept_polygon,
    is_almost_equal, is_point_in_polygon, is_polygon_clockwise,
    is_polygon_convex, normalize_polygons, offset_polygon_with_style,
    path_to_polygon, paths_to_polygons, point_line_distance, polygon_to_path,
    polygons_to_paths, rotate_polygon, rotate_polygons, scale_polygon,
    translate_bounds, translate_polygon, translate_polygons, ClipperPath,
    ClipperPaths, GeoScale, JoinStyle,
};
pub use polygon3d::{
    flip_polygon_3d, flip_polygons_3d, get_polygon_bounds_3d,
    get_polygon_centroid_3d, get_polygon_convex_hull_3d, get_polygon_edges_3d,
    get_polygon_group_bounds_3d, get_polygon_perimeter_3d,
    get_polygons_difference_3d, get_polygons_group_difference_3d,
    get_polygons_group_intersection_3d, get_polygons_intersection_3d,
    get_polygons_union_3d, get_polyline_end_tangent_3d, offset_polygon_3d,
    offset_polyline_3d, rotate_polygon_3d, rotate_polygons_3d,
    scale_polygon_3d, translate_polygon_3d, translate_polygons_3d,
};
pub use rect::{do_rects_intersect, get_combined_rect};
