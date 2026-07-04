//! Adaptive-clearing trace record format and recorder adapter.
//!
//! Defines the per-step record serialised as MessagePack via rmp-serde.
//! The generic [`crate::trace::Tracer`] writes these records to the
//! self-contained trace file (geometry + toolpath + records).
//!
//! [`TraceRecorder`] wraps the tracer and exposes one-line methods for
//! each record type.  All `#[cfg(debug_assertions)]` gating lives
//! inside the adapter — call sites in the orchestrator are unconditional.

use prof_macros::prof;

use serde::{Deserialize, Serialize};

use crate::ops::assembly::adaptive::AdaptiveClearingOptions;
use crate::ops::container::Ops;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::StepStatus;
#[cfg(debug_assertions)]
use crate::trace::{TracePoint, Tracer};
use crate::types::Point;

use super::tool::Tool;

// ── TraceKind ───────────────────────────────────────────────────────

/// Record kind byte values.
#[repr(u8)]
#[derive(Clone, Copy)]
pub(super) enum TraceKind {
    Init = 0,
    Cut = 1,
    ResumeStall = 2,
    ResumeStuck = 3,
    Exit = 4,
}

// ── TraceRecord ─────────────────────────────────────────────────────

/// Per-step trace record, serialised as MessagePack.
///
/// All fields that appear in every record are included; Cut-specific
/// fields (iters, iteration_angle, eng_*, cut_area) are set to 0 / 0.0
/// for non-Cut records.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(super) struct TraceRecord {
    pub kind: u8,
    pub status: u8,
    pub step_idx: u32,
    pub iters: u32,
    pub pos_x: f64,
    pub pos_y: f64,
    pub heading: f64,
    pub smoothed_heading: f64,
    pub predicted_angle: f64,
    pub iteration_angle: f64,
    pub eng_angle: f64,
    pub eng_area: f64,
    pub eng_chord: f64,
    pub cut_area: f64,
    pub total_area: f64,
    pub remaining_area: f64,
    pub prev_x: f64,
    pub prev_y: f64,
    pub ops_len: u32,
    pub resume_source: u8,
    pub route_source: u8,
    /// Wall-hug points in resume order: current segment first, then
    /// previous segments from oldest to newest.
    pub wall_hug_points: Vec<(f64, f64)>,
    /// Per-segment point counts corresponding to `wall_hug_points`.
    /// First entry = current segment, remaining = previous segments
    /// from oldest to newest.
    pub wall_hug_segment_counts: Vec<u32>,
    /// Per-strategy outcome for the current resume attempt.
    /// Index 0-5 correspond to the strategy priority order:
    ///   WallHug, Segment, Mat, Frontier, Envelope, Island.
    /// Each byte: 0 = not tried, 1 = no candidate, 2 = blacklisted.
    pub resume_strategy_reasons: [u8; 6],
    /// Per-strategy detail code giving context for *why* the strategy
    /// returned None.  Parallel to `resume_strategy_reasons`.
    /// See `resume::DETAIL_*` constants for values.
    pub resume_strategy_details: [u8; 6],
    /// Per-strategy detail code for the last routing failure.
    /// Index 0-3 = Direct, Frontier, Mat, AStar.
    /// 0 = success / not tried.  See `routing::ROUTE_*` constants.
    pub route_strategy_details: [u8; 4],
    /// Position of the last resume point candidate (routing target).
    pub resume_point_x: f64,
    pub resume_point_y: f64,
    /// Per-strategy candidate positions (x, y).  None entries are stored
    /// as (NaN, NaN).
    pub resume_candidate_points: [(f64, f64); 6],
}

impl TraceRecord {
    /// Build a record with the common tool-state fields filled in from
    /// their source objects.  Kind-specific fields (iters, eng_*, etc.)
    /// default to 0 / 0.0.
    pub fn from_tool_state(
        kind: u8,
        status: StepStatus,
        step_idx: u32,
        tool: &Tool,
        cleared: &ClearedArea,
        prev_pos: Point,
        ops_len: u32,
    ) -> Self {
        let status: u8 = match status {
            StepStatus::Ok => 0,
            StepStatus::BoundaryHit => 1,
            StepStatus::LostEngagement => 2,
            StepStatus::NoConvergence => 3,
        };
        Self {
            kind,
            status,
            step_idx,
            iters: 0,
            pos_x: tool.pos.x,
            pos_y: tool.pos.y,
            heading: tool.heading,
            smoothed_heading: tool.smoothed_heading(),
            predicted_angle: tool.raw_predictor(),
            iteration_angle: 0.0,
            eng_angle: 0.0,
            eng_area: 0.0,
            eng_chord: 0.0,
            cut_area: 0.0,
            total_area: cleared.total_area(),
            remaining_area: cleared.remaining_area(),
            prev_x: prev_pos.x,
            prev_y: prev_pos.y,
            ops_len,
            resume_source: 0,
            route_source: 0,
            wall_hug_points: Vec::new(),
            wall_hug_segment_counts: Vec::new(),
            resume_strategy_reasons: [0u8; 6],
            resume_strategy_details: [0u8; 6],
            route_strategy_details: [0u8; 4],
            resume_point_x: 0.0,
            resume_point_y: 0.0,
            resume_candidate_points: [(f64::NAN, f64::NAN); 6],
        }
    }
}

// ── Toolpath extraction ─────────────────────────────────────────────

use crate::geo::algo::medial_axis::MedialAxis;

/// Extract the moving commands (travel + cut) from `ops` as a
/// [`TracePoint`] list suitable for [`Tracer::write_toolpath`].
///
/// Order matches the record stream so the inspector can index toolpath
/// points by `ops_len` stored in each trace record.
#[cfg(debug_assertions)]
#[prof]
pub(super) fn extract_toolpath(ops: &Ops) -> Vec<TracePoint> {
    let mut out = Vec::new();
    for i in 0..ops.len() {
        let is_travel = ops.is_travel(i);
        let is_cutting = ops.is_cutting(i);
        if !is_travel && !is_cutting {
            continue;
        }
        let ep = ops.endpoint(i);
        out.push(TracePoint {
            x: ep.x,
            y: ep.y,
            is_travel,
        });
    }
    out
}

// ── TraceRecorder ───────────────────────────────────────────────────

/// Adapter that owns an optional [`Tracer`] and exposes one-line methods
/// for each record type.  All `#[cfg(debug_assertions)]` gating is
/// internal — call sites in the orchestrator are unconditional.
pub(super) struct TraceRecorder {
    #[cfg(debug_assertions)]
    tracer: Option<Tracer>,
    step_idx: u32,
}

#[cfg_attr(not(debug_assertions), allow(unused_variables))]
impl TraceRecorder {
    /// Create a recorder, opening the trace file when
    /// `opts.trace_path` is `Some`.
    pub fn new(opts: &AdaptiveClearingOptions, cleared: &ClearedArea) -> Self {
        #[cfg(debug_assertions)]
        let tracer = match &opts.trace_path {
            Some(path) => match Tracer::open(
                path,
                &crate::trace::TraceContext {
                    tool_radius: opts.radius,
                    boundary: opts.pocket_boundary.clone(),
                    islands: opts.islands.clone(),
                    seeds: cleared.fragments().to_vec(),
                },
            ) {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("trace: failed to open {:?}: {}", path, e);
                    None
                }
            },
            None => None,
        };

        Self {
            #[cfg(debug_assertions)]
            tracer,
            step_idx: 1,
        }
    }

    /// Write the optional MAT block.  Must be called once after
    /// creation and before any record methods.
    #[cfg(debug_assertions)]
    pub fn write_mat(&mut self, mat: Option<&MedialAxis>) {
        if let Some(ref mut tr) = self.tracer {
            tr.write_mat(mat.map(|m| m.into()));
        }
    }

    /// Record the initial tool state.
    pub fn record_init(
        &mut self,
        tool: &Tool,
        cleared: &ClearedArea,
        prev_pos: Point,
        ops_len: u32,
        wall_hug_points: &[(f64, f64)],
        wall_hug_segment_counts: &[u32],
    ) {
        #[cfg(debug_assertions)]
        if let Some(ref mut tr) = self.tracer {
            let mut rec = TraceRecord::from_tool_state(
                TraceKind::Init as u8,
                StepStatus::Ok,
                0,
                tool,
                cleared,
                prev_pos,
                ops_len,
            );
            rec.wall_hug_points = wall_hug_points.to_vec();
            rec.wall_hug_segment_counts = wall_hug_segment_counts.to_vec();
            tr.write(&rec);
        }
    }

    /// Record a cut step.  Increments the internal step index.
    #[allow(clippy::too_many_arguments)]
    pub fn record_cut(
        &mut self,
        status: StepStatus,
        tool: &Tool,
        cleared: &ClearedArea,
        prev_pos: Point,
        ops_len: u32,
        iters: u32,
        iteration_angle: f64,
        eng_angle: f64,
        eng_area: f64,
        eng_chord: f64,
        cut_area: f64,
        wall_hug_points: &[(f64, f64)],
        wall_hug_segment_counts: &[u32],
    ) {
        #[cfg(debug_assertions)]
        if let Some(ref mut tr) = self.tracer {
            let mut rec = TraceRecord::from_tool_state(
                TraceKind::Cut as u8,
                status,
                self.step_idx,
                tool,
                cleared,
                prev_pos,
                ops_len,
            );
            rec.iters = iters;
            rec.iteration_angle = iteration_angle;
            rec.eng_angle = eng_angle;
            rec.eng_area = eng_area;
            rec.eng_chord = eng_chord;
            rec.cut_area = cut_area;
            rec.wall_hug_points = wall_hug_points.to_vec();
            rec.wall_hug_segment_counts = wall_hug_segment_counts.to_vec();
            tr.write(&rec);
        }
        self.step_idx += 1;
    }

    /// Record a resume event (stall or stuck).  Increments the
    /// internal step index.
    #[allow(clippy::too_many_arguments)]
    pub fn record_resume(
        &mut self,
        kind: TraceKind,
        status: StepStatus,
        resume_source: u8,
        route_source: u8,
        tool: &Tool,
        cleared: &ClearedArea,
        prev_pos: Point,
        ops_len: u32,
        reasons: &[u8; 6],
        details: &[u8; 6],
        route_details: &[u8; 4],
        rp: Point,
        candidate_pts: &[(f64, f64); 6],
        wall_hug_points: &[(f64, f64)],
        wall_hug_segment_counts: &[u32],
    ) {
        #[cfg(debug_assertions)]
        if let Some(ref mut tr) = self.tracer {
            let mut rec = TraceRecord::from_tool_state(
                kind as u8,
                status,
                self.step_idx,
                tool,
                cleared,
                prev_pos,
                ops_len,
            );
            rec.resume_source = resume_source;
            rec.route_source = route_source;
            rec.resume_strategy_reasons = *reasons;
            rec.resume_strategy_details = *details;
            rec.route_strategy_details = *route_details;
            rec.resume_point_x = rp.x;
            rec.resume_point_y = rp.y;
            rec.resume_candidate_points = *candidate_pts;
            rec.wall_hug_points = wall_hug_points.to_vec();
            rec.wall_hug_segment_counts = wall_hug_segment_counts.to_vec();
            tr.write(&rec);
        }
        self.step_idx += 1;
    }

    /// Record an exit event.
    #[allow(clippy::too_many_arguments)]
    pub fn record_exit(
        &mut self,
        status: StepStatus,
        tool: &Tool,
        cleared: &ClearedArea,
        prev_pos: Point,
        ops_len: u32,
        reasons: &[u8; 6],
        details: &[u8; 6],
        route_details: &[u8; 4],
        rp: Point,
        candidate_pts: &[(f64, f64); 6],
        wall_hug_points: &[(f64, f64)],
        wall_hug_segment_counts: &[u32],
    ) {
        #[cfg(debug_assertions)]
        if let Some(ref mut tr) = self.tracer {
            let mut rec = TraceRecord::from_tool_state(
                TraceKind::Exit as u8,
                status,
                self.step_idx,
                tool,
                cleared,
                prev_pos,
                ops_len,
            );
            rec.resume_strategy_reasons = *reasons;
            rec.resume_strategy_details = *details;
            rec.route_strategy_details = *route_details;
            rec.resume_point_x = rp.x;
            rec.resume_point_y = rp.y;
            rec.resume_candidate_points = *candidate_pts;
            rec.wall_hug_points = wall_hug_points.to_vec();
            rec.wall_hug_segment_counts = wall_hug_segment_counts.to_vec();
            tr.write(&rec);
        }
    }

    /// Write the toolpath block and finalise the trace file.
    pub fn finish(self, ops: &Ops) {
        #[cfg(debug_assertions)]
        if let Some(mut t) = self.tracer {
            t.write_toolpath(&extract_toolpath(ops));
            let _ = t.finish();
        }
    }
}

// ── Helpers (called from the recorder) ──────────────────────────────
