//! Shared meta and snapshot helpers for assembly trace modules.

use crate::geo::types::{Point3D, Polygon};
use crate::trace_types::{Meta, MetaValue, ToolSnapshot};

pub(crate) fn polygon_to_meta(poly: &Polygon) -> MetaValue {
    MetaValue::List(
        poly.iter()
            .map(|p| {
                MetaValue::List(vec![MetaValue::F64(p.x), MetaValue::F64(p.y)])
            })
            .collect(),
    )
}

pub(crate) fn xy_points_to_meta(points: &[(f64, f64)]) -> MetaValue {
    MetaValue::List(
        points
            .iter()
            .map(|(x, y)| {
                MetaValue::List(vec![MetaValue::F64(*x), MetaValue::F64(*y)])
            })
            .collect(),
    )
}

pub(crate) fn point3d_to_list(p: Point3D) -> MetaValue {
    MetaValue::List(vec![
        MetaValue::F64(p.x),
        MetaValue::F64(p.y),
        MetaValue::F64(p.z),
    ])
}

pub(crate) fn meta_insert_f64(meta: &mut Meta, key: &str, value: f64) {
    meta.insert(key.into(), MetaValue::F64(value));
}

pub(crate) fn meta_insert_u32(meta: &mut Meta, key: &str, value: u32) {
    meta.insert(key.into(), MetaValue::U32(value));
}

pub(crate) fn meta_insert_i64(meta: &mut Meta, key: &str, value: i64) {
    meta.insert(key.into(), MetaValue::I64(value));
}

pub(crate) fn meta_insert_bool(meta: &mut Meta, key: &str, value: bool) {
    meta.insert(key.into(), MetaValue::Bool(value));
}

pub(crate) fn tool_snapshot(
    pos: Point3D,
    heading: f64,
    prev: Point3D,
) -> ToolSnapshot {
    ToolSnapshot {
        pos_x: pos.x,
        pos_y: pos.y,
        pos_z: pos.z,
        heading,
        prev_x: prev.x,
        prev_y: prev.y,
        prev_z: prev.z,
    }
}

/// Compute the tangent heading at index `i` in a 3D path using forward
/// finite-difference. Returns 0.0 when the next point is unavailable or
/// the segment is degenerate.
pub(crate) fn path_heading(path: &[Point3D], i: usize) -> f64 {
    if i + 1 < path.len() {
        let dx = path[i + 1].x - path[i].x;
        let dy = path[i + 1].y - path[i].y;
        if dx.abs() > 1e-12 || dy.abs() > 1e-12 {
            return dy.atan2(dx);
        }
    }
    0.0
}

/// Compute start and end [`ToolPose`] from a path, using `fallback` as the
/// position when the path is empty.
pub(crate) fn start_end_poses(
    path: &[Point3D],
    fallback: Point3D,
) -> (crate::ops::types::ToolPose, crate::ops::types::ToolPose) {
    use crate::ops::types::ToolPose;
    let start = if path.is_empty() {
        ToolPose {
            pos: fallback,
            heading: 0.0,
        }
    } else {
        ToolPose {
            pos: path[0],
            heading: path_heading(path, 0),
        }
    };
    let end = if path.is_empty() {
        ToolPose {
            pos: fallback,
            heading: 0.0,
        }
    } else {
        let n = path.len();
        ToolPose {
            pos: path[n - 1],
            heading: path_heading(path, n - 1),
        }
    };
    (start, end)
}
