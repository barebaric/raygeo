//! Query: Path queries and geometric analysis.
//!
//! This module provides functions for querying geometric paths including:
//! - Bounding box computation
//! - Total path distance calculation
//! - Closest point on path
//! - Bounding box intersection tests
//! - Path segment extraction

use std::f64::consts::PI;

use crate::constants::EPSILON_COLLINEAR;
use crate::geo::shape::arc::{get_arc_bounds, get_arc_closest_point};
use crate::geo::shape::bezier::{
    compute_cubic_bezier_bounds_1d, get_bezier_closest_point,
    linearize_bezier_from_array,
};
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::types::{Command, Point, Point3D, Rect};

use super::analysis::segment_length_from_row;

/// Compute the axis-aligned bounding rectangle for a geometry array.
///
/// Handles line, arc, and Bezier segments by computing their exact bounds.
///
/// - `data`: Array of geometry command rows (8 columns each).
/// - Returns: `(min_x, min_y, max_x, max_y)` bounding rectangle.
pub fn get_bounding_rect_from_array(data: &[[f64; 8]]) -> Rect {
    if data.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }

    // First pass: compute bounds from all line endpoints
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for row in data {
        let cmd = Command::from_row(row).expect("invalid command");
        let end = cmd.end_point();
        if end.0 < min_x {
            min_x = end.0;
        }
        if end.0 > max_x {
            max_x = end.0;
        }
        if end.1 < min_y {
            min_y = end.1;
        }
        if end.1 > max_y {
            max_y = end.1;
        }
    }

    // Second pass: check arcs for potentially larger bounds
    let mut last_point_2d: Point = (0.0, 0.0);
    for row in data {
        let cmd = Command::from_row(row).expect("invalid command");
        let end = cmd.end_point();
        if let Command::Arc {
            center_offset,
            clockwise,
            ..
        } = &cmd
        {
            let (ax1, ay1, ax2, ay2) = get_arc_bounds(
                last_point_2d,
                (end.0, end.1),
                *center_offset,
                *clockwise,
            );

            if ax1 < min_x {
                min_x = ax1;
            }
            if ay1 < min_y {
                min_y = ay1;
            }
            if ax2 > max_x {
                max_x = ax2;
            }
            if ay2 > max_y {
                max_y = ay2;
            }
        }
        last_point_2d = (end.0, end.1);
    }

    // Third pass: compute Bezier curve extrema analytically
    let mut last_point_3d: Point3D = (0.0, 0.0, 0.0);
    for row in data {
        let cmd = Command::from_row(row).expect("invalid command");
        let end = cmd.end_point();
        if let Command::Bezier {
            control1, control2, ..
        } = &cmd
        {
            let p0_x = vec![last_point_3d.0];
            let p0_y = vec![last_point_3d.1];
            let p1_x = vec![control1.0];
            let p1_y = vec![control1.1];
            let p2_x = vec![control2.0];
            let p2_y = vec![control2.1];
            let p3_x = vec![end.0];
            let p3_y = vec![end.1];

            let (bx_min, bx_max) =
                compute_cubic_bezier_bounds_1d(&p0_x, &p1_x, &p2_x, &p3_x);
            let (by_min, by_max) =
                compute_cubic_bezier_bounds_1d(&p0_y, &p1_y, &p2_y, &p3_y);

            min_x = min_x.min(bx_min[0]);
            max_x = max_x.max(bx_max[0]);
            min_y = min_y.min(by_min[0]);
            max_y = max_y.max(by_max[0]);
        }
        last_point_3d = end;
    }

    (min_x, min_y, max_x, max_y)
}

/// Compute the total path length by summing segment lengths.
///
/// Handles lines (Euclidean distance), arcs (arc length), and Beziers (linearized).
///
/// - `data`: Array of geometry command rows (8 columns each).
/// - Returns: The cumulative path distance.
pub fn get_total_distance_from_array(data: &[[f64; 8]]) -> f64 {
    let mut total_dist = 0.0;
    let mut last_point: Point3D = (0.0, 0.0, 0.0);

    for row in data {
        let cmd = Command::from_row(row).expect("invalid command");
        let end_point = cmd.end_point();

        match &cmd {
            Command::Move { .. } | Command::Line { .. } => {
                // Line segment: Euclidean distance
                total_dist += (end_point.0 - last_point.0)
                    .hypot(end_point.1 - last_point.1);
            }
            Command::Arc {
                center_offset,
                clockwise,
                ..
            } => {
                // Arc segment: arc length = radius * angle
                let center_x = last_point.0 + center_offset.0;
                let center_y = last_point.1 + center_offset.1;
                let radius = center_offset.0.hypot(center_offset.1);

                if radius > EPSILON_COLLINEAR {
                    let start_angle = (last_point.1 - center_y)
                        .atan2(last_point.0 - center_x);
                    let end_angle =
                        (end_point.1 - center_y).atan2(end_point.0 - center_x);
                    let mut angle_span = end_angle - start_angle;

                    // Normalize to handle full circles
                    if *clockwise {
                        if angle_span > EPSILON_COLLINEAR {
                            angle_span -= 2.0 * PI;
                        }
                    } else {
                        if angle_span < -EPSILON_COLLINEAR {
                            angle_span += 2.0 * PI;
                        }
                    }

                    total_dist += (angle_span * radius).abs();
                }
            }
            Command::Bezier { .. } => {
                // Bezier: linearize and sum segment lengths
                let segments =
                    linearize_bezier_from_array(row, last_point, 0.1);
                for (p1, p2) in segments {
                    total_dist += (p2.0 - p1.0).hypot(p2.1 - p1.1);
                }
            }
        }

        last_point = end_point;
    }

    total_dist
}

/// Find the closest point on the path to a given `(x, y)` coordinate.
///
/// - `data`: Array of geometry command rows (8 columns each).
/// - `x`: Target X coordinate.
/// - `y`: Target Y coordinate.
/// - Returns: `(command_index, t_parameter, closest_point)` if the path is non-empty.
pub fn find_closest_point_on_path_from_array(
    data: &[[f64; 8]],
    x: f64,
    y: f64,
) -> Option<(usize, f64, Point)> {
    let mut min_dist_sq = f64::INFINITY;
    let mut closest_info: Option<(usize, f64, Point)> = None;

    let mut last_pos_3d: Point3D = (0.0, 0.0, 0.0);

    for (i, row) in data.iter().enumerate() {
        let cmd = Command::from_row(row).expect("invalid command");
        let end_point_3d = cmd.end_point();

        if matches!(cmd, Command::Move { .. }) {
            last_pos_3d = end_point_3d;
            continue;
        }

        let start_pos_3d = last_pos_3d;

        match &cmd {
            Command::Line { .. } => {
                let t = get_line_segment_closest_point(
                    (start_pos_3d.0, start_pos_3d.1),
                    (end_point_3d.0, end_point_3d.1),
                    x,
                    y,
                );
                if t.2 < min_dist_sq {
                    min_dist_sq = t.2;
                    closest_info = Some((i, t.0, t.1));
                }
            }
            Command::Arc { .. } => {
                if let Some((t_arc, pt_arc, dist_sq_arc)) =
                    get_arc_closest_point(row, start_pos_3d, x, y)
                {
                    if dist_sq_arc < min_dist_sq {
                        min_dist_sq = dist_sq_arc;
                        closest_info = Some((i, t_arc, pt_arc));
                    }
                }
            }
            Command::Bezier { .. } => {
                if let Some((t_bezier, pt_bezier, dist_sq_bezier)) =
                    get_bezier_closest_point(row, start_pos_3d, x, y)
                {
                    if dist_sq_bezier < min_dist_sq {
                        min_dist_sq = dist_sq_bezier;
                        closest_info = Some((i, t_bezier, pt_bezier));
                    }
                }
            }
            Command::Move { .. } => {}
        }

        last_pos_3d = end_point_3d;
    }

    closest_info
}

/// Extract path segments up to a maximum length for overcut operations.
///
/// Returns the commands that fall within `max_length`, including a partial command
/// at the boundary if needed.
///
/// - `data`: Array of geometry command rows (8 columns each).
/// - `max_length`: Maximum path distance to extract.
/// - Returns: Collected rows up to `max_length`, or `None` if nothing was collected.
pub fn extract_overcut_rows(
    data: &[[f64; 8]],
    max_length: f64,
) -> Option<Vec<[f64; 8]>> {
    if data.len() < 2 || max_length <= 0.0 {
        return None;
    }

    let first_cmd = Command::from_row(&data[0]).expect("invalid command");
    let mut last_point: Point3D = first_cmd.end_point();
    let mut accumulated = 0.0;
    let mut collected: Vec<[f64; 8]> = Vec::new();

    for row in data.iter().skip(1) {
        let cmd = Command::from_row(row).expect("invalid command");
        let seg_length = segment_length_from_row(row, last_point);
        if seg_length < EPSILON_COLLINEAR {
            last_point = cmd.end_point();
            continue;
        }

        if accumulated + seg_length <= max_length + EPSILON_COLLINEAR {
            collected.push(*row);
            accumulated += seg_length;
        } else {
            let remaining = max_length - accumulated;
            if remaining > EPSILON_COLLINEAR {
                let t = remaining / seg_length;
                if let Some(partial) =
                    cmd.split_at_t(last_point, t).map(|c| c.to_row())
                {
                    collected.push(partial);
                }
            }
            break;
        }
        last_point = cmd.end_point();
    }

    if collected.is_empty() {
        None
    } else {
        Some(collected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::*;

    #[test]
    fn test_get_bounding_rect() {
        let data: [[f64; 8]; 3] = [
            [CMD_TYPE_MOVE, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [CMD_TYPE_LINE, 10.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [CMD_TYPE_LINE, 10.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        let rect = get_bounding_rect_from_array(&data[..]);
        assert_eq!(rect, (0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn test_get_total_distance() {
        let data: [[f64; 8]; 3] = [
            [CMD_TYPE_MOVE, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [CMD_TYPE_LINE, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [CMD_TYPE_LINE, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        let dist = get_total_distance_from_array(&data[..]);
        assert!((dist - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_find_closest_point_on_path() {
        let data: [[f64; 8]; 2] = [
            [CMD_TYPE_MOVE, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [CMD_TYPE_LINE, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        let result = find_closest_point_on_path_from_array(&data[..], 5.0, 1.0);
        assert!(result.is_some());
        let (idx, _t, pt) = result.unwrap();
        assert_eq!(idx, 1);
        assert!((pt.0 - 5.0).abs() < 1e-9);
        assert!((pt.1 - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_extract_overcut_rows() {
        let data: [[f64; 8]; 4] = [
            [CMD_TYPE_MOVE, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [CMD_TYPE_LINE, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [CMD_TYPE_LINE, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [CMD_TYPE_LINE, 15.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        let result = extract_overcut_rows(&data[..], 7.5);
        assert!(result.is_some());
        let rows = result.unwrap();
        assert_eq!(rows.len(), 2);
    }
}
