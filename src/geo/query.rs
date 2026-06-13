//! Query: Path queries and geometric analysis.
//!
//! This module provides functions for querying geometric paths including:
//! - Bounding box computation
//! - Total path distance calculation
//! - Closest point on path
//! - Bounding box intersection tests
//! - Path segment extraction

use crate::constants::EPSILON_COLLINEAR;
use crate::geo::algo::analysis::{get_point_at_from_array, segment_length};
use crate::geo::shape::arc::{
    get_arc_bounds, get_arc_closest_point, get_arc_length,
};
use crate::geo::shape::bezier::{
    compute_cubic_bezier_bounds_1d, get_bezier_closest_point,
    linearize_bezier_from_params,
};
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::types::{Command, Point, Point3D, Rect};

/// Compute the axis-aligned bounding rectangle for a geometry command slice.
///
/// Handles line, arc, and Bezier segments by computing their exact bounds.
///
/// - `data`: Array of geometry commands.
/// - Returns: `(min_x, min_y, max_x, max_y)` bounding rectangle.
pub fn get_bounding_rect_from_array(data: &[Command]) -> Rect {
    if data.is_empty() {
        return Rect(0.0, 0.0, 0.0, 0.0);
    }

    // First pass: compute bounds from all endpoints
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for cmd in data {
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
    for cmd in data {
        let end = cmd.end_point();
        if let Command::Arc {
            center_offset,
            clockwise,
            ..
        } = cmd
        {
            let Rect(ax1, ay1, ax2, ay2) = get_arc_bounds(
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
    for cmd in data {
        let end = cmd.end_point();
        if let Command::Bezier {
            control1, control2, ..
        } = cmd
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

    Rect(min_x, min_y, max_x, max_y)
}

/// Compute the total path length by summing segment lengths.
///
/// Handles lines (Euclidean distance), arcs (arc length), and Beziers (linearized).
///
/// - `data`: Slice of geometry commands.
/// - Returns: The cumulative path distance.
pub fn get_total_distance_from_array(data: &[Command]) -> f64 {
    let mut total_dist = 0.0;
    let mut last_point: Point3D = (0.0, 0.0, 0.0);

    for cmd in data {
        let end_point = cmd.end_point();

        match cmd {
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
                total_dist += get_arc_length(
                    (last_point.0, last_point.1),
                    (end_point.0, end_point.1),
                    *center_offset,
                    *clockwise,
                );
            }
            Command::Bezier {
                end,
                control1,
                control2,
                ..
            } => {
                // Bezier: linearize and sum segment lengths
                let segments = linearize_bezier_from_params(
                    *end, *control1, *control2, last_point, 0.1,
                );
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
/// - `data`: Slice of geometry commands.
/// - `x`: Target X coordinate.
/// - `y`: Target Y coordinate.
/// - Returns: `(command_index, t_parameter, closest_point)` if the path is non-empty.
pub fn find_closest_point_on_path_from_array(
    data: &[Command],
    x: f64,
    y: f64,
) -> Option<(usize, f64, Point)> {
    let mut min_dist_sq = f64::INFINITY;
    let mut closest_info: Option<(usize, f64, Point)> = None;

    let mut last_pos_3d: Point3D = (0.0, 0.0, 0.0);

    for (i, cmd) in data.iter().enumerate() {
        let end_point_3d = cmd.end_point();

        if matches!(cmd, Command::Move { .. }) {
            last_pos_3d = end_point_3d;
            continue;
        }

        let start_pos_3d = last_pos_3d;

        match cmd {
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
            Command::Arc {
                end,
                center_offset,
                clockwise,
                ..
            } => {
                if let Some((t_arc, pt_arc, dist_sq_arc)) =
                    get_arc_closest_point(
                        *end,
                        *center_offset,
                        *clockwise,
                        start_pos_3d,
                        x,
                        y,
                    )
                {
                    if dist_sq_arc < min_dist_sq {
                        min_dist_sq = dist_sq_arc;
                        closest_info = Some((i, t_arc, pt_arc));
                    }
                }
            }
            Command::Bezier {
                end,
                control1,
                control2,
                ..
            } => {
                if let Some((t_bezier, pt_bezier, dist_sq_bezier)) =
                    get_bezier_closest_point(
                        *end,
                        *control1,
                        *control2,
                        start_pos_3d,
                        x,
                        y,
                    )
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
/// - `data`: Slice of geometry commands.
/// - `max_length`: Maximum path distance to extract.
/// - Returns: Collected commands up to `max_length`, or `None` if nothing was collected.
pub fn extract_overcut_rows(
    data: &[Command],
    max_length: f64,
) -> Option<Vec<Command>> {
    if data.len() < 2 || max_length <= 0.0 {
        return None;
    }

    let mut last_point: Point3D = data[0].end_point();
    let mut accumulated = 0.0;
    let mut collected: Vec<Command> = Vec::new();

    for cmd in data.iter().skip(1) {
        let seg_length = segment_length(cmd, last_point);
        if seg_length < EPSILON_COLLINEAR {
            last_point = cmd.end_point();
            continue;
        }

        if accumulated + seg_length <= max_length + EPSILON_COLLINEAR {
            collected.push(cmd.clone());
            accumulated += seg_length;
        } else {
            let remaining = max_length - accumulated;
            if remaining > EPSILON_COLLINEAR {
                let t = remaining / seg_length;
                if let Some(partial) = cmd.split_at_t(last_point, t) {
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

/// Given a list of distances along the path, returns the corresponding
/// (segment_index, t, point) for each distance.
///
/// Distances are clamped to [0, total_length]. Distances beyond the path
/// end return the last point.
///
/// - `data`: Slice of geometry commands.
/// - `distances`: Sorted list of distances along the path.
/// - Returns: Vec of (segment_index, t, point) in the same order as distances.
pub fn get_positions_at_distances_from_array(
    data: &[Command],
    distances: &[f64],
) -> Vec<(usize, f64, Point)> {
    if data.is_empty() || distances.is_empty() {
        return Vec::new();
    }

    let total = get_total_distance_from_array(data);
    let mut results = Vec::with_capacity(distances.len());
    let mut last_point: Point3D = (0.0, 0.0, 0.0);
    let mut cumulative = 0.0;
    let mut di = 0;

    for (seg_idx, cmd) in data.iter().enumerate() {
        if di >= distances.len() {
            break;
        }
        let end = cmd.end_point();

        if matches!(cmd, Command::Move { .. }) {
            last_point = end;
            continue;
        }

        let seg_len = segment_length(cmd, last_point);
        if seg_len < EPSILON_COLLINEAR {
            last_point = end;
            continue;
        }

        let next_cumulative = cumulative + seg_len;

        while di < distances.len() {
            let dist = distances[di].clamp(0.0, total);
            if dist > next_cumulative + EPSILON_COLLINEAR {
                break;
            }
            let dist_into = (dist - cumulative).max(0.0);
            let t = (dist_into / seg_len).clamp(0.0, 1.0);
            let pt = match get_point_at_from_array(data, seg_idx, t) {
                Some(p3) => (p3.0, p3.1),
                None => (last_point.0, last_point.1),
            };
            results.push((seg_idx, t, pt));
            di += 1;
        }

        cumulative = next_cumulative;
        last_point = end;
    }

    // Any remaining distances beyond path end: clamp to last point
    while di < distances.len() {
        let last_seg = data.len() - 1;
        let last_pt = data[last_seg].end_point();
        results.push((last_seg, 1.0, (last_pt.0, last_pt.1)));
        di += 1;
    }

    results
}
