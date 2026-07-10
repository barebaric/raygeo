//! Result types and trace infrastructure for assembly-level generators.

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
pub struct TraceEventData {
    pub(crate) kind: EventKind,
    pub(crate) move_kind: Option<MoveKind>,
    pub(crate) tool: Option<ToolSnapshot>,
    pub(crate) progress: Option<ProgressSnapshot>,
    pub(crate) meta: Option<Meta>,
}

/// Trace bundle: span attrs + ordered events.
/// Constructed by the caller from Tracelet data, not returned by assemblers.
#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub(crate) struct AssemblyTrace {
    pub(crate) attrs: Option<Meta>,
    pub(crate) events: Vec<TraceEventData>,
}

/// What an assembler returns alongside the ops/events written to the Tracelet.
#[derive(Clone, Debug)]
pub struct AssemblyMeta {
    pub cleared_polygons: Vec<Polygon>,
    pub start: ToolPose,
    pub end: ToolPose,
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
                ev.progress.clone(),
                ev.meta.clone(),
            ),
        }
    }
}
