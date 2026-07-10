//! Unified output channel for assemblers: builds Ops + records trace events.
//!
//! Motion methods (`move_to`, `line_to`, `apply_state`, …) push to the
//! internal [`Ops`] sequence.  Event methods (`init`, `cut`, `resume`,
//! `exit`) record [`TraceEventData`] for tracing.  At drain time, if no
//! events were emitted, Move events are auto-generated from the Ops —
//! eliminating the need for `replay_ops`.
//!
//! An optional progress callback receives batches of ops in real time for
//! streaming UI updates.

use crate::ops::container::Ops;
use crate::ops::types::{MoveCmd, OpCategory, OpNode};
use crate::ops::{Axis, State};
use crate::trace_types::{
    EventKind, Meta, MoveKind, ProgressSnapshot, ToolSnapshot,
};
use crate::types::Point3D;

use super::result::TraceEventData;

/// Single output channel for every assembler.
///
/// See the [module-level documentation](self) for details.
#[allow(dead_code)]
pub struct Tracelet {
    // Ops construction
    ops: Ops,
    // Trace events
    events: Vec<TraceEventData>,
    attrs: Option<Meta>,
    // Section tracking (for Workplan step boundaries)
    source: String,
    label: String,
    ops_offset: usize,
    step_idx: u32,
    // Position tracking (for auto-generated Move events)
    pos: Point3D,
    section_start_pos: Point3D,
    heading: f64,
    // Optional progress callback
    callback: Option<Box<dyn FnMut(ProgressEvent)>>,
    batch_size: usize,
    pending: usize,
    ops_at_last_flush: usize,
}

#[derive(Clone, Debug)]
pub enum ProgressEvent {
    StepStart {
        step_index: usize,
        label: String,
    },
    Ops {
        commands: Vec<OpNode>,
        ops_total: usize,
    },
    StepEnd {
        step_index: usize,
    },
}

#[allow(dead_code)]
impl Tracelet {
    pub fn new() -> Self {
        Tracelet {
            ops: Ops::new(),
            events: Vec::new(),
            attrs: None,
            source: String::new(),
            label: String::new(),
            ops_offset: 0,
            step_idx: 0,
            pos: Point3D::ZERO,
            section_start_pos: Point3D::ZERO,
            heading: 0.0,
            callback: None,
            batch_size: 0,
            pending: 0,
            ops_at_last_flush: 0,
        }
    }

    pub fn with_callback(
        callback: Box<dyn FnMut(ProgressEvent)>,
        batch_size: usize,
    ) -> Self {
        Tracelet {
            callback: Some(callback),
            batch_size,
            ..Self::new()
        }
    }

    // --- Motion methods (A2) ---

    pub fn move_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.ops.move_to(x, y, z, extra);
        self.pos = Point3D::new(x, y, z);
        self.maybe_flush();
    }

    pub fn line_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.ops.line_to(x, y, z, extra);
        self.pos = Point3D::new(x, y, z);
        self.maybe_flush();
    }

    pub fn apply_state(&mut self, state: &State) {
        self.ops.apply_state(state);
    }

    pub fn set_feed_rate(&mut self, rate: i32) {
        self.ops.set_feed_rate(rate);
    }

    pub fn set_power(&mut self, power: f64) {
        self.ops.set_power(power);
    }

    // --- Event methods (A3) ---

    pub fn init(&mut self, tool: ToolSnapshot, meta: Option<Meta>) {
        self.events.push(TraceEventData {
            kind: EventKind::Init,
            move_kind: None,
            tool: Some(tool),
            progress: Some(ProgressSnapshot {
                step_idx: 0,
                ops_len: self.ops.len() as u32,
                status: 0,
            }),
            meta,
        });
    }

    pub fn move_event(
        &mut self,
        move_kind: MoveKind,
        tool: ToolSnapshot,
        meta: Option<Meta>,
    ) {
        self.events.push(TraceEventData {
            kind: EventKind::Move,
            move_kind: Some(move_kind),
            tool: Some(tool),
            progress: Some(ProgressSnapshot {
                step_idx: self.step_idx,
                ops_len: self.ops.len() as u32,
                status: 0,
            }),
            meta,
        });
        self.step_idx += 1;
    }

    pub fn cut(&mut self, tool: ToolSnapshot, meta: Option<Meta>) {
        self.move_event(MoveKind::Cut, tool, meta);
    }

    pub fn resume(&mut self, tool: ToolSnapshot, meta: Option<Meta>) {
        self.events.push(TraceEventData {
            kind: EventKind::Resume,
            move_kind: None,
            tool: Some(tool),
            progress: Some(ProgressSnapshot {
                step_idx: self.step_idx,
                ops_len: self.ops.len() as u32,
                status: 0,
            }),
            meta,
        });
    }

    pub fn exit(&mut self, tool: ToolSnapshot, meta: Option<Meta>) {
        self.events.push(TraceEventData {
            kind: EventKind::Exit,
            move_kind: None,
            tool: Some(tool),
            progress: None,
            meta,
        });
    }

    // --- Section management (A4) ---

    pub fn set_attrs(&mut self, attrs: Meta) {
        self.attrs = Some(attrs);
    }

    pub fn attrs(&self) -> Option<&Meta> {
        self.attrs.as_ref()
    }

    pub fn begin_section(&mut self, source: &str, label: &str) {
        self.ops_offset = self.ops.len();
        self.section_start_pos = self.pos;
        self.source = source.to_string();
        self.label = label.to_string();
        self.step_idx = 0;
    }

    pub(crate) fn drain(&mut self) -> Vec<TraceEventData> {
        if self.events.is_empty() {
            self.generate_move_events();
        }
        self.ops_offset = self.ops.len();
        std::mem::take(&mut self.events)
    }

    // --- generate_move_events (private) ---

    fn generate_move_events(&mut self) {
        let commands = &self.ops.commands;
        let mut pos = self.section_start_pos;
        let mut heading = self.heading;
        let mut step_idx: u32 = 0;

        for i in self.ops_offset..commands.len() {
            let node = &commands[i];
            if !node.is_moving() {
                continue;
            }
            let endpoint = node.end_point();
            let kind = match &node.category {
                OpCategory::Moving {
                    cmd: MoveCmd::MoveTo,
                    ..
                } => MoveKind::Travel,
                _ => MoveKind::Cut,
            };
            let dx = endpoint.x - pos.x;
            let dy = endpoint.y - pos.y;
            if dx.abs() > 1e-12 || dy.abs() > 1e-12 {
                heading = dy.atan2(dx);
            }
            self.events.push(TraceEventData {
                kind: EventKind::Move,
                move_kind: Some(kind),
                tool: Some(ToolSnapshot {
                    pos_x: endpoint.x,
                    pos_y: endpoint.y,
                    pos_z: endpoint.z,
                    heading,
                    prev_x: pos.x,
                    prev_y: pos.y,
                    prev_z: pos.z,
                }),
                progress: Some(ProgressSnapshot {
                    step_idx,
                    ops_len: commands.len() as u32,
                    status: 0,
                }),
                meta: None,
            });
            pos = endpoint;
            step_idx += 1;
        }
    }

    // --- Progress callback (A5) ---

    fn maybe_flush(&mut self) {
        if self.callback.is_none() {
            return;
        }
        self.pending += 1;
        if self.pending >= self.batch_size {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.callback.is_none() || self.pending == 0 {
            self.pending = 0;
            return;
        }
        let new_ops = &self.ops.commands[self.ops_at_last_flush..];
        if let Some(ref mut cb) = self.callback {
            cb(ProgressEvent::Ops {
                commands: new_ops.to_vec(),
                ops_total: self.ops.len(),
            });
        }
        self.ops_at_last_flush = self.ops.len();
        self.pending = 0;
    }

    pub fn emit_step_start(&mut self, step_index: usize, label: &str) {
        self.flush();
        if let Some(ref mut cb) = self.callback {
            cb(ProgressEvent::StepStart {
                step_index,
                label: label.to_string(),
            });
        }
    }

    pub fn emit_step_end(&mut self, step_index: usize) {
        self.flush();
        if let Some(ref mut cb) = self.callback {
            cb(ProgressEvent::StepEnd { step_index });
        }
    }

    pub fn finish(&mut self) {
        self.flush();
    }

    /// Push a pre-built OpNode directly (used by Workplan to assemble ops
    /// from per-step temp tracelets in the correct order).
    pub fn push_raw(&mut self, node: OpNode) {
        if let OpCategory::Moving { end, .. } = &node.category {
            self.pos = *end;
        }
        self.ops.commands.push(node);
        self.ops.invalidate_time_cache();
        self.maybe_flush();
    }

    // --- Result extraction (A6) ---

    pub fn ops(&self) -> &Ops {
        &self.ops
    }

    pub fn into_ops(self) -> Ops {
        self.ops
    }
}

impl Default for Tracelet {
    fn default() -> Self {
        Self::new()
    }
}

// --- write_polyline helper (A7) ---

pub fn write_polyline(
    trace: &mut Tracelet,
    polyline: &[Point3D],
    move_first: bool,
    state: Option<&State>,
) {
    if let Some(s) = state {
        trace.apply_state(s);
    }
    if polyline.is_empty() {
        return;
    }
    if move_first {
        let f = polyline[0];
        trace.move_to(f.x, f.y, f.z, None);
        for pt in &polyline[1..] {
            trace.line_to(pt.x, pt.y, pt.z, None);
        }
    } else {
        for pt in polyline {
            trace.line_to(pt.x, pt.y, pt.z, None);
        }
    }
}
