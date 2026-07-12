//! Merge overlapping line segments across all paths.
//!
//! Detects line segments that are collinear and overlapping (typically from
//! adjacent workpieces sharing an edge) and replaces the covered sub-segments
//! with travel moves to avoid cutting the same line twice.

use std::collections::{HashMap, HashSet};

use crate::ops::container::Ops;
use crate::ops::enums::{CommandCategory, CommandType};
use crate::types::Point3D;

/// Apply merge-lines to the given ops.
///
/// For each pair of collinear, overlapping line segments, the shorter
/// segment's covered portion is replaced with a travel move so the tool
/// doesn't cut the same line twice.
///
/// - `ops`: The input ops (will be replaced in-place).
/// - `tolerance`: Maximum distance for considering lines collinear.
pub fn merge_overlapping_lines(ops: &mut Ops, tolerance: f64) {
    if ops.is_empty() {
        return;
    }

    ops.preload_state();

    let segments = ops.segment_indices();
    let mut line_segments = extract_line_segments(ops, &segments);

    if line_segments.is_empty() {
        return;
    }

    find_duplicates(&mut line_segments, tolerance);

    let mut segment_map: HashMap<(usize, usize), usize> = HashMap::new();
    let mut has_covered = false;
    for (i, seg) in line_segments.iter().enumerate() {
        segment_map.insert((seg.segment_index, seg.command_index), i);
        if !seg.covered_intervals.is_empty() {
            has_covered = true;
        }
    }

    if !has_covered {
        return;
    }

    let mut new_ops = Ops::new();
    let mut machine_pos: Option<Point3D> = None;

    for (seg_idx, seg_indices) in segments.iter().enumerate() {
        let mut expected_pos: Option<Point3D> = None;

        for (cmd_pos, &idx) in seg_indices.iter().enumerate() {
            let line_seg_idx = segment_map.get(&(seg_idx, cmd_pos));
            let cat = ops.category(idx);
            let is_moving = cat == CommandCategory::Moving;

            if let Some(&lsi) = line_seg_idx {
                let seg = &line_segments[lsi];

                if !seg.covered_intervals.is_empty() {
                    let uncovered = get_uncovered(&seg.covered_intervals);
                    let p1 = seg.start;
                    let p2 = seg.end;
                    let seg_l = seg.length;

                    let filtered: Vec<(f64, f64)> = uncovered
                        .into_iter()
                        .filter(|&(u, v)| (v - u) * seg_l > tolerance * 0.5)
                        .collect();

                    for (u, v) in filtered {
                        let start_pt = interpolate(p1, p2, u);
                        let end_pt = interpolate(p1, p2, v);

                        if dist3d(machine_pos, start_pt) > 1e-5 {
                            new_ops.move_to(
                                start_pt.x, start_pt.y, start_pt.z, None,
                            );
                        }
                        new_ops.line_to(end_pt.x, end_pt.y, end_pt.z, None);
                        machine_pos = Some(end_pt);
                    }
                } else {
                    let p1 = seg.start;
                    if dist3d(machine_pos, p1) > 1e-5 {
                        new_ops.move_to(p1.x, p1.y, p1.z, None);
                    }
                    new_ops.commands.push(ops.commands[idx].clone());
                    machine_pos = Some(ops.endpoint(idx));
                }

                expected_pos = Some(ops.endpoint(idx));
            } else {
                let ct = ops.command_type(idx);
                let is_cut = is_moving && ct != CommandType::MoveTo;

                if is_cut {
                    if let Some(ep) = expected_pos {
                        if dist3d(machine_pos, ep) > 1e-5 {
                            new_ops.move_to(ep.x, ep.y, ep.z, None);
                            machine_pos = Some(ep);
                        }
                    }
                }

                new_ops.commands.push(ops.commands[idx].clone());

                if is_moving {
                    let end_pt = ops.endpoint(idx);
                    expected_pos = Some(end_pt);
                    if ct == CommandType::MoveTo || is_cut {
                        machine_pos = Some(end_pt);
                    }
                }
            }
        }
    }

    ops.replace_with(&new_ops);
}

// ---------------------------------------------------------------------------
// Line segment representation
// ---------------------------------------------------------------------------

struct LineSegment {
    start: Point3D,
    end: Point3D,
    segment_index: usize,
    command_index: usize,
    covered_intervals: Vec<(f64, f64)>,
    dx: f64,
    dy: f64,
    length_sq: f64,
    length: f64,
    dir_x: f64,
    dir_y: f64,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl LineSegment {
    fn new(
        start: Point3D,
        end: Point3D,
        segment_index: usize,
        command_index: usize,
    ) -> Self {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length_sq = dx * dx + dy * dy;
        let length = length_sq.sqrt();
        let (dir_x, dir_y) = if length > 1e-9 {
            (dx / length, dy / length)
        } else {
            (0.0, 0.0)
        };
        let min_x = start.x.min(end.x);
        let max_x = start.x.max(end.x);
        let min_y = start.y.min(end.y);
        let max_y = start.y.max(end.y);

        LineSegment {
            start,
            end,
            segment_index,
            command_index,
            covered_intervals: Vec::new(),
            dx,
            dy,
            length_sq,
            length,
            dir_x,
            dir_y,
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }
}

// ---------------------------------------------------------------------------
// Segment extraction
// ---------------------------------------------------------------------------

fn extract_line_segments(
    ops: &Ops,
    segments: &[Vec<usize>],
) -> Vec<LineSegment> {
    let mut line_segments: Vec<LineSegment> = Vec::new();

    for (seg_idx, seg_indices) in segments.iter().enumerate() {
        if !is_line_segment(ops, seg_indices) {
            continue;
        }

        let mut current_pos = ops.endpoint(seg_indices[0]);

        for (cmd_pos, &idx) in seg_indices.iter().enumerate().skip(1) {
            let end_pos = ops.endpoint(idx);
            line_segments.push(LineSegment::new(
                current_pos,
                end_pos,
                seg_idx,
                cmd_pos,
            ));
            current_pos = end_pos;
        }
    }

    line_segments
}

fn is_line_segment(ops: &Ops, indices: &[usize]) -> bool {
    if indices.len() < 2 {
        return false;
    }
    if ops.command_type(indices[0]) != CommandType::MoveTo {
        return false;
    }
    for &i in &indices[1..] {
        if ops.command_type(i) != CommandType::LineTo {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Duplicate detection with spatial index
// ---------------------------------------------------------------------------

fn find_duplicates(line_segments: &mut [LineSegment], tolerance: f64) {
    if line_segments.is_empty() {
        return;
    }

    let cell_size = (tolerance * 10.0).max(1.0);
    let mut checked_pairs: HashSet<(usize, usize)> = HashSet::new();
    let mut index: HashMap<(i64, i64), Vec<usize>> = HashMap::new();

    for (i, seg) in line_segments.iter().enumerate() {
        for cell_key in get_cell_keys(seg, cell_size, tolerance) {
            index.entry(cell_key).or_default().push(i);
        }
    }

    for indices in index.values() {
        if indices.len() < 2 {
            continue;
        }

        for i in 0..indices.len() {
            let idx1 = indices[i];

            for &idx2 in &indices[i + 1..] {
                let pair = if idx1 < idx2 {
                    (idx1, idx2)
                } else {
                    (idx2, idx1)
                };
                if checked_pairs.contains(&pair) {
                    continue;
                }
                checked_pairs.insert(pair);

                let (seg1, seg2) = (&line_segments[idx1], &line_segments[idx2]);

                if !z_overlap(seg1, seg2) {
                    continue;
                }

                if !are_collinear(seg1, seg2, tolerance) {
                    continue;
                }

                let len1 = seg1.length;
                let len2 = seg2.length;

                let (coverer_idx, coveree_idx) = if len1 > len2 + 1e-5 {
                    (idx1, idx2)
                } else if len2 > len1 + 1e-5 {
                    (idx2, idx1)
                } else if (seg1.segment_index, seg1.command_index)
                    < (seg2.segment_index, seg2.command_index)
                {
                    (idx1, idx2)
                } else {
                    (idx2, idx1)
                };

                let coverer_start = line_segments[coverer_idx].start;
                let coverer_end = line_segments[coverer_idx].end;
                let coveree_start = line_segments[coveree_idx].start;
                let coveree_dx = line_segments[coveree_idx].dx;
                let coveree_dy = line_segments[coveree_idx].dy;
                let coveree_l_sq = line_segments[coveree_idx].length_sq;
                let coveree_length = line_segments[coveree_idx].length;

                if coveree_l_sq < 1e-12 {
                    continue;
                }

                let p1 = coveree_start;
                let (dx, dy) = (coveree_dx, coveree_dy);
                let c = coverer_start;
                let d = coverer_end;

                let t_c =
                    ((c.x - p1.x) * dx + (c.y - p1.y) * dy) / coveree_l_sq;
                let t_d =
                    ((d.x - p1.x) * dx + (d.y - p1.y) * dy) / coveree_l_sq;

                let t_min_raw = t_c.min(t_d);
                let t_max_raw = t_c.max(t_d);

                let t_tol = tolerance / coveree_length;

                let t_min = (0.0_f64).max(t_min_raw - t_tol);
                let t_max = (1.0_f64).min(t_max_raw + t_tol);

                if t_min < t_max - 1e-6 {
                    line_segments[coveree_idx]
                        .covered_intervals
                        .push((t_min, t_max));
                }
            }
        }
    }
}

fn get_cell_keys(
    seg: &LineSegment,
    cell_size: f64,
    tolerance: f64,
) -> Vec<(i64, i64)> {
    let cx1 = ((seg.min_x - tolerance) / cell_size).floor() as i64;
    let cx2 = ((seg.max_x + tolerance) / cell_size).floor() as i64;
    let cy1 = ((seg.min_y - tolerance) / cell_size).floor() as i64;
    let cy2 = ((seg.max_y + tolerance) / cell_size).floor() as i64;

    let mut keys = Vec::new();
    for cx in cx1..=cx2 {
        for cy in cy1..=cy2 {
            keys.push((cx, cy));
        }
    }
    keys
}

// ---------------------------------------------------------------------------
// Geometry predicates
// ---------------------------------------------------------------------------

fn z_overlap(seg1: &LineSegment, seg2: &LineSegment) -> bool {
    let z1s = seg1.start.z;
    let z1e = seg1.end.z;
    let z2s = seg2.start.z;
    let z2e = seg2.end.z;
    if z1s == 0.0 && z1e == 0.0 && z2s == 0.0 && z2e == 0.0 {
        return true;
    }
    let min_z1 = z1s.min(z1e);
    let max_z1 = z1s.max(z1e);
    let min_z2 = z2s.min(z2e);
    let max_z2 = z2s.max(z2e);
    min_z1 <= max_z2 && min_z2 <= max_z1
}

fn are_parallel(seg1: &LineSegment, seg2: &LineSegment) -> bool {
    let dot = (seg1.dir_x * seg2.dir_x + seg1.dir_y * seg2.dir_y).abs();
    dot > 0.9999
}

fn get_point_line_distance(point: Point3D, seg: &LineSegment) -> f64 {
    let (x0, y0) = (point.x, point.y);
    let (x1, y1) = (seg.start.x, seg.start.y);
    let (x2, y2) = (seg.end.x, seg.end.y);
    if seg.length_sq < 1e-12 {
        return (x0 - x1).hypot(y0 - y1);
    }
    let num = ((y2 - y1) * x0 - (x2 - x1) * y0 + x2 * y1 - y2 * x1).abs();
    num / seg.length
}

fn are_collinear(seg1: &LineSegment, seg2: &LineSegment, tol: f64) -> bool {
    if seg1.max_x < seg2.min_x - tol
        || seg2.max_x < seg1.min_x - tol
        || seg1.max_y < seg2.min_y - tol
        || seg2.max_y < seg1.min_y - tol
    {
        return false;
    }
    if !are_parallel(seg1, seg2) {
        return false;
    }
    let dist1 = get_point_line_distance(seg2.start, seg1);
    if dist1 > tol {
        return false;
    }
    let dist2 = get_point_line_distance(seg2.end, seg1);
    dist2 <= tol
}

// ---------------------------------------------------------------------------
// Interval utilities
// ---------------------------------------------------------------------------

fn get_uncovered(intervals: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if intervals.is_empty() {
        return vec![(0.0, 1.0)];
    }

    let mut sorted: Vec<(f64, f64)> = intervals.to_vec();
    sorted.sort_by(|a, b| crate::utils::sort_f64(a.0, b.0));

    let mut merged: Vec<(f64, f64)> = vec![sorted[0]];
    for current in &sorted[1..] {
        let Some(last) = merged.last_mut() else { break };
        if current.0 <= last.1 + 1e-6 {
            *last = (last.0, last.1.max(current.1));
        } else {
            merged.push(*current);
        }
    }

    let mut uncovered: Vec<(f64, f64)> = Vec::new();
    let mut current_t = 0.0;
    for (start, end) in &merged {
        if *start > current_t + 1e-6 {
            uncovered.push((current_t, *start));
        }
        current_t = current_t.max(*end);
    }
    if current_t < 1.0 - 1e-6 {
        uncovered.push((current_t, 1.0));
    }
    uncovered
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

fn interpolate(p1: Point3D, p2: Point3D, t: f64) -> Point3D {
    Point3D::new(
        p1.x + (p2.x - p1.x) * t,
        p1.y + (p2.y - p1.y) * t,
        p1.z + (p2.z - p1.z) * t,
    )
}

fn dist3d(a: Option<Point3D>, b: Point3D) -> f64 {
    match a {
        Some(a) => {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let dz = b.z - a.z;
            (dx * dx + dy * dy + dz * dz).sqrt()
        }
        None => f64::INFINITY,
    }
}
