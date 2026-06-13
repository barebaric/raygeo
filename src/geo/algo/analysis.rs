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
use crate::geo::shape::arc::{get_arc_sweep, linearize_arc};
use crate::geo::shape::bezier::linearize_bezier_from_params;
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
    vertices.push((last_pos_3d.0, last_pos_3d.1));

    for cmd in data.iter().skip(start_cmd_index + 1) {
        if matches!(cmd, Command::Move { .. }) {
            break;
        }

        let end_point_3d = cmd.end_point();

        match cmd {
            Command::Line { .. } => {
                vertices.push((end_point_3d.0, end_point_3d.1));
            }
            Command::Arc {
                end,
                center_offset,
                clockwise,
                ..
            } => {
                let start_3d: Point3D = if vertices.len() >= 2 {
                    (
                        vertices[vertices.len() - 1].0,
                        vertices[vertices.len() - 1].1,
                        last_pos_3d.2,
                    )
                } else {
                    last_pos_3d
                };
                let segments = linearize_arc(
                    *end,
                    *center_offset,
                    *clockwise,
                    start_3d,
                    0.1,
                );
                for (_, p2) in segments {
                    vertices.push((p2.0, p2.1));
                }
            }
            Command::Bezier {
                end,
                control1,
                control2,
                ..
            } => {
                let start_3d: Point3D = if vertices.len() >= 2 {
                    (
                        vertices[vertices.len() - 1].0,
                        vertices[vertices.len() - 1].1,
                        last_pos_3d.2,
                    )
                } else {
                    last_pos_3d
                };
                let segments = linearize_bezier_from_params(
                    *end, *control1, *control2, start_3d, 0.1,
                );
                for (_, p2) in segments {
                    vertices.push((p2.0, p2.1));
                }
            }
            _ => {}
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
        (0.0, 0.0, 0.0)
    };

    let p0 = start_pos_3d;
    let end_3d = cmd.end_point();
    let p1 = end_3d;

    let (px, py) = match cmd {
        Command::Line { .. } => {
            let px = p0.0 + t * (p1.0 - p0.0);
            let py = p0.1 + t * (p1.1 - p0.1);
            (px, py)
        }
        Command::Arc {
            center_offset,
            clockwise,
            ..
        } => {
            let center = (p0.0 + center_offset.0, p0.1 + center_offset.1);

            let start_angle = (p0.1 - center.1).atan2(p0.0 - center.0);
            let end_angle = (p1.1 - center.1).atan2(p1.0 - center.0);
            let angle_range = get_arc_sweep(start_angle, end_angle, *clockwise);
            let current_angle = start_angle + t * angle_range;
            let radius_start = (p0.0 - center.0).hypot(p0.1 - center.1);
            let radius_end = (p1.0 - center.0).hypot(p1.1 - center.1);
            let radius = radius_start + t * (radius_end - radius_start);

            let px = center.0 + radius * current_angle.cos();
            let py = center.1 + radius * current_angle.sin();
            (px, py)
        }
        Command::Bezier {
            control1, control2, ..
        } => {
            let c1 = *control1;
            let c2 = *control2;

            let omt = 1.0 - t;
            let px = omt.powi(3) * p0.0
                + 3.0 * omt.powi(2) * t * c1.0
                + 3.0 * omt * t.powi(2) * c2.0
                + t.powi(3) * p1.0;
            let py = omt.powi(3) * p0.1
                + 3.0 * omt.powi(2) * t * c1.1
                + 3.0 * omt * t.powi(2) * c2.1
                + t.powi(3) * p1.1;
            (px, py)
        }
        _ => return None,
    };

    let pz = p0.2 + t * (p1.2 - p0.2);
    Some((px, py, pz))
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
        (0.0, 0.0, 0.0)
    };

    let p0 = (start_pos_3d.0, start_pos_3d.1);
    let end_3d = cmd.end_point();
    let p1 = (end_3d.0, end_3d.1);

    let tangent_vec: Point = match cmd {
        Command::Line { .. } => (p1.0 - p0.0, p1.1 - p0.1),
        Command::Arc {
            center_offset,
            clockwise,
            ..
        } => {
            let center = (p0.0 + center_offset.0, p0.1 + center_offset.1);

            let start_angle = (p0.1 - center.1).atan2(p0.0 - center.0);
            let end_angle = (p1.1 - center.1).atan2(p1.0 - center.0);
            let angle_range = get_arc_sweep(start_angle, end_angle, *clockwise);
            let current_angle = start_angle + t * angle_range;
            let radius_start = (p0.0 - center.0).hypot(p0.1 - center.1);
            let radius_end = (p1.0 - center.0).hypot(p1.1 - center.1);
            let radius = radius_start + t * (radius_end - radius_start);

            let point = (
                center.0 + radius * current_angle.cos(),
                center.1 + radius * current_angle.sin(),
            );

            let radius_vec = (point.0 - center.0, point.1 - center.1);
            if *clockwise {
                (radius_vec.1, -radius_vec.0)
            } else {
                (-radius_vec.1, radius_vec.0)
            }
        }
        Command::Bezier {
            control1, control2, ..
        } => {
            let c1 = *control1;
            let c2 = *control2;

            let omt = 1.0 - t;
            let tx = 3.0 * omt.powi(2) * (c1.0 - p0.0)
                + 6.0 * omt * t * (c2.0 - c1.0)
                + 3.0 * t.powi(2) * (p1.0 - c2.0);
            let ty = 3.0 * omt.powi(2) * (c1.1 - p0.1)
                + 6.0 * omt * t * (c2.1 - c1.1)
                + 3.0 * t.powi(2) * (p1.1 - c2.1);
            (tx, ty)
        }
        _ => return None,
    };

    let norm = (tangent_vec.0.powi(2) + tangent_vec.1.powi(2)).sqrt();
    if norm < 1e-9 {
        return Some((1.0, 0.0));
    }

    Some((tangent_vec.0 / norm, tangent_vec.1 / norm))
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
    let (tx, ty) = tangent;

    match winding {
        WindingOrder::CCW => Some((ty, -tx)),
        WindingOrder::CW => Some((-ty, tx)),
    }
}

pub fn remove_duplicates<T: Clone + PartialEq>(points: &[T]) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    for p in points {
        if result.is_empty() || !result.contains(p) {
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
    let test_point: Point = (other_segments[0][0].0, other_segments[0][0].1);

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
