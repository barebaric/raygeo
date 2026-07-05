//! Adaptive-clearing trace record format and recorder adapter.
//!
//! Defines the per-step record serialised as MessagePack via rmp-serde.
//! The generic [`crate::trace::Tracer`] writes these records to the
//! self-contained trace file.
//!
//! [`TraceRecorder`] wraps an optional [`Tracer`] and exposes one-line
//! methods for each record type.  Runtime gating (via
//! [`AdaptiveClearingOptions::trace_path`]) means call sites in the
//! orchestrator are unconditional.

use serde::Serialize;

use crate::geo::algo::medial_axis::MedialAxis;
use crate::ops::assembly::adaptive::AdaptiveClearingOptions;
use crate::ops::container::Ops;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::StepStatus;
use crate::trace::Tracer;
use crate::types::Point3D;

use super::tool::Tool;

// ── TraceKind ───────────────────────────────────────────────────────

/// Record kind byte values.
pub(crate) use crate::trace_types::TraceKind;

// ── Geometry / MAT records (emitted once at the start) ─────────────

/// Geometry parameters embedded in the trace file.
#[derive(Serialize)]
struct GeometryRecord {
    pub kind: &'static str,
    pub tool_radius: f64,
    pub boundary: Vec<(f64, f64)>,
    pub islands: Vec<Vec<(f64, f64)>>,
    pub seeds: Vec<Vec<(f64, f64)>>,
}

/// Lightweight serializable snapshot of the Medial Axis Transform.
///
/// Only the data needed for visualisation (nodes, clearances, edges, root)
/// — no LCA cache, no branches.
#[derive(Serialize)]
struct MatRecord {
    pub kind: &'static str,
    pub nodes: Vec<(f64, f64)>,
    pub clearances: Vec<f64>,
    pub edges: Vec<(u32, u32)>,
    pub root: u32,
}

impl From<&MedialAxis> for MatRecord {
    fn from(ma: &MedialAxis) -> Self {
        Self {
            kind: "mat",
            nodes: ma.nodes.iter().map(|n| (n.point.x, n.point.y)).collect(),
            clearances: ma.nodes.iter().map(|n| n.clearance).collect(),
            edges: ma
                .edges
                .iter()
                .map(|&(i, j)| (i as u32, j as u32))
                .collect(),
            root: ma.root as u32,
        }
    }
}

// ── TraceRecord ─────────────────────────────────────────────────────

/// Generic record header (same across all operations).
#[derive(Serialize, Clone, Debug)]
pub(super) struct TraceRecordHeader {
    pub kind: &'static str,
    pub status: u8,
    pub step_idx: u32,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub heading: f64,
    pub prev_x: f64,
    pub prev_y: f64,
    pub prev_z: f64,
    pub ops_len: u32,
}

/// Adaptive-clearing-specific payload nested inside every trace record.
#[derive(Serialize, Clone, Debug)]
pub(super) struct AdaptivePayload {
    pub iters: u32,
    pub smoothed_heading: f64,
    pub predicted_angle: f64,
    pub iteration_angle: f64,
    pub eng_angle: f64,
    pub eng_area: f64,
    pub eng_chord: f64,
    pub cut_area: f64,
    pub total_area: f64,
    pub remaining_area: f64,
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
    /// Index 0-4 = Direct, Frontier, Mat, AStar, ZHop.
    /// 0 = success / not tried.  See `routing::ROUTE_*` constants.
    pub route_strategy_details: [u8; 5],
    /// Position of the last resume point candidate (routing target).
    pub resume_point_x: f64,
    pub resume_point_y: f64,
    pub resume_point_z: f64,
    /// Per-strategy candidate positions (x, y, z).  None entries are stored
    /// as (NaN, NaN, NaN).
    pub resume_candidate_points: [(f64, f64, f64); 6],
}

/// Per-step trace record, serialised as MessagePack.
///
/// Generic fields live at the top level; operation-specific data is in
/// the nested `payload` map so that a non-adaptive reader never sees
/// keys like ``eng_angle`` or ``wall_hug_points``.
#[derive(Serialize, Clone, Debug)]
pub(super) struct TraceRecord {
    #[serde(flatten)]
    pub header: TraceRecordHeader,
    pub payload: AdaptivePayload,
}

impl TraceRecord {
    /// Build a record with the common tool-state fields filled in from
    /// their source objects.  Kind-specific fields (iters, eng_*, etc.)
    /// default to 0 / 0.0.
    pub fn from_tool_state(
        kind: &'static str,
        status: StepStatus,
        step_idx: u32,
        tool: &Tool,
        cleared: &ClearedArea,
        prev_pos: Point3D,
        ops_len: u32,
    ) -> Self {
        let status: u8 = match status {
            StepStatus::Ok => 0,
            StepStatus::BoundaryHit => 1,
            StepStatus::LostEngagement => 2,
            StepStatus::NoConvergence => 3,
        };
        Self {
            header: TraceRecordHeader {
                kind,
                status,
                step_idx,
                pos_x: tool.pos.x,
                pos_y: tool.pos.y,
                pos_z: tool.pos.z,
                heading: tool.heading,
                prev_x: prev_pos.x,
                prev_y: prev_pos.y,
                prev_z: prev_pos.z,
                ops_len,
            },
            payload: AdaptivePayload {
                iters: 0,
                smoothed_heading: tool.smoothed_heading(),
                predicted_angle: tool.raw_predictor(),
                iteration_angle: 0.0,
                eng_angle: 0.0,
                eng_area: 0.0,
                eng_chord: 0.0,
                cut_area: 0.0,
                total_area: cleared.total_area(),
                remaining_area: cleared.remaining_area(),
                resume_source: 0,
                route_source: 0,
                wall_hug_points: Vec::new(),
                wall_hug_segment_counts: Vec::new(),
                resume_strategy_reasons: [0u8; 6],
                resume_strategy_details: [0u8; 6],
                route_strategy_details: [0u8; 5],
                resume_point_x: 0.0,
                resume_point_y: 0.0,
                resume_point_z: 0.0,
                resume_candidate_points: [(f64::NAN, f64::NAN, f64::NAN); 6],
            },
        }
    }
}

// ── TraceRecorder ───────────────────────────────────────────────────

/// Adapter that owns an optional [`Tracer`] and exposes one-line methods
/// for each record type.  When `tracer` is `None` all methods are
/// no-ops — call sites in the orchestrator are unconditional.
pub(super) struct TraceRecorder {
    tracer: Option<Tracer>,
    step_idx: u32,
}

impl TraceRecorder {
    /// Create a recorder, opening the trace file when
    /// `opts.trace_path` is `Some`.  Emits geometry and MAT records
    /// immediately after the header.
    pub fn new(
        opts: &AdaptiveClearingOptions,
        cleared: &ClearedArea,
        mat: Option<&MedialAxis>,
    ) -> Self {
        let tracer = match &opts.trace_path {
            Some(path) => match Tracer::open(path) {
                Ok(mut t) => {
                    let boundary: Vec<(f64, f64)> = opts
                        .pocket_boundary
                        .iter()
                        .map(|p| (p.x, p.y))
                        .collect();
                    let islands: Vec<Vec<(f64, f64)>> = opts
                        .islands
                        .iter()
                        .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
                        .collect();
                    let seeds: Vec<Vec<(f64, f64)>> = cleared
                        .fragments()
                        .iter()
                        .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
                        .collect();
                    t.write(&GeometryRecord {
                        kind: "geometry",
                        tool_radius: opts.radius,
                        boundary,
                        islands,
                        seeds,
                    });
                    if let Some(ma) = mat {
                        t.write(&MatRecord::from(ma));
                    }
                    Some(t)
                }
                Err(e) => {
                    eprintln!("trace: failed to open {:?}: {}", path, e);
                    None
                }
            },
            None => None,
        };

        Self {
            tracer,
            step_idx: 1,
        }
    }

    /// Record the initial tool state.
    pub fn record_init(
        &mut self,
        tool: &Tool,
        cleared: &ClearedArea,
        prev_pos: Point3D,
        ops_len: u32,
        wall_hug_points: &[(f64, f64)],
        wall_hug_segment_counts: &[u32],
    ) {
        if let Some(ref mut tr) = self.tracer {
            let mut rec = TraceRecord::from_tool_state(
                "init",
                StepStatus::Ok,
                0,
                tool,
                cleared,
                prev_pos,
                ops_len,
            );
            rec.payload.wall_hug_points = wall_hug_points.to_vec();
            rec.payload.wall_hug_segment_counts =
                wall_hug_segment_counts.to_vec();
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
        prev_pos: Point3D,
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
        if let Some(ref mut tr) = self.tracer {
            let mut rec = TraceRecord::from_tool_state(
                "cut",
                status,
                self.step_idx,
                tool,
                cleared,
                prev_pos,
                ops_len,
            );
            rec.payload.iters = iters;
            rec.payload.iteration_angle = iteration_angle;
            rec.payload.eng_angle = eng_angle;
            rec.payload.eng_area = eng_area;
            rec.payload.eng_chord = eng_chord;
            rec.payload.cut_area = cut_area;
            rec.payload.wall_hug_points = wall_hug_points.to_vec();
            rec.payload.wall_hug_segment_counts =
                wall_hug_segment_counts.to_vec();
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
        prev_pos: Point3D,
        ops_len: u32,
        reasons: &[u8; 6],
        details: &[u8; 6],
        route_details: &[u8; 5],
        rp: Point3D,
        candidate_pts: &[(f64, f64, f64); 6],
        wall_hug_points: &[(f64, f64)],
        wall_hug_segment_counts: &[u32],
    ) {
        if let Some(ref mut tr) = self.tracer {
            let kind_str = match kind {
                TraceKind::ResumeStall => "resume_stall",
                TraceKind::ResumeStuck => "resume_stuck",
                _ => "resume",
            };
            let mut rec = TraceRecord::from_tool_state(
                kind_str,
                status,
                self.step_idx,
                tool,
                cleared,
                prev_pos,
                ops_len,
            );
            rec.payload.resume_source = resume_source;
            rec.payload.route_source = route_source;
            rec.payload.resume_strategy_reasons = *reasons;
            rec.payload.resume_strategy_details = *details;
            rec.payload.route_strategy_details = *route_details;
            rec.payload.resume_point_x = rp.x;
            rec.payload.resume_point_y = rp.y;
            rec.payload.resume_point_z = rp.z;
            rec.payload.resume_candidate_points = *candidate_pts;
            rec.payload.wall_hug_points = wall_hug_points.to_vec();
            rec.payload.wall_hug_segment_counts =
                wall_hug_segment_counts.to_vec();
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
        prev_pos: Point3D,
        ops_len: u32,
        reasons: &[u8; 6],
        details: &[u8; 6],
        route_details: &[u8; 5],
        rp: Point3D,
        candidate_pts: &[(f64, f64, f64); 6],
        wall_hug_points: &[(f64, f64)],
        wall_hug_segment_counts: &[u32],
    ) {
        if let Some(ref mut tr) = self.tracer {
            let mut rec = TraceRecord::from_tool_state(
                "exit",
                status,
                self.step_idx,
                tool,
                cleared,
                prev_pos,
                ops_len,
            );
            rec.payload.resume_strategy_reasons = *reasons;
            rec.payload.resume_strategy_details = *details;
            rec.payload.route_strategy_details = *route_details;
            rec.payload.resume_point_x = rp.x;
            rec.payload.resume_point_y = rp.y;
            rec.payload.resume_point_z = rp.z;
            rec.payload.resume_candidate_points = *candidate_pts;
            rec.payload.wall_hug_points = wall_hug_points.to_vec();
            rec.payload.wall_hug_segment_counts =
                wall_hug_segment_counts.to_vec();
            tr.write(&rec);
        }
    }

    /// Finalise the trace file.
    /// Write the toolpath block and finalise the trace file.
    pub fn finish(self, _ops: &Ops) {
        if let Some(mut t) = self.tracer {
            let _ = t.finish();
        }
    }
}

// ── Helpers (called from the recorder) ──────────────────────────────
