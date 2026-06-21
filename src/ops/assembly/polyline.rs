//! Motion assembly: polyline-to-Ops conversion and pass linking.
//!
//! Bridges Tier-1 pure geometry (polylines as `Vec<Point3D>`) and the
//! [`Ops`] container. Domain-neutral — no `Tool`, no machine state.

use crate::ops::container::Ops;
use crate::ops::enums::CommandCategory;
use crate::types::Point3D;

/// Strategy for linking consecutive machining passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkStrategy {
    /// Retract to `safe_z` between passes, move XY at that height,
    /// then descend to the next pass start Z.
    Retract,
    /// Move directly from the previous pass end to the next pass
    /// start without retracting.
    StayDown,
}

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

// ── Helpers to find pass boundaries ──

pub(crate) fn find_pass_entry(ops: &Ops) -> Option<Point3D> {
    for i in 0..ops.len() {
        if ops.is_travel(i) {
            return Some(ops.endpoint(i));
        }
    }
    for i in 0..ops.len() {
        if ops.category(i) == CommandCategory::Moving {
            return Some(ops.endpoint(i));
        }
    }
    None
}

pub(crate) fn find_pass_exit(ops: &Ops) -> Option<Point3D> {
    for i in (0..ops.len()).rev() {
        if ops.category(i) == CommandCategory::Moving {
            return Some(ops.endpoint(i));
        }
    }
    None
}

/// Join ordered machining passes into a single [`Ops`] sequence with
/// travel moves between them according to `strategy`.
///
/// * `Retract` — between passes the tool retracts to `safe_z`, moves
///   XY at that height, then plunges to the next pass start Z.
/// * `StayDown` — the tool moves directly from the previous pass end to
///   the next pass start without retracting.
///
/// The first pass is emitted as-is; subsequent passes are prefixed with
/// the appropriate travel move(s) and then appended.
///
/// Returns an empty `Ops` when `passes` is empty.
pub fn link_passes(passes: &[Ops], safe_z: f64, strategy: LinkStrategy) -> Ops {
    if passes.is_empty() {
        return Ops::new();
    }

    let mut result = Ops::new();
    result.extend(&passes[0]);

    for pass in &passes[1..] {
        let prev_exit = match find_pass_exit(&result) {
            Some(p) => p,
            None => continue,
        };
        let entry = match find_pass_entry(pass) {
            Some(p) => p,
            None => continue,
        };

        match strategy {
            LinkStrategy::Retract => {
                result.move_to(prev_exit.x, prev_exit.y, safe_z, None);
                if (entry.x - prev_exit.x).abs() > 1e-12
                    || (entry.y - prev_exit.y).abs() > 1e-12
                {
                    result.move_to(entry.x, entry.y, safe_z, None);
                }
                if (entry.z - safe_z).abs() > 1e-12 {
                    result.move_to(entry.x, entry.y, entry.z, None);
                }
            }
            LinkStrategy::StayDown => {
                result.move_to(entry.x, entry.y, entry.z, None);
            }
        }

        result.extend(pass);
    }

    result
}
