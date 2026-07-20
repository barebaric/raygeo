//! Adaptive Clearing orchestrator (forward-stepping walking path).
//!
//! Drives a [`Tool`] forward in a single continuous spiral from the seed
//! clearing to the pocket wall.  The cleared area is expanded **per step**
//! so the tool naturally spirals outward: each step's capsule blocks the
//! backward direction, and the angular engagement solver — aided by the
//! tool's heading momentum — steers into fresh material.
//!
//! The caller is responsible for pre-populating the `ClearedArea`
//! with entry polygons before invoking this module.

mod chain;
pub mod resume;
mod resume_envelope;
mod resume_frontier;
mod resume_island;
mod resume_mat;
mod resume_segment;
mod resume_wall_hug;
pub mod routing;
mod routing_direct;
mod routing_frontier;
mod routing_mat;
mod routing_zhop;
mod stuck;
pub mod tool;
mod trace_helpers;
mod wallhug;

use crate::dbg_log;
use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::shape::arc::normalize_angle_signed;
use crate::geo::shape::polygon::{get_polygon_area, get_polygon_centroid};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::Tracelet;
use crate::ops::cut::step;
use crate::ops::cut::stepper::MAX_IT;
use crate::ops::cut::StepStatus;
use crate::ops::cut::StepperOptions;
use crate::ops::part::FaceState;
use crate::ops::state::State;
use crate::ops::types::CutDirection;
use crate::ops::types::ToolPose;
use crate::types::{Point3D, Polygon};
use prof_macros::prof;

use std::path::PathBuf;

use resume::{try_resume, ResumeCtx, MAX_RESUMES};
use tool::Tool;
use trace_helpers as th;
// ── Named constants ────────────────────────────────────────────────────

/// Maximum total steps before giving up (safety valve).
const MAX_TOTAL_STEPS: usize = 100_000;

// ── Spec ────────────────────────────────────────────────────────────

/// Spec for the adaptive-clearing assembler.
///
/// Carries every parameter the adaptive-clearing orchestrator needs
/// (tool geometry, step sizes, engagement thresholds, optional
/// callback for cancellation). Implements
/// [`Assembler`](crate::ops::assembly::Assembler) by delegating to
/// [`adaptive_clearing`].
#[derive(Clone, Debug)]
pub struct AdaptiveClearingSpec {
    pub tool_radius: f64,
    pub step_over: f64,
    pub step_length: f64,
    pub target_z: f64,
    pub safe_z: f64,
    pub max_deflection_deg: f64,
    pub wall_margin: f64,
    pub area_tolerance: f64,
    /// Rotational direction of all cutting moves for the run.
    /// Constrains the stepper's deflection range and tells resume
    /// strategies which way the frontier winds.
    pub cut_direction: CutDirection,
    /// Initial tool position.  When `None`, the starting position is
    /// auto-detected from the cleared-area frontier.
    pub start_pos: Option<Point3D>,
    /// Initial tool heading in radians.  When `None`, the heading is
    /// auto-detected as the CCW tangent at the starting position.
    pub start_heading: Option<f64>,
    /// How many steps to accumulate before committing cleared-area
    /// expansions.  Larger values reduce per‑step overhead at the cost
    /// of slightly stale engagement queries.  Default 20 is a good
    /// balance; reduce to 1 for best path quality.
    pub expansion_batch_size: usize,
    /// When set, write per-step trace records to this file for the
    /// Python inspector.
    pub trace_path: Option<PathBuf>,
    /// Tolerance for vertex simplification and clean-up (mm).
    /// Default 0.1.
    pub tolerance: f64,
    /// Optional callback checked periodically in the main loop.
    /// When it returns `true`, the operation is cancelled and a
    /// `RaygeoError::Cancelled` error is returned.
    pub cancel_check: Option<fn() -> bool>,
}

impl Default for AdaptiveClearingSpec {
    fn default() -> Self {
        Self {
            tool_radius: 3.0,
            step_over: 1.5,
            step_length: 0.6,
            target_z: -5.0,
            safe_z: 2.0,
            max_deflection_deg: 30.0,
            wall_margin: 0.0,
            area_tolerance: 1.0,
            cut_direction: CutDirection::Ccw,
            start_pos: None,
            start_heading: None,
            expansion_batch_size: 20,
            trace_path: None,
            tolerance: 0.1,
            cancel_check: None,
        }
    }
}

impl crate::ops::assembly::Assembler for AdaptiveClearingSpec {
    fn assemble(
        &self,
        ctx: &mut crate::ops::assembly::AssembleCtx,
    ) -> Result<crate::ops::assembly::AssemblyMeta, String> {
        ctx.callbacks
            .report_progress(0.0, "adaptive_clearing: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = adaptive_clearing(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks
            .report_progress(1.0, "adaptive_clearing: done");
        Ok(meta)
    }

    fn name(&self) -> &'static str {
        "adaptive_clearing"
    }

    fn cache_key_for_face(
        &self,
        face: &crate::ops::part::FaceState,
    ) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.tool_radius.to_bits().hash(&mut h);
        self.step_over.to_bits().hash(&mut h);
        self.step_length.to_bits().hash(&mut h);
        self.target_z.to_bits().hash(&mut h);
        self.safe_z.to_bits().hash(&mut h);
        self.max_deflection_deg.to_bits().hash(&mut h);
        self.wall_margin.to_bits().hash(&mut h);
        self.area_tolerance.to_bits().hash(&mut h);
        self.cut_direction.hash(&mut h);
        match self.start_pos {
            Some(p) => {
                1u8.hash(&mut h);
                p.x.to_bits().hash(&mut h);
                p.y.to_bits().hash(&mut h);
                p.z.to_bits().hash(&mut h);
            }
            None => 0u8.hash(&mut h),
        }
        match self.start_heading {
            Some(v) => {
                1u8.hash(&mut h);
                v.to_bits().hash(&mut h);
            }
            None => 0u8.hash(&mut h),
        }
        self.expansion_batch_size.hash(&mut h);
        self.tolerance.to_bits().hash(&mut h);
        if let Some(geo) = &face.geometry {
            geo.len().hash(&mut h);
        } else {
            0usize.hash(&mut h);
        }
        hash_polygons(&mut h, &face.stock_region.boundary);
        for island in &face.stock_region.islands {
            hash_polygons(&mut h, island);
        }
        for frag in face.cleared.fragments() {
            hash_polygons(&mut h, frag);
        }
        Some(h.finish())
    }

    fn restore_cache(
        &self,
        cached: &crate::ops::assembly::AssemblyOutput,
    ) -> Option<crate::ops::assembly::AssemblyOutput> {
        Some(cached.clone())
    }

    fn store_cache(
        &self,
        output: &crate::ops::assembly::AssemblyOutput,
    ) -> Option<crate::ops::assembly::AssemblyOutput> {
        Some(crate::ops::assembly::AssemblyOutput {
            ops: output.ops.copy(),
            is_scalable: output.is_scalable,
            source_dimensions: output.source_dimensions,
            cleared_fragments: output.cleared_fragments.clone(),
        })
    }
}

/// Hash a polygon's vertices into `h`.
fn hash_polygons<H: std::hash::Hasher>(h: &mut H, poly: &Polygon) {
    use std::hash::Hash;
    poly.len().hash(h);
    for p in poly {
        p.x.to_bits().hash(h);
        p.y.to_bits().hash(h);
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Target cut-area per unit distance for the engagement solver.
///
/// Computes the exact crescent area (`disk(c2) − disk(c1)`) that falls
/// beyond a straight wall at `wall_x = radius − advance`, then divides
/// by `step_length`.
///
/// The crescent height perpendicular to the step direction is:
/// * `step_length` for `|x| ≤ x_trans` (the overlapping cap)
/// * `2·sqrt(r²−x²)` for `|x| > x_trans` (the circular edges)
///
/// where `x_trans = sqrt(r² − step_length²/4)`.
///
/// Three cases arise depending on where `wall_x` sits relative to
/// `±x_trans`; each is evaluated via [`get_disk_segment_area`].
#[prof]
pub fn target_area_per_distance(
    radius: f64,
    advance: f64,
    step_length: f64,
) -> f64 {
    if step_length <= 0.0 || radius <= 0.0 {
        return advance.max(0.0);
    }
    let r = radius;
    let s = step_length;
    let wall_x = (r - advance).clamp(-r, r);
    let x_trans = (r * r - s * s * 0.25).max(0.0).sqrt();

    let area = if wall_x >= x_trans {
        // Wall is past the overlap cap: only the right circular edge
        // contributes.
        crate::geo::algo::engagement::get_disk_segment_area(wall_x, r)
    } else if wall_x >= -x_trans {
        // Wall cuts through the overlap cap: constant-height middle
        // plus right circular edge.
        s * (x_trans - wall_x)
            + crate::geo::algo::engagement::get_disk_segment_area(x_trans, r)
    } else {
        // Wall is in the left circular edge: left edge + middle + right.
        let left =
            crate::geo::algo::engagement::get_disk_segment_area(wall_x, r)
                - crate::geo::algo::engagement::get_disk_segment_area(
                    -x_trans, r,
                );
        let middle = 2.0 * s * x_trans;
        let right =
            crate::geo::algo::engagement::get_disk_segment_area(x_trans, r);
        left + middle + right
    };

    area / s
}

// ── Main entry point ─────────────────────────────────────────────────

enum StallResult {
    Applied,
    Exit,
    Failed { all_blacklisted: bool },
}

struct StallState<'a> {
    face: &'a mut FaceState,
    trace: &'a mut Tracelet,
    tool: &'a mut Tool,
    hug_tracker: &'a mut wallhug::WallHugTracker,
    stuck_detector: &'a mut stuck::StuckDetector,
    prev_pos: &'a mut Point3D,
    steps_since_batch: &'a mut usize,
    segment_start: &'a mut ToolPose,
    last_resume_area: &'a mut f64,
    last_resume_pos: &'a mut Point3D,
    resume_blacklist: &'a mut Vec<Point3D>,
    resume_count: &'a mut usize,
    resume_reasons: &'a mut resume::ResumeReasons,
    resume_details: &'a mut resume::ResumeReasons,
    route_details: &'a mut [u8; 4],
    last_resume_point: &'a mut Point3D,
    resume_candidate_pts: &'a mut resume::ResumeCandidatePoints,
}

#[allow(clippy::too_many_arguments)]
fn handle_stall(
    s: &mut StallState,
    stalled: bool,
    stuck_triggered: bool,
    _status: StepStatus,
    growth: f64,
    expected: f64,
    iter: usize,
    opts: &AdaptiveClearingSpec,
    step_opts: &StepperOptions,
    advance: f64,
    step_length: f64,
    mat: Option<&MedialAxis>,
    valid_total: f64,
) -> RaygeoResult<StallResult> {
    // Stall path: commit batch + clear blacklist (stuck already did).
    if stalled {
        if *s.steps_since_batch > 0 {
            s.face.cleared.commit_batch_local();
            *s.steps_since_batch = 0;
        }
        if s.face.cleared.total_area() - *s.last_resume_area > 0.0 {
            s.resume_blacklist.clear();
        }
    }

    *s.resume_count += 1;
    if *s.resume_count > MAX_RESUMES {
        if stuck_triggered {
            dbg_log!(
                "EXIT  reason=max_resumes(stuck)  step_count={}  \
                 resume_count={}  growth={:.3}  expected={:.3}  \
                 frag_total={:.3}  valid_total={:.3}",
                s.stuck_detector.step_count(),
                *s.resume_count,
                growth,
                expected,
                s.face.cleared.total_area(),
                valid_total,
            );
        } else {
            dbg_log!(
                "EXIT  reason=max_resumes(stall)  step_count={}  \
                 resume_count={}  frag_total={:.3}  valid_total={:.3}",
                s.stuck_detector.step_count(),
                *s.resume_count,
                s.face.cleared.total_area(),
                valid_total,
            );
        }
        let whp_exit = s.hug_tracker.wall_hug_ref();
        let wsc_exit = s.hug_tracker.segment_counts_ref();
        s.trace.exit(
            th::make_tool_snapshot(s.tool, *s.prev_pos),
            Some(th::resume_exit_meta(
                &s.face.cleared,
                &s.face.stock_region,
                s.resume_reasons,
                s.resume_details,
                s.route_details,
                *s.last_resume_point,
                s.resume_candidate_pts,
                &whp_exit,
                &wsc_exit,
                None,
                None,
            )),
        );
        return Ok(StallResult::Exit);
    }

    *s.resume_reasons = resume::ResumeReasons::default();
    let mut resume_done = false;
    'resume: loop {
        let wall_hug_flat = s.hug_tracker.ordered_points();
        let result = {
            let ctx = ResumeCtx {
                opts,
                step_opts,
                part: &*s.face,
                advance,
                step_length,
                mat,
                segment_start: *s.segment_start,
                last_resume_area: *s.last_resume_area,
                last_resume_pos: *s.last_resume_pos,
                wall_hug_points: &wall_hug_flat,
                blacklist: s.resume_blacklist,
            };
            try_resume(
                &ctx,
                s.tool,
                s.resume_reasons,
                s.resume_details,
                s.resume_candidate_pts,
            )
        };
        let (source, rp) = match result {
            Some(r) => r,
            None => break 'resume,
        };
        *s.last_resume_point = rp.pos;
        s.resume_blacklist.push(rp.pos);
        match resume::emit_resume_travel(
            s.trace,
            &s.face.cleared,
            mat,
            s.tool.pos,
            rp.pos,
            &*s.face,
            opts,
            Some(s.route_details),
        ) {
            Ok(route_source) => {
                let resume_source_u8 = source as u8;
                let route_source_u8 = route_source as u8;
                s.tool.pos = rp.pos;
                s.tool.heading = rp.heading;
                s.tool.reset_gyro();

                let whp = s.hug_tracker.wall_hug_ref();
                let wsc = s.hug_tracker.segment_counts_ref();
                *s.prev_pos = s.tool.pos;
                s.stuck_detector.reset(s.face.cleared.total_area());
                *s.last_resume_area = s.face.cleared.total_area();
                *s.last_resume_pos = s.tool.pos;
                *s.segment_start = ToolPose {
                    pos: s.tool.pos,
                    heading: s.tool.heading,
                };
                s.hug_tracker.reset();
                s.trace.resume(
                    th::make_tool_snapshot(s.tool, *s.prev_pos),
                    Some(th::resume_exit_meta(
                        &s.face.cleared,
                        &s.face.stock_region,
                        s.resume_reasons,
                        s.resume_details,
                        s.route_details,
                        *s.last_resume_point,
                        s.resume_candidate_pts,
                        &whp,
                        &wsc,
                        Some(resume_source_u8),
                        Some(route_source_u8),
                    )),
                );
                resume_done = true;
                break 'resume;
            }
            Err(RaygeoError::RoutingError(_)) => {
                // Candidate blacklisted — inner retry loop will
                // try the next resume candidate.
            }
            Err(other) => return Err(other),
        }
    }
    if resume_done {
        return Ok(StallResult::Applied);
    }

    // Commit any remaining batch so convergence check is accurate.
    if *s.steps_since_batch > 0 {
        s.face.cleared.commit_batch_local();
    }

    // If the pocket is effectively converged, exit normally.
    // `actionable_remaining` is the residual inside the actionable
    // zone (boundary inset by step_length), excluding slivers the
    // stepper cannot productively engage with.
    let clipped_remaining: f64 = s
        .face
        .cleared
        .actionable_remaining(&s.face.stock_region, step_length * 1.5);
    if clipped_remaining < opts.area_tolerance {
        dbg_log!(
            "EXIT  reason=converged(close-enough)  step_count={}  \
             resume_count={}  iter={}  frag_total={:.3}  \
             valid_total={:.3}",
            s.stuck_detector.step_count(),
            *s.resume_count,
            iter,
            s.face.cleared.total_area(),
            valid_total,
        );
        let whp_exit = s.hug_tracker.wall_hug_ref();
        let wsc_exit = s.hug_tracker.segment_counts_ref();
        s.trace.exit(
            th::make_tool_snapshot(s.tool, *s.prev_pos),
            Some(th::resume_exit_meta(
                &s.face.cleared,
                &s.face.stock_region,
                s.resume_reasons,
                s.resume_details,
                s.route_details,
                *s.last_resume_point,
                s.resume_candidate_pts,
                &whp_exit,
                &wsc_exit,
                None,
                None,
            )),
        );
        return Ok(StallResult::Exit);
    }

    if stuck_triggered {
        dbg_log!(
            "EXIT  reason=resume_failed(stuck)  step_count={}  \
             resume_count={}  growth={:.3}  expected={:.3}  \
             frag_total={:.3}  valid_total={:.3}",
            s.stuck_detector.step_count(),
            *s.resume_count,
            growth,
            expected,
            s.face.cleared.total_area(),
            valid_total,
        );
    } else {
        dbg_log!(
            "EXIT  reason=resume_failed(stall)  step_count={}  \
             resume_count={}  frag_total={:.3}  valid_total={:.3}",
            s.stuck_detector.step_count(),
            *s.resume_count,
            s.face.cleared.total_area(),
            valid_total,
        );
    }
    let whp_exit = s.hug_tracker.wall_hug_ref();
    let wsc_exit = s.hug_tracker.segment_counts_ref();
    s.trace.exit(
        th::make_tool_snapshot(s.tool, *s.prev_pos),
        Some(th::resume_exit_meta(
            &s.face.cleared,
            &s.face.stock_region,
            s.resume_reasons,
            s.resume_details,
            s.route_details,
            *s.last_resume_point,
            s.resume_candidate_pts,
            &whp_exit,
            &wsc_exit,
            None,
            None,
        )),
    );
    let all_blacklisted =
        s.resume_reasons.contains(&resume::REASON_BLACKLISTED);
    Ok(StallResult::Failed { all_blacklisted })
}

#[prof]
#[allow(unused_assignments, unused_variables)]
pub fn adaptive_clearing(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &AdaptiveClearingSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let pocket_boundary = face.stock_region.boundary.clone();
    let islands = face.stock_region.islands.clone();

    // ── 1. Pre-process ────────────────────────────────────────────
    let (valid_tool_area, valid_total) =
        compute_inset_region(&pocket_boundary, opts.tool_radius, &islands);
    if valid_tool_area.is_empty() || valid_total <= opts.area_tolerance {
        return Ok(AssemblyMeta {
            start: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
            end: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
        });
    }
    if face.cleared.is_empty() {
        return Ok(AssemblyMeta {
            start: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
            end: ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            },
        });
    }

    let advance = opts.step_over;
    let step_length = opts.step_length;

    let max_def = opts.max_deflection_deg.to_radians();
    let dir_sign = opts.cut_direction.sign();
    let target_area_pd =
        target_area_per_distance(opts.tool_radius, advance, step_length);

    // Medial Axis Transform of the pocket, used by the resume fallback
    // to route through cleared territory to the nearest uncleared region
    // (e.g. around an island).  Computed once; failures fall back to the
    // legacy centroid jump.
    let mat = MedialAxis::compute(
        &pocket_boundary,
        &islands,
        opts.tool_radius,
        opts.tool_radius.max(2.0),
    )
    .ok();

    // ── 2. Initialise the tool ───────────────────────────────────
    let centre = face
        .cleared
        .fragments()
        .iter()
        .max_by(|a, b| {
            let aa = get_polygon_area(a);
            let ab = get_polygon_area(b);
            crate::utils::sort_f64(ab, aa)
        })
        .map(get_polygon_centroid)
        .unwrap_or(crate::types::Point::ZERO);

    // Use caller-provided position/heading when available (e.g. the
    // tool is already in motion after an entry strategy).  Otherwise
    // auto-detect from the cleared-area frontier.
    let frontier = face.cleared.frontier(&face.stock_region, 0.5);
    let (default_pos, default_heading) =
        initial_pose(&frontier, centre, opts.target_z);
    let start_pos = opts.start_pos.unwrap_or(default_pos);
    let start_heading = opts.start_heading.unwrap_or(default_heading);

    let mut tool = Tool::new(start_pos, start_heading, opts.tool_radius);

    dbg_log!(
        "INIT  frag_count={}  frag_total={:.3}  valid_total={:.3}  \
         start=({:.3},{:.3})  heading={:.4}  target_apd={:.4}",
        face.cleared.len(),
        face.cleared.total_area(),
        valid_total,
        start_pos.x,
        start_pos.y,
        start_heading,
        target_area_pd,
    );

    // ── 3. Tracelet — span attrs + init event ─────────────────────

    trace.set_attrs(th::build_attrs(
        opts,
        &pocket_boundary,
        &islands,
        face.cleared.fragments(),
        mat.as_ref(),
    ));
    trace.init(
        th::make_tool_snapshot(&tool, tool.pos),
        Some(th::init_meta(&face.cleared, &face.stock_region)),
    );

    // ── 4. Continuous spiral: step → expand → repeat ─────────────

    trace.apply_state(cut_state);
    trace.move_to(tool.pos.x, tool.pos.y, opts.target_z, None);

    let mut prev_pos = tool.pos;
    let mut steps_since_batch: usize = 0;

    // Position and heading where the current cutting segment began.
    // Set on init and after each resume.  NOT updated on successful
    // steps — SegmentResume jumps here and probes using the original
    // heading, not the drifted per-step heading.
    let mut segment_start = ToolPose {
        pos: tool.pos,
        heading: tool.heading,
    };

    // Wall-hug tracking: enter when the tool cutting edge reaches
    // the pocket boundary (centre < radius from the envelope).
    // Track the minimum-distance pose per envelope visit, and
    // accumulate across all visits in the current cut segment so
    // the resume strategy can try earlier wall-hug points first.
    // Points from previous segments are preserved for fallback
    // resume attempts and pruned when the tool sweeps near them.
    let mut hug_tracker = wallhug::WallHugTracker::new();

    // Stuck detection: every STUCK_CHECK_INTERVAL steps, verify the
    // cleared area has grown by at least the expected amount.  If not,
    // the solver is oscillating in a corner and we trigger resume /
    // re-engagement.  Using area growth (rather than displacement)
    // allows the initial inner contraction spiral to keep cutting.
    let mut stuck_detector = stuck::StuckDetector::new(
        target_area_pd,
        step_length,
        face.cleared.total_area(),
    );
    let mut resume_count: usize = 0;
    // Cleared area recorded at the last resume; used to detect a
    // stall-bounce where the frontier search returns a new (but
    // immediately-stalling) position without any actual cutting.
    let mut last_resume_area: f64 = -1.0;
    let mut last_resume_pos = tool.pos;
    let mut resume_blacklist: Vec<Point3D> = Vec::new();

    // Options constructed once and reused for all step calls in the loop.
    let step_opts = StepperOptions {
        target_area_pd,
        step_length,
        radius: opts.tool_radius,
        max_deflection: max_def,
        valid_area: &valid_tool_area,
        dir_sign,
        ..Default::default()
    };

    let mut resume_reasons = resume::ResumeReasons::default();
    let mut resume_details = resume::ResumeReasons::default();
    let mut route_details = [0u8; 4];
    let mut last_resume_point = Point3D::ZERO;
    let mut resume_candidate_pts = resume::ResumeCandidatePoints::default();

    let mut iter: usize = 0;
    for _ in 0..MAX_TOTAL_STEPS {
        iter += 1;

        // Cancellation check — called every iteration so Ctrl+C is
        // responsive even in release builds.
        if let Some(check) = opts.cancel_check {
            if check() {
                trace.exit(
                    th::make_tool_snapshot(&tool, prev_pos),
                    Some(th::resume_exit_meta(
                        &face.cleared,
                        &face.stock_region,
                        &resume_reasons,
                        &resume_details,
                        &route_details,
                        last_resume_point,
                        &resume_candidate_pts,
                        &hug_tracker.wall_hug_ref(),
                        &hug_tracker.segment_counts_ref(),
                        None,
                        None,
                    )),
                );
                return Err(RaygeoError::Cancelled);
            }
        }

        // Convergence: check that actionable uncut area (inside the
        // tool-centre envelope) is below tolerance.  An inexpensive
        // fragment-sum check gates the more expensive
        // envelope-vs-fragments intersection.
        //
        // `actionable_remaining` excludes the wall band (the strip
        // between the stock outline and the tool-centre envelope):
        // the tool cannot reach that material with its centre, so it
        // should never block convergence even if `remaining_area`
        // is non-zero.  Wall-band slivers are the residual the
        // stepper chases without making progress.
        let frag_total = face.cleared.total_area();
        if frag_total >= valid_total - opts.area_tolerance && {
            let rem = face
                .cleared
                .actionable_remaining(&face.stock_region, step_length * 1.5);
            rem < opts.area_tolerance
        } {
            dbg_log!(
                "EXIT  reason=converged  step_count={}  resume_count={}  \
                  iter={}  frag_total={:.3}  valid_total={:.3}",
                stuck_detector.step_count(),
                resume_count,
                iter,
                frag_total,
                valid_total,
            );
            break;
        }

        let heading = tool.smoothed_heading();
        let predicted = tool.predicted_angle(max_def);
        let result = step(
            &face.cleared,
            crate::types::Point::new(tool.pos.x, tool.pos.y),
            heading,
            predicted,
            &step_opts,
        );
        let status = result.status;
        if result.status == StepStatus::Ok {
            let dir = crate::types::Point::new(
                result.heading.cos(),
                result.heading.sin(),
            );
            tool.pos =
                Point3D::new(result.next.x, result.next.y, opts.target_z);
            tool.heading = result.heading;
            tool.push_gyro(dir);
            // Only feed the deflection back into the predictor when the
            // solver converged quickly (iters < MAX_IT).  A step that
            // exhausted all 20 iterations almost certainly picked an
            // overshoot `best_angle` to escape a stall — propagating
            // that as the next step's seed is what causes the
            // over-correct / snap-back oscillation (e.g. +39° then
            // +74°) that leaves scalloped leftover material.
            if result.iters < MAX_IT {
                tool.update_predictor(result.iteration_angle);
            }
        }

        // ── Wrong-side safehold ──────────────────────────────────
        if result.status == StepStatus::Ok {
            stuck::wrong_side_safehold(
                &face.cleared,
                dir_sign,
                crate::types::Point::new(prev_pos.x, prev_pos.y),
                crate::types::Point::new(tool.pos.x, tool.pos.y),
                opts.tool_radius,
                target_area_pd,
                step_length,
            );
        }
        // Wall-hug tracking: enter when the tool cutting edge
        // reaches the pocket boundary (centre < radius from
        // envelope).  Track the minimum distance while the
        // distance is decreasing, then record it the first time the
        // distance starts increasing (departure).
        if status == StepStatus::Ok {
            hug_tracker.on_step(
                tool.pos,
                tool.heading,
                opts.tool_radius,
                &valid_tool_area,
            );
        }
        hug_tracker.prune(tool.pos, opts.tool_radius);
        let stalled = status != StepStatus::Ok;

        // Stuck detection: check every STUCK_CHECK_INTERVAL steps
        // whether the tool is expanding the cleared pocket.  Area
        // growth is more robust than displacement: a contracting
        // inner spiral still removes material and should not be
        // confused with wall oscillation.
        let mut stuck_triggered = false;
        let mut growth = 0.0;
        let mut expected = 0.0;
        match stuck_detector.tick(face.cleared.total_area(), stalled) {
            stuck::StuckOutcome::Ok => {}
            stuck::StuckOutcome::Oscillating {
                growth: g,
                expected: e,
            } => {
                growth = g;
                expected = e;
                if steps_since_batch > 0 {
                    face.cleared.commit_batch_local();
                    steps_since_batch = 0;
                }
                if face.cleared.total_area() - last_resume_area > 0.0 {
                    resume_blacklist.clear();
                }
                stuck_triggered = true;
            }
        }

        if stalled || stuck_triggered {
            let mut stall = StallState {
                face,
                trace,
                tool: &mut tool,
                hug_tracker: &mut hug_tracker,
                stuck_detector: &mut stuck_detector,
                prev_pos: &mut prev_pos,
                steps_since_batch: &mut steps_since_batch,
                segment_start: &mut segment_start,
                last_resume_area: &mut last_resume_area,
                last_resume_pos: &mut last_resume_pos,
                resume_blacklist: &mut resume_blacklist,
                resume_count: &mut resume_count,
                resume_reasons: &mut resume_reasons,
                resume_details: &mut resume_details,
                route_details: &mut route_details,
                last_resume_point: &mut last_resume_point,
                resume_candidate_pts: &mut resume_candidate_pts,
            };
            match handle_stall(
                &mut stall,
                stalled,
                stuck_triggered,
                status,
                growth,
                expected,
                iter,
                opts,
                &step_opts,
                advance,
                step_length,
                mat.as_ref(),
                valid_total,
            )? {
                StallResult::Applied => continue,
                StallResult::Exit => break,
                StallResult::Failed { all_blacklisted } => {
                    if all_blacklisted {
                        return Err(RaygeoError::RoutingError(
                            "all resume candidates failed routing".into(),
                        ));
                    }
                    return Err(RaygeoError::ResumePointNotFound(
                        "all resume strategies failed".into(),
                    ));
                }
            }
        }

        // Emit cutting move.
        trace.line_to(tool.pos.x, tool.pos.y, opts.target_z, None);

        // Expand cleared area.
        if steps_since_batch == 0 {
            face.cleared.begin_batch();
        }
        face.cleared.expand_batched(
            crate::types::Point::new(prev_pos.x, prev_pos.y),
            crate::types::Point::new(tool.pos.x, tool.pos.y),
            opts.tool_radius,
        );
        steps_since_batch += 1;

        if steps_since_batch >= opts.expansion_batch_size {
            face.cleared.commit_batch_local();
            steps_since_batch = 0;
            face.cleared
                .compact_if_needed(&face.stock_region, opts.tolerance);
        }

        let eng = face.cleared.get_point_engagement(
            crate::types::Point::new(tool.pos.x, tool.pos.y),
            opts.tool_radius,
        );
        let ca = face.cleared.cut_area(
            crate::types::Point::new(prev_pos.x, prev_pos.y),
            crate::types::Point::new(tool.pos.x, tool.pos.y),
            opts.tool_radius,
        );
        trace.cut(
            th::make_tool_snapshot(&tool, prev_pos),
            Some(th::cut_meta(
                &tool,
                &face.cleared,
                &face.stock_region,
                result.iters as u32,
                eng.angle,
                eng.area,
                eng.chord_depth,
                ca,
                result.iteration_angle,
            )),
        );
        prev_pos = tool.pos;
        // Clear blacklist only when area growth since the last resume
        // exceeds the engagement at the current tool position.  Tiny
        // engagement-noise growth (a fraction of the tool disk overlapping
        // the boundary) will not clear it, preventing repeated same-point
        // resumes.
        let area_growth = face.cleared.total_area() - last_resume_area;
        if area_growth >= eng.area {
            resume_blacklist.clear();
        }
    }

    // Flush any remaining batch.
    if steps_since_batch > 0 {
        face.cleared.commit_batch_local();
    }

    trace.exit(
        th::make_tool_snapshot(&tool, prev_pos),
        Some(th::resume_exit_meta(
            &face.cleared,
            &face.stock_region,
            &resume_reasons,
            &resume_details,
            &route_details,
            last_resume_point,
            &resume_candidate_pts,
            &hug_tracker.wall_hug_ref(),
            &hug_tracker.segment_counts_ref(),
            None,
            None,
        )),
    );

    Ok(AssemblyMeta {
        start: ToolPose {
            pos: start_pos,
            heading: start_heading,
        },
        end: ToolPose {
            pos: tool.pos,
            heading: tool.heading,
        },
    })
}

// ── Initial pose ─────────────────────────────────────────────────────

#[prof]
fn initial_pose(
    frontier: &[Polygon],
    centre: crate::types::Point,
    z: f64,
) -> (Point3D, f64) {
    let mut best_poly: Option<&Polygon> = None;
    let mut best_area = 0.0f64;
    for poly in frontier {
        if poly.len() < 3 {
            continue;
        }
        let area = get_polygon_area(poly);
        if area > best_area {
            best_area = area;
            best_poly = Some(poly);
        }
    }

    let poly = match best_poly {
        Some(p) => p,
        None => return (Point3D::new(centre.x, centre.y, z), 0.0),
    };

    let pos = poly[0];
    let radial = pos - centre;
    let radial_angle = radial.y.atan2(radial.x);
    let tangent_angle =
        normalize_angle_signed(radial_angle + std::f64::consts::FRAC_PI_2);

    (Point3D::new(pos.x, pos.y, z), tangent_angle)
}
