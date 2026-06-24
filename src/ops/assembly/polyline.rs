//! Motion assembly: polyline-to-Ops conversion.
//!
//! Bridges Tier-1 pure geometry (polylines as `Vec<Point3D>`) and the
//! [`Ops`] container. Domain-neutral — no `Tool`, no machine state.

use crate::ops::container::Ops;
use crate::types::Point3D;

/// Convert a 3D polyline into an [`Ops`] command sequence.
///
/// When `move_first` is `true` the first point is emitted as a
/// [`MoveTo`][crate::ops::enums::CommandType::MoveTo] and subsequent points
/// as [`LineTo`][crate::ops::enums::CommandType::LineTo] (the normal case for
/// starting a new cutting path).
///
/// When `move_first` is `false` every point is emitted as a `LineTo`,
/// useful for appending a polyline to an in-progress cut (the cutter
/// starts cutting from its current position with no preceding travel
/// move).
pub fn polyline_to_ops(polyline: &[Point3D], move_first: bool) -> Ops {
    let mut ops = Ops::new();

    if polyline.is_empty() {
        return ops;
    }

    if move_first {
        let first = polyline[0];
        ops.move_to(first.x, first.y, first.z, None);
        for pt in &polyline[1..] {
            ops.line_to(pt.x, pt.y, pt.z, None);
        }
    } else {
        for pt in polyline {
            ops.line_to(pt.x, pt.y, pt.z, None);
        }
    }

    ops
}
