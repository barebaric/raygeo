//! Intersect: Geometry intersection detection.
//!
//! Provides functions for checking self-intersection and cross-intersection
//! of geometry command arrays. Arcs and Bezier curves are linearized into
//! line segments for testing, and an R-tree spatial index is used for
//! O(N log M) bounding-box lookups instead of brute-force O(N × M) scans.

use crate::constants::EPSILON_INTERSECT;
use crate::geo::shape::line::get_line_segment_intersection;
use crate::geo::types::{Command, Point, Point3D};
use glam::DVec2;
use rstar::{PointDistance, RTree, RTreeObject, AABB};

/// Returns a list of linearized line segments for a given command index.
fn get_segments_for_cmd(
    data: &[Command],
    index: usize,
    out: &mut Vec<(Point3D, Point3D)>,
) {
    let cmd = &data[index];
    let start_point = if index > 0 {
        data[index - 1].end_point()
    } else {
        Point3D::new(0.0, 0.0, 0.0)
    };
    cmd.linearize(start_point, 0.1, out);
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
        Point::new(dx, dy).length_squared()
    }
}

/// Pre-compute linearized segments and bounding boxes for all draw commands.
fn precompute_cmd_segments(data: &[Command]) -> Vec<RowSegments> {
    let mut rows = Vec::new();
    let mut buf = Vec::new();
    for i in 0..data.len() {
        let cmd = &data[i];
        if matches!(cmd, Command::Move { .. }) {
            continue;
        }
        get_segments_for_cmd(data, i, &mut buf);
        if buf.is_empty() {
            continue;
        }
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for (p1, p2) in &buf {
            for &pt in &[p1, p2] {
                if pt.x < min_x {
                    min_x = pt.x;
                }
                if pt.x > max_x {
                    max_x = pt.x;
                }
                if pt.y < min_y {
                    min_y = pt.y;
                }
                if pt.y > max_y {
                    max_y = pt.y;
                }
            }
        }
        rows.push(RowSegments {
            index: i,
            segments: std::mem::take(&mut buf),
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
        for ri2 in tree.locate_in_envelope_intersecting(query) {
            if is_self_check && ri2.index <= ri1.index {
                continue;
            }

            for &(seg1_p1, seg1_p2) in &ri1.segments {
                for &(seg2_p1, seg2_p2) in &ri2.segments {
                    let intersection = get_line_segment_intersection(
                        Point::new(seg1_p1.x, seg1_p1.y),
                        Point::new(seg1_p2.x, seg1_p2.y),
                        Point::new(seg2_p1.x, seg2_p1.y),
                        Point::new(seg2_p2.x, seg2_p2.y),
                    );

                    if let Some(pt) = intersection {
                        if is_self_check && ri2.index == ri1.index + 1 {
                            let shared_vertex = data1[ri1.index].end_point();
                            let dsq = pt.distance_squared(Point::new(
                                shared_vertex.x,
                                shared_vertex.y,
                            ));
                            if dsq < EPSILON_INTERSECT {
                                continue;
                            }
                            return true;
                        }

                        let at_end1 = pt
                            .distance_squared(Point::new(seg1_p1.x, seg1_p1.y))
                            < EPSILON_INTERSECT
                            || pt.distance_squared(Point::new(
                                seg1_p2.x, seg1_p2.y,
                            )) < EPSILON_INTERSECT;
                        let at_end2 = pt
                            .distance_squared(Point::new(seg2_p1.x, seg2_p1.y))
                            < EPSILON_INTERSECT
                            || pt.distance_squared(Point::new(
                                seg2_p2.x, seg2_p2.y,
                            )) < EPSILON_INTERSECT;

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

/// Intersect a ray with a line segment.
///
/// Given a ray starting at `origin` in direction `dir`, and a line segment
/// from `a` to `b`, returns the intersection point if the ray hits the
/// segment (including endpoints) in the forward direction.
pub fn get_ray_line_intersection(
    origin: Point,
    dir: Point,
    a: Point,
    b: Point,
) -> Option<Point> {
    let ex = b.x - a.x;
    let ey = b.y - a.y;
    let len2 = Point::new(ex, ey).length_squared();
    if len2 < 1e-24 {
        return None;
    }

    let nx = ey;
    let ny = -ex;
    let ndotd = DVec2::new(nx, ny).dot(dir);
    if ndotd.abs() < 1e-24 {
        return None;
    }

    let dx = a.x - origin.x;
    let dy = a.y - origin.y;
    let t = DVec2::new(nx, ny).dot(DVec2::new(dx, dy)) / ndotd;
    if t <= 1e-12 {
        return None;
    }

    let ix = origin.x + t * dir.x;
    let iy = origin.y + t * dir.y;

    let proj = (Point::new(ix, iy) - a).dot(Point::new(ex, ey)) / len2;
    if !(-1e-12..=1.0 + 1e-12).contains(&proj) {
        return None;
    }

    Some(Point::new(ix, iy))
}

/// Cast a ray from `origin` in `direction` and return the closest
/// intersection with any edge of `polygon` (or `None`).
pub fn get_ray_polygon_intersection(
    origin: Point,
    dir: Point,
    polygon: &[Point],
) -> Option<Point> {
    let mut best: Option<Point> = None;
    let mut best_t = f64::MAX;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if let Some(pt) = get_ray_line_intersection(origin, dir, a, b) {
            let t = (pt - origin).length_squared();
            if t > 1e-12 && t < best_t {
                best_t = t;
                best = Some(pt);
            }
        }
    }
    best
}

/// Check if two geometry data arrays intersect each other.
pub fn check_intersection_from_array(
    data1: &[Command],
    data2: &[Command],
    fail_on_t_junction: bool,
) -> bool {
    data_intersect(data1, data2, false, fail_on_t_junction)
}
