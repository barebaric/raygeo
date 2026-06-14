//! Analysis: Path analysis and geometry metrics.
//!
//! This module provides functions for analyzing geometric paths including:
//! - Closed path detection
//! - Area calculation using the shoelace formula
//! - Winding order determination
//! - Point and tangent computation along paths
//! - Outward normal computation

use crate::geo::algo::topology::{
    get_valid_contours_data, split_into_contours,
};
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::is_point_inside_polygon;
use crate::types::{Command, Point, Point3D, Polygon, WindingOrder};

/// Checks if a path forms a closed loop within the given tolerance.
/// A closed path starts and ends at the same point.
pub fn is_closed(commands: &[Command], tolerance: f64) -> bool {
    if commands.len() < 2 {
        return false;
    }

    if !matches!(&commands[0], Command::Move { .. }) {
        return false;
    }

    let start_point = commands[0].end_point();
    let end_point = commands[commands.len() - 1].end_point();

    let dist_sq = (start_point.0 - end_point.0).powi(2)
        + (start_point.1 - end_point.1).powi(2)
        + (start_point.2 - end_point.2).powi(2);

    dist_sq < tolerance * tolerance
}

/// Extracts all vertices from a subpath starting at the given command index.
/// Linearizes arcs and Beziers into vertex sequences.
pub fn get_subpath_vertices_from_array(
    data: &[Command],
    start_cmd_index: usize,
) -> Polygon {
    let mut vertices: Polygon = Vec::new();
    if start_cmd_index >= data.len() {
        return vertices;
    }

    let last_pos_3d = data[start_cmd_index].end_point();
    vertices.push(Point(last_pos_3d.0, last_pos_3d.1));

    let mut linearize_buf = Vec::new();
    for cmd in data.iter().skip(start_cmd_index + 1) {
        if matches!(cmd, Command::Move { .. }) {
            break;
        }

        let start_3d: Point3D = if vertices.len() >= 2 {
            Point3D(
                vertices[vertices.len() - 1].0,
                vertices[vertices.len() - 1].1,
                last_pos_3d.2,
            )
        } else {
            last_pos_3d
        };

        cmd.linearize(start_3d, 0.1, &mut linearize_buf);
        for (_, p2) in linearize_buf.drain(..) {
            vertices.push(Point(p2.0, p2.1));
        }
    }

    vertices
}

/// Computes the signed area of a subpath using the shoelace formula.
/// Positive area indicates counter-clockwise (CCW), negative indicates clockwise (CW).
pub fn get_subpath_area_from_array(
    data: &[Command],
    start_cmd_index: usize,
) -> f64 {
    let vertices = get_subpath_vertices_from_array(data, start_cmd_index);
    if vertices.len() < 3 {
        return 0.0;
    }

    let p_start = vertices[0];
    let p_end = vertices[vertices.len() - 1];

    if (p_start.0 - p_end.0).abs() >= 1e-9
        || (p_start.1 - p_end.1).abs() >= 1e-9
    {
        return 0.0;
    }

    let mut area = 0.0;
    for i in 0..vertices.len() - 1 {
        let x = vertices[i].0;
        let y_shifted = vertices[i + 1].1;
        let y = vertices[i].1;
        let x_shifted = vertices[i + 1].0;
        area += x * y_shifted - x_shifted * y;
    }

    area / 2.0
}

/// Computes the total area enclosed by the geometry, summing all subpaths.
/// Returns the absolute value (unsigned area).
pub fn get_area_from_array(data: &[Command]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    if !matches!(&data[0], Command::Move { .. }) {
        return 0.0;
    }

    let mut total_signed_area = 0.0;

    let mut move_indices: Vec<usize> = Vec::new();
    for (i, cmd) in data.iter().enumerate() {
        if matches!(cmd, Command::Move { .. }) {
            move_indices.push(i);
        }
    }

    for &i in &move_indices {
        total_signed_area += get_subpath_area_from_array(data, i);
    }

    total_signed_area.abs()
}

/// Determines the winding order of a subpath based on signed area.
/// Returns `Some(CCW)` for counter-clockwise, `Some(CW)` for clockwise,
/// or `None` if the subpath is degenerate (zero area).
pub fn get_path_winding_order_from_array(
    data: &[Command],
    start_cmd_index: usize,
) -> Option<WindingOrder> {
    let area = get_subpath_area_from_array(data, start_cmd_index);

    if area.abs() < 1e-9 {
        None
    } else if area > 0.0 {
        Some(WindingOrder::CCW)
    } else {
        Some(WindingOrder::CW)
    }
}

/// Evaluates a point at a given t parameter along a path segment.
/// Returns None for MOVE commands or invalid indices.
pub fn get_point_at_from_array(
    data: &[Command],
    row_index: usize,
    t: f64,
) -> Option<Point3D> {
    if row_index >= data.len() {
        return None;
    }

    let cmd = &data[row_index];
    let start_pos_3d: Point3D = if row_index > 0 {
        data[row_index - 1].end_point()
    } else {
        Point3D(0.0, 0.0, 0.0)
    };

    cmd.point_at(start_pos_3d, t)
}

/// Evaluates a tangent at a given t parameter along a path segment.
/// Returns None for MOVE commands or invalid indices.
pub fn get_tangent_at_from_array(
    data: &[Command],
    row_index: usize,
    t: f64,
) -> Option<Point> {
    if row_index >= data.len() {
        return None;
    }

    let cmd = &data[row_index];
    let start_pos_3d: Point3D = if row_index > 0 {
        data[row_index - 1].end_point()
    } else {
        Point3D(0.0, 0.0, 0.0)
    };

    cmd.tangent_at(start_pos_3d, t)
}

/// Computes the outward-facing normal vector at a point on the path.
pub fn get_outward_normal_at_from_array(
    data: &[Command],
    row_index: usize,
    t: f64,
) -> Option<Point> {
    let mut subpath_start_index: isize = -1;
    for i in (0..=row_index).rev() {
        if matches!(&data[i], Command::Move { .. }) {
            subpath_start_index = i as isize;
            break;
        }
    }
    if subpath_start_index == -1 {
        subpath_start_index = 0;
    }

    let winding =
        get_path_winding_order_from_array(data, subpath_start_index as usize)?;

    let tangent = get_tangent_at_from_array(data, row_index, t)?;
    let tx = tangent.0;
    let ty = tangent.1;

    match winding {
        WindingOrder::CCW => Some(Point(ty, -tx)),
        WindingOrder::CW => Some(Point(-ty, tx)),
    }
}

pub fn remove_duplicates<T: Clone + PartialEq>(points: &[T]) -> Vec<T> {
    let mut result: Vec<T> = Vec::with_capacity(points.len());
    for p in points {
        if !result.contains(p) {
            result.push(p.clone());
        }
    }
    result
}

/// Check if a container geometry fully encloses a content geometry.
pub fn does_enclose(container: &Geometry, content: &Geometry) -> bool {
    if container.is_empty() || content.is_empty() {
        return false;
    }

    let cont_rect = container.rect();
    let other_rect = content.rect();
    if !(cont_rect.0 <= other_rect.0
        && cont_rect.1 <= other_rect.1
        && cont_rect.2 >= other_rect.2
        && cont_rect.3 >= other_rect.3)
    {
        return false;
    }

    if container_intersects_content(container, content) {
        return false;
    }

    let other_segments = content.segments();
    if other_segments.is_empty() || other_segments[0].is_empty() {
        return false;
    }
    let test_point: Point =
        Point(other_segments[0][0].0, other_segments[0][0].1);

    let self_contours = split_into_contours(container);
    let all_contour_data = get_valid_contours_data(&self_contours);

    let mut winding_number = 0;
    for (geo, vertices, is_closed) in &all_contour_data {
        if !is_closed {
            continue;
        }
        let area = get_subpath_area_from_array(&geo.data, 0);
        if is_point_inside_polygon(test_point, vertices) {
            if area > 1e-9 {
                winding_number += 1;
            } else if area < -1e-9 {
                winding_number -= 1;
            }
        }
    }

    winding_number > 0
}

pub fn segment_length(cmd: &Command, start_point: Point3D) -> f64 {
    cmd.length(start_point)
}

/// Computes a partial command from a command by interpolating at
/// parameter t along the segment. Returns None for MOVE commands.
pub fn partial_segment(
    cmd: &Command,
    start_point: Point3D,
    t: f64,
) -> Option<Command> {
    cmd.split_at_t(start_point, t)
}

fn container_intersects_content(
    container: &Geometry,
    content: &Geometry,
) -> bool {
    crate::geo::algo::intersect::check_intersection_from_array(
        &container.data,
        &content.data,
        true,
    )
}
