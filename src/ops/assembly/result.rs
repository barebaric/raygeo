//! Universal return type for assembly-level generators.

use crate::ops::container::Ops;
use crate::ops::cut::ToolPose;
use crate::trace::Tracer;
use crate::trace_types::{
    EventKind, Meta, MoveKind, ProgressSnapshot, ToolSnapshot,
};
use crate::types::Polygon;

/// One self-contained trace event produced by an assembler. The workplan
/// assigns seq/span when emitting; the assembler only describes WHAT
/// happened (kind, tool state, progress, move classification, meta).
#[derive(Clone, Debug)]
pub(crate) struct TraceEventData {
    pub(crate) kind: EventKind,
    pub(crate) move_kind: Option<MoveKind>,
    pub(crate) tool: Option<ToolSnapshot>,
    pub(crate) progress: Option<ProgressSnapshot>,
    pub(crate) meta: Option<Meta>,
}

/// Assembler-produced trace bundle: setup attrs for the span, plus an
/// ordered list of events. `None` on `AssemblyResult` means the assembler
/// did not self-trace and the workplan falls back to Ops replay.
#[derive(Clone, Debug, Default)]
pub(crate) struct AssemblyTrace {
    pub(crate) attrs: Option<Meta>,
    pub(crate) events: Vec<TraceEventData>,
}

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
    pub(crate) trace: Option<AssemblyTrace>,
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
    let trace = match (first.trace, second.trace) {
        (Some(mut t1), Some(t2)) => {
            t1.events.extend(t2.events);
            Some(t1)
        }
        (t1, t2) => t1.or(t2),
    };
    AssemblyResult {
        ops,
        cleared_polygons,
        start: first.start,
        end: second.end,
        trace,
    }
}

/// Emit trace events from an [`AssemblyTrace`] via the given [`Tracer`].
///
/// Maps each [`TraceEventData`] to the corresponding `tracer.*` call:
///
/// | `EventKind` | Tracer method |
/// |---|---|
/// | `Init` | [`tracer.init`] |
/// | `Move` | [`tracer.move_point`] |
/// | `Resume` / `Exit` / other | [`tracer.event`] |
///
/// Used by both the workplan executor and the Python `write_trace` binding.
pub(crate) fn emit_trace_events(
    tracer: &mut Tracer,
    span: u32,
    source: &str,
    events: &[TraceEventData],
) {
    for ev in events {
        match ev.kind {
            EventKind::Init => tracer.init(
                span,
                source,
                ev.tool.clone().unwrap_or_default(),
                ev.progress.clone().unwrap_or_default(),
                ev.meta.clone(),
            ),
            EventKind::Move => tracer.move_point(
                span,
                source,
                ev.move_kind.unwrap_or(MoveKind::Cut),
                ev.tool.clone().unwrap_or_default(),
                ev.progress.clone(),
                ev.meta.clone(),
            ),
            _ => tracer.event(
                span,
                source,
                ev.kind,
                ev.tool.clone(),
                ev.meta.clone(),
            ),
        }
    }
}
