//! Universal return type for assembly-level generators.

use crate::ops::container::Ops;
use crate::ops::cut::ToolPose;
use crate::types::Polygon;

/// Universal return type for every assembly-level generator.
///
/// Every `generate_*()` and every existing assembler (`adaptive_clearing`,
/// `adaptive_wavefronts`, etc.) returns this, so any two can be chained by
/// linking `end` → `start` and merging `ops` + `cleared_polygons`.
#[derive(Clone, Debug)]
pub struct AssemblyResult {
    pub ops: Ops,
    pub cleared_polygons: Vec<Polygon>,
    pub start: ToolPose,
    pub end: ToolPose,
}

/// Chain two `AssemblyResult`s by concatenating ops and cleared polygons.
///
/// `second` is expected to begin where `first` left off; no extra travel
/// move is inserted (the caller is responsible for alignment).
pub fn chain(first: AssemblyResult, second: AssemblyResult) -> AssemblyResult {
    let mut ops = first.ops;
    ops.extend(&second.ops);
    let mut cleared_polygons = first.cleared_polygons;
    cleared_polygons.extend(second.cleared_polygons);
    AssemblyResult {
        ops,
        cleared_polygons,
        start: first.start,
        end: second.end,
    }
}
