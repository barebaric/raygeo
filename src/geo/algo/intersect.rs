//! Intersect: Geometry intersection detection.
//!
//! Provides functions for checking self-intersection and cross-intersection
//! of geometry command arrays. Arcs and Bezier curves are linearized into
//! line segments for testing, and an R-tree spatial index is used for
//! O(N log M) bounding-box lookups instead of brute-force O(N × M) scans.

use crate::constants::EPSILON_INTERSECT;
use crate::geo::shape::arc::linearize_arc;
use crate::geo::shape::bezier::linearize_bezier_from_params;
use crate::geo::shape::line::get_line_segment_intersection;
use crate::types::{Command, Point3D};
use rstar::{PointDistance, RTree, RTreeObject, AABB};

/// Returns a list of linearized line segments for a given command index.
fn get_segments_for_cmd(
    data: &[Command],
    index: usize,
) -> Vec<(Point3D, Point3D)> {
    let cmd = &data[index];
    let end_point = cmd.end_point();

    let start_point = if index > 0 {
        data[index - 1].end_point()
    } else {
        (0.0, 0.0, 0.0)
    };

    match cmd {
        Command::Line { .. } => {
            vec![(start_point, end_point)]
        }
        Command::Arc {
            end,
            center_offset,
            clockwise,
            ..
        } => linearize_arc(*end, *center_offset, *clockwise, start_point, 0.1),
        Command::Bezier {
            end,
            control1,
            control2,
            ..
        } => linearize_bezier_from_params(
            *end,
            *control1,
            *control2,
            start_point,
            0.1,
        ),
        Command::Move { .. } => vec![],
    }
}

/// Pre-computed segments and bounding box for a single command row.
struct RowSegments {
    index: usize,
    segments: Vec<(Point3D, Point3D)>,
    bbox: (f64, f64, f64, f64),
}

impl RowSegments {
    fn aabb(&self) -> AABB<[f64; 2]> {
        AABB::from_corners(
            [self.bbox.0, self.bbox.1],
            [self.bbox.2, self.bbox.3],
        )
    }
}

impl RTreeObject for RowSegments {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.aabb()
    }
}

impl PointDistance for RowSegments {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let dx = point[0].clamp(self.bbox.0, self.bbox.2) - point[0];
        let dy = point[1].clamp(self.bbox.1, self.bbox.3) - point[1];
        dx * dx + dy * dy
    }
}

/// Pre-compute linearized segments and bounding boxes for all draw commands.
fn precompute_cmd_segments(data: &[Command]) -> Vec<RowSegments> {
    let mut rows = Vec::new();
    for i in 0..data.len() {
        let cmd = &data[i];
        if matches!(cmd, Command::Move { .. }) {
            continue;
        }
        let segments = get_segments_for_cmd(data, i);
        if segments.is_empty() {
            continue;
        }
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for (p1, p2) in &segments {
            for &pt in &[p1, p2] {
                if pt.0 < min_x {
                    min_x = pt.0;
                }
                if pt.0 > max_x {
                    max_x = pt.0;
                }
                if pt.1 < min_y {
                    min_y = pt.1;
                }
                if pt.1 > max_y {
                    max_y = pt.1;
                }
            }
        }
        rows.push(RowSegments {
            index: i,
            segments,
            bbox: (min_x, min_y, max_x, max_y),
        });
    }
    rows
}

/// Core intersection test between two geometry command arrays.
fn data_intersect(
    data1: &[Command],
    data2: &[Command],
    is_self_check: bool,
    fail_on_t_junction: bool,
) -> bool {
    let rows1 = precompute_cmd_segments(data1);
    let rows2 = precompute_cmd_segments(data2);

    let tree = RTree::bulk_load(rows2);

    for ri1 in &rows1 {
        let query = ri1.aabb();
        for ri2 in tree.locate_in_envelope_intersecting(&query) {
            if is_self_check && ri2.index <= ri1.index {
                continue;
            }

            for &(seg1_p1, seg1_p2) in &ri1.segments {
                for &(seg2_p1, seg2_p2) in &ri2.segments {
                    let intersection = get_line_segment_intersection(
                        (seg1_p1.0, seg1_p1.1),
                        (seg1_p2.0, seg1_p2.1),
                        (seg2_p1.0, seg2_p1.1),
                        (seg2_p2.0, seg2_p2.1),
                    );

                    if let Some(pt) = intersection {
                        if is_self_check && ri2.index == ri1.index + 1 {
                            let shared_vertex = data1[ri1.index].end_point();
                            let dsq = (pt.0 - shared_vertex.0).powi(2)
                                + (pt.1 - shared_vertex.1).powi(2);
                            if dsq < EPSILON_INTERSECT {
                                continue;
                            }
                            return true;
                        }

                        let at_end1 = (pt.0 - seg1_p1.0).powi(2)
                            + (pt.1 - seg1_p1.1).powi(2)
                            < EPSILON_INTERSECT
                            || (pt.0 - seg1_p2.0).powi(2)
                                + (pt.1 - seg1_p2.1).powi(2)
                                < EPSILON_INTERSECT;
                        let at_end2 = (pt.0 - seg2_p1.0).powi(2)
                            + (pt.1 - seg2_p1.1).powi(2)
                            < EPSILON_INTERSECT
                            || (pt.0 - seg2_p2.0).powi(2)
                                + (pt.1 - seg2_p2.1).powi(2)
                                < EPSILON_INTERSECT;

                        if is_self_check
                            && (at_end1 || at_end2)
                            && !fail_on_t_junction
                        {
                            continue;
                        }
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if a path self-intersects.
pub fn check_self_intersection_from_array(
    data: &[Command],
    fail_on_t_junction: bool,
) -> bool {
    let mut move_indices: Vec<usize> = Vec::new();
    for (i, cmd) in data.iter().enumerate() {
        if matches!(cmd, Command::Move { .. }) {
            move_indices.push(i);
        }
    }

    if move_indices.is_empty() {
        return data_intersect(data, data, true, fail_on_t_junction);
    }

    for i in 0..move_indices.len() {
        let start = move_indices[i];
        let end = if i + 1 < move_indices.len() {
            move_indices[i + 1]
        } else {
            data.len()
        };

        if end - start > 1 {
            let subpath = &data[start..end];
            if data_intersect(subpath, subpath, true, fail_on_t_junction) {
                return true;
            }
        }
    }
    false
}

/// Check if two geometry data arrays intersect each other.
pub fn check_intersection_from_array(
    data1: &[Command],
    data2: &[Command],
    fail_on_t_junction: bool,
) -> bool {
    data_intersect(data1, data2, false, fail_on_t_junction)
}
