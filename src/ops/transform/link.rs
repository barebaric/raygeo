//! Transform: pass linking utilities.
//!
//! Joins ordered machining passes into a single Ops sequence with travel
//! moves between them, and provides helpers for finding pass boundaries.

use crate::ops::assembly::result::AssemblyResult;
use crate::ops::container::Ops;
use crate::ops::cut::ToolPose;
use crate::ops::enums::CommandCategory;
use crate::types::{Point3D, Polygon};

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

/// Join ordered `AssemblyResult`s into a single result with travel moves.
///
/// Uses each result's `.end` and `.start` poses to connect passes
/// according to `strategy`.  Cleared polygons are merged across passes.
///
/// Returns an empty `AssemblyResult` when `passes` is empty.
pub fn link_assembly_passes(
    passes: &[AssemblyResult],
    safe_z: f64,
    strategy: LinkStrategy,
) -> AssemblyResult {
    if passes.is_empty() {
        return AssemblyResult {
            ops: Ops::new(),
            cleared_polygons: vec![],
            start: ToolPose {
                pos: crate::types::Point::ZERO,
                heading: 0.0,
            },
            end: ToolPose {
                pos: crate::types::Point::ZERO,
                heading: 0.0,
            },
        };
    }

    let mut ops = Ops::new();
    let mut cleared_polygons: Vec<Polygon> = Vec::new();
    let mut prev_end = passes[0].end;

    ops.extend(&passes[0].ops);
    cleared_polygons.extend(passes[0].cleared_polygons.iter().cloned());

    for pass in &passes[1..] {
        let entry = pass.start.pos;
        let entry_z = pass_start_z(pass);

        match strategy {
            LinkStrategy::Retract => {
                ops.move_to(prev_end.pos.x, prev_end.pos.y, safe_z, None);
                if (entry.x - prev_end.pos.x).abs() > 1e-12
                    || (entry.y - prev_end.pos.y).abs() > 1e-12
                {
                    ops.move_to(entry.x, entry.y, safe_z, None);
                }
                if (entry_z - safe_z).abs() > 1e-12 {
                    ops.move_to(entry.x, entry.y, entry_z, None);
                }
            }
            LinkStrategy::StayDown => {
                ops.move_to(entry.x, entry.y, entry_z, None);
            }
        }

        ops.extend(&pass.ops);
        cleared_polygons.extend(pass.cleared_polygons.iter().cloned());
        prev_end = pass.end;
    }

    AssemblyResult {
        ops,
        cleared_polygons,
        start: passes[0].start,
        end: passes[passes.len() - 1].end,
    }
}

fn pass_start_z(result: &AssemblyResult) -> f64 {
    for i in 0..result.ops.len() {
        if result.ops.is_cutting(i) || result.ops.is_travel(i) {
            return result.ops.endpoint(i).z;
        }
    }
    0.0
}
