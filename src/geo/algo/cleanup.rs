//! Cleanup: Deduplication and gap closing for geometry data.
//!
//! Provides functions for cleaning geometry command arrays by removing
//! duplicate segments and closing small gaps between connected paths.

use crate::geo::shape::point::are_points_equal;
use crate::types::Command;

/// Extract a hashable key for a segment. Returns None for MOVE commands.
pub fn get_segment_key(cmd: &Command) -> Option<(u32, [f64; 3], [f64; 4])> {
    match cmd {
        Command::Move { .. } => None,
        Command::Line { end } => Some((2, [end.0, end.1, end.2], [0.0; 4])),
        Command::Arc {
            end,
            center_offset,
            clockwise,
        } => {
            let params = [
                center_offset.0,
                center_offset.1,
                if *clockwise { 1.0 } else { 0.0 },
                0.0,
            ];
            Some((3, [end.0, end.1, end.2], params))
        }
        Command::Bezier {
            end,
            control1,
            control2,
        } => {
            let params = [control1.0, control1.1, control2.0, control2.1];
            Some((4, [end.0, end.1, end.2], params))
        }
    }
}

/// Check if two segment keys represent identical segments within tolerance.
pub fn are_segments_equal(
    k1: &(u32, [f64; 3], [f64; 4]),
    k2: &(u32, [f64; 3], [f64; 4]),
    tolerance: f64,
) -> bool {
    if k1.0 != k2.0 {
        return false;
    }
    if !are_points_equal(&k1.1, &k2.1, tolerance) {
        return false;
    }
    if k1.0 == 2 {
        return true;
    }
    if k1.0 == 3 {
        return are_points_equal(
            &[k1.2[0], k1.2[1], 0.0],
            &[k2.2[0], k2.2[1], 0.0],
            tolerance,
        ) && (k1.2[2] - k2.2[2]).abs() < tolerance;
    }
    if k1.0 == 4 {
        let p1 = [k1.2[0], k1.2[1], k1.2[2]];
        let p2 = [k2.2[0], k2.2[1], k2.2[2]];
        return are_points_equal(&p1, &p2, tolerance)
            && (k1.2[3] - k2.2[3]).abs() < tolerance;
    }
    false
}

/// Remove duplicate segments from geometry command data.
pub fn remove_duplicate_segments(
    data: &[Command],
    tolerance: f64,
) -> Vec<Command> {
    if data.is_empty() {
        return data.to_vec();
    }

    let mut result: Vec<Command> = Vec::new();
    let mut seen_segments: Vec<(u32, [f64; 3], [f64; 4])> = Vec::new();

    for cmd in data {
        if matches!(cmd, Command::Move { .. }) {
            seen_segments.clear();
            result.push(cmd.clone());
            continue;
        }

        if let Some(key) = get_segment_key(cmd) {
            let is_dup = seen_segments
                .iter()
                .any(|sk| are_segments_equal(&key, sk, tolerance));
            if is_dup {
                continue;
            }
            seen_segments.push(key);
        }
        result.push(cmd.clone());
    }

    result
}

/// Close small gaps in a geometry data array to form clean, connected paths.
pub fn close_geometry_gaps_from_array(
    data: &[Command],
    tolerance: f64,
) -> Vec<Command> {
    if data.len() < 2 {
        return data.to_vec();
    }

    let tol_sq = tolerance * tolerance;

    let mut move_indices: Vec<usize> = Vec::new();
    for (i, cmd) in data.iter().enumerate() {
        if matches!(cmd, Command::Move { .. }) {
            move_indices.push(i);
        }
    }

    let mut modified: Vec<Command> = data.to_vec();

    let sub_ranges: Vec<(usize, usize)> = if move_indices.is_empty() {
        vec![(0, data.len())]
    } else {
        let mut ranges = Vec::new();
        let mut prev = 0;
        for &mi in &move_indices[1..] {
            ranges.push((prev, mi));
            prev = mi;
        }
        ranges.push((prev, data.len()));
        ranges
    };

    for &(start, end) in &sub_ranges {
        if end - start >= 2 {
            let s = modified[start].end_point();
            let e_cmd = &modified[end - 1];
            let e = e_cmd.end_point();
            let dsq =
                (s.0 - e.0).powi(2) + (s.1 - e.1).powi(2) + (s.2 - e.2).powi(2);
            if dsq < tol_sq {
                let new_cmd = match e_cmd {
                    Command::Move { .. } => Command::Move { end: s },
                    Command::Line { .. } => Command::Line { end: s },
                    Command::Arc {
                        center_offset,
                        clockwise,
                        ..
                    } => Command::Arc {
                        end: s,
                        center_offset: *center_offset,
                        clockwise: *clockwise,
                    },
                    Command::Bezier {
                        control1, control2, ..
                    } => Command::Bezier {
                        end: s,
                        control1: *control1,
                        control2: *control2,
                    },
                };
                modified[end - 1] = new_cmd;
            }
        }
    }

    let mut final_rows: Vec<Command> = Vec::new();
    let mut last_end: Option<(f64, f64, f64)> = None;

    for cmd in &modified {
        let end_pt = cmd.end_point();

        if matches!(cmd, Command::Move { .. }) {
            if let Some(prev) = last_end {
                let dsq = (end_pt.0 - prev.0).powi(2)
                    + (end_pt.1 - prev.1).powi(2)
                    + (end_pt.2 - prev.2).powi(2);
                if dsq < tol_sq {
                    final_rows.push(Command::Line { end: prev });
                } else {
                    final_rows.push(cmd.clone());
                }
            } else {
                final_rows.push(cmd.clone());
            }
        } else {
            final_rows.push(cmd.clone());
        }
        last_end = Some(end_pt);
    }

    final_rows
}
