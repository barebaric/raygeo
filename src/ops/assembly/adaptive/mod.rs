//! Adaptive Clearing orchestrator (forward-stepping walking path).
//!
//! Drives a [`Tool`] forward in a single continuous spiral from the seed
//! clearing to the pocket wall.  The cleared area is expanded **per step**
//! so the tool naturally spirals outward: each step's capsule blocks the
//! backward direction, and the angular engagement solver — aided by the
//! tool's heading momentum — steers into fresh material.
//!
//! The caller is responsible for pre-populating the `ClearedArea` with
//! entry polygons (e.g. via `adaptive_entry`).

mod chain;
pub mod resume;
mod resume_envelope;
mod resume_frontier;
mod resume_island;
mod resume_mat;
mod resume_segment;
mod resume_wall_hug;
pub mod routing;
mod routing_astar;
mod routing_direct;
mod routing_frontier;
mod routing_mat;
mod stuck;
pub mod tool;
mod trace;
mod wallhug;

use crate::dbg_log;
use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::shape::arc::normalize_angle_signed;
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_centroid, get_polygon_signed_area,
    get_polygons_group_intersection,
};
use crate::ops::container::Ops;
use crate::ops::cut::step;
use crate::ops::cut::stepper::MAX_IT;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::CutDirection;
use crate::ops::cut::StepStatus;
use crate::ops::cut::StepperOptions;
use crate::ops::cut::ToolPose;
use crate::ops::state::State;
use crate::types::{Point, Polygon};
use prof_macros::prof;

use std::path::PathBuf;

use resume::{try_resume, ResumeCtx, MAX_RESUMES};
use tool::Tool;
use trace::TraceKind;
use trace::TraceRecorder;

// ── Named constants ────────────────────────────────────────────────────

/// Maximum total steps before giving up (safety valve).
const MAX_TOTAL_STEPS: usize = 100_000;

// ── Options ──────────────────────────────────────────────────────────

/// Options for [`adaptive_clearing`].
#[derive(Clone, Debug)]
pub struct AdaptiveClearingOptions {
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub radius: f64,
    pub advance: f64,
    pub cut_z: f64,
    pub safe_z: f64,
    pub step_length: f64,
    pub max_deflection_deg: f64,
    pub wall_margin: f64,
    pub area_tolerance: f64,
    /// Rotational direction of all cutting moves for the run.
    /// Constrains the stepper's deflection range and tells resume
    /// strategies which way the frontier winds.
    pub cut_direction: CutDirection,
    /// Initial tool position.  When `None`, the starting position is
    /// auto-detected from the cleared-area frontier.
    pub start_pos: Option<Point>,
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

impl Default for AdaptiveClearingOptions {
    fn default() -> Self {
        Self {
            pocket_boundary: Vec::new(),
            islands: Vec::new(),
            radius: 3.0,
            advance: 1.5,
            cut_z: -5.0,
            safe_z: 2.0,
            step_length: 0.6,
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
/// `±x_trans`; each is evaluated via [`disk_segment_area`].
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
        crate::geo::algo::engagement::disk_segment_area(wall_x, r)
    } else if wall_x >= -x_trans {
        // Wall cuts through the overlap cap: constant-height middle
        // plus right circular edge.
        s * (x_trans - wall_x)
            + crate::geo::algo::engagement::disk_segment_area(x_trans, r)
    } else {
        // Wall is in the left circular edge: left edge + middle + right.
        let left = crate::geo::algo::engagement::disk_segment_area(wall_x, r)
            - crate::geo::algo::engagement::disk_segment_area(-x_trans, r);
        let middle = 2.0 * s * x_trans;
        let right = crate::geo::algo::engagement::disk_segment_area(x_trans, r);
        left + middle + right
    };

    area / s
}

// ── Main entry point ─────────────────────────────────────────────────

#[prof]
#[allow(unused_assignments, unused_variables)]
pub fn adaptive_clearing(
    cleared: &mut ClearedArea,
    opts: &AdaptiveClearingOptions,
    cut_state: &State,
) -> RaygeoResult<Ops> {
    // ── 1. Pre-process ────────────────────────────────────────────
    let (valid_tool_area, valid_total) =
        compute_inset_region(&opts.pocket_boundary, opts.radius, &opts.islands);
    if valid_tool_area.is_empty() || valid_total <= opts.area_tolerance {
        return Ok(Ops::new());
    }
    if cleared.is_empty() {
        return Ok(Ops::new());
    }

    let max_def = opts.max_deflection_deg.to_radians();
    let dir_sign = opts.cut_direction.sign();
    let target_area_pd =
        target_area_per_distance(opts.radius, opts.advance, opts.step_length);

    // Medial Axis Transform of the pocket, used by the resume fallback
    // to route through cleared territory to the nearest uncleared region
    // (e.g. around an island).  Computed once; failures fall back to the
    // legacy centroid jump.
    let mat = MedialAxis::compute(
        &opts.pocket_boundary,
        &opts.islands,
        opts.radius,
        opts.radius.max(2.0),
    )
    .ok();

    // ── 2. Initialise the tool ───────────────────────────────────
    let centre = cleared
        .fragments()
        .iter()
        .max_by(|a, b| {
            let aa = get_polygon_area(a);
            let ab = get_polygon_area(b);
            ab.partial_cmp(&aa).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(get_polygon_centroid)
        .unwrap_or(Point::ZERO);

    // Use caller-provided position/heading when available (e.g. the
    // tool is already in motion after an entry strategy).  Otherwise
    // auto-detect from the cleared-area frontier.
    let frontier = cleared.frontier(0.5);
    let (default_pos, default_heading) = initial_pose(&frontier, centre);
    let start_pos = opts.start_pos.unwrap_or(default_pos);
    let start_heading = opts.start_heading.unwrap_or(default_heading);

    let mut tool = Tool::new(start_pos, start_heading, opts.radius);

    dbg_log!(
        "INIT  frag_count={}  frag_total={:.3}  valid_total={:.3}  \
         start=({:.3},{:.3})  heading={:.4}  target_apd={:.4}",
        cleared.len(),
        cleared.total_area(),
        valid_total,
        start_pos.x,
        start_pos.y,
        start_heading,
        target_area_pd,
    );

    // ── 3. Continuous spiral: step → expand → repeat ─────────────

    let mut ops = Ops::new();
    ops.apply_state(cut_state);
    ops.move_to(tool.pos.x, tool.pos.y, opts.cut_z, None);

    // Counter for moving commands (move_to + line_to) only — tracked
    // for the trace recorder's ops_len field.
    let mut tp_len: u32 = 1; // the initial move_to above

    // Helper: recount moving commands from ops (used after try_resume
    // which may emit multiple move_to calls).
    fn moving_count(ops: &Ops) -> u32 {
        (0..ops.len())
            .filter(|&i| ops.is_travel(i) || ops.is_cutting(i))
            .count() as u32
    }

    // Helper: convert resume candidate points to flat array for the
    // trace recorder.
    fn candidate_pts_as_flat(
        pts: &resume::ResumeCandidatePoints,
    ) -> [(f64, f64); 6] {
        std::array::from_fn(|i| match &pts[i] {
            Some(pt) => (pt.x, pt.y),
            None => (f64::NAN, f64::NAN),
        })
    }

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

    let mut recorder = TraceRecorder::new(opts, cleared);
    #[cfg(debug_assertions)]
    recorder.write_mat(mat.as_ref());
    recorder.record_init(
        &tool,
        cleared,
        tool.pos,
        tp_len,
        &hug_tracker.wall_hug_ref(),
        &hug_tracker.segment_counts_ref(),
    );

    // Stuck detection: every STUCK_CHECK_INTERVAL steps, verify the
    // cleared area has grown by at least the expected amount.  If not,
    // the solver is oscillating in a corner and we trigger resume /
    // re-engagement.  Using area growth (rather than displacement)
    // allows the initial inner contraction spiral to keep cutting.
    let mut stuck_detector = stuck::StuckDetector::new(
        target_area_pd,
        opts.step_length,
        cleared.total_area(),
    );
    let mut resume_count: usize = 0;
    // Cleared area recorded at the last resume; used to detect a
    // stall-bounce where the frontier search returns a new (but
    // immediately-stalling) position without any actual cutting.
    let mut last_resume_area: f64 = -1.0;
    let mut last_resume_pos = tool.pos;
    let mut resume_blacklist: Vec<Point> = Vec::new();

    // Options constructed once and reused for all step calls in the loop.
    let step_opts = StepperOptions {
        target_area_pd,
        step_length: opts.step_length,
        radius: opts.radius,
        max_deflection: max_def,
        valid_area: &valid_tool_area,
        dir_sign,
        ..Default::default()
    };

    let mut resume_reasons = resume::ResumeReasons::default();
    let mut resume_details = resume::ResumeReasons::default();
    let mut route_details = [0u8; 4];
    let mut last_resume_point = Point::ZERO;
    let mut resume_candidate_pts = resume::ResumeCandidatePoints::default();

    let mut iter: usize = 0;
    for _ in 0..MAX_TOTAL_STEPS {
        iter += 1;

        // Cancellation check — called every iteration so Ctrl+C is
        // responsive even in release builds.
        if let Some(check) = opts.cancel_check {
            if check() {
                recorder.record_exit(
                    StepStatus::Ok,
                    &tool,
                    cleared,
                    prev_pos,
                    tp_len,
                    &resume_reasons,
                    &resume_details,
                    &route_details,
                    last_resume_point,
                    &candidate_pts_as_flat(&resume_candidate_pts),
                    &hug_tracker.wall_hug_ref(),
                    &hug_tracker.segment_counts_ref(),
                );
                return Err(RaygeoError::Cancelled);
            }
        }

        // Convergence: check that remaining uncut area is below
        // tolerance.  Use an inexpensive fragment-sum check first
        // and only pay for the full union+diff when it looks close.
        let frag_total = cleared.total_area();
        if frag_total >= valid_total - opts.area_tolerance && {
            let rem = cleared.remaining_area();
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
            recorder.record_exit(
                StepStatus::Ok,
                &tool,
                cleared,
                prev_pos,
                tp_len,
                &resume_reasons,
                &resume_details,
                &route_details,
                last_resume_point,
                &candidate_pts_as_flat(&resume_candidate_pts),
                &hug_tracker.wall_hug_ref(),
                &hug_tracker.segment_counts_ref(),
            );
            break;
        }

        let heading = tool.smoothed_heading();
        let predicted = tool.predicted_angle(max_def);
        let result = step(cleared, tool.pos, heading, predicted, &step_opts);
        let status = result.status;
        if result.status == StepStatus::Ok {
            let dir = Point::new(result.heading.cos(), result.heading.sin());
            tool.pos = result.next;
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
                cleared,
                dir_sign,
                prev_pos,
                tool.pos,
                opts.radius,
                target_area_pd,
                opts.step_length,
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
                opts.radius,
                &valid_tool_area,
            );
        }
        hug_tracker.prune(tool.pos, opts.radius);
        let stalled = status != StepStatus::Ok;

        // Stuck detection: check every STUCK_CHECK_INTERVAL steps
        // whether the tool is expanding the cleared pocket.  Area
        // growth is more robust than displacement: a contracting
        // inner spiral still removes material and should not be
        // confused with wall oscillation.
        let mut stuck_triggered = false;
        let mut growth = 0.0;
        let mut expected = 0.0;
        match stuck_detector.tick(cleared.total_area(), stalled) {
            stuck::StuckOutcome::Ok => {}
            stuck::StuckOutcome::Oscillating {
                growth: g,
                expected: e,
            } => {
                growth = g;
                expected = e;
                if steps_since_batch > 0 {
                    cleared.commit_batch_local();
                    steps_since_batch = 0;
                }
                if cleared.total_area() - last_resume_area > 0.0 {
                    resume_blacklist.clear();
                }
                stuck_triggered = true;
            }
        }

        if stalled || stuck_triggered {
            // Stall path: commit batch + clear blacklist (stuck already did).
            if stalled {
                if steps_since_batch > 0 {
                    cleared.commit_batch_local();
                    steps_since_batch = 0;
                }
                if cleared.total_area() - last_resume_area > 0.0 {
                    resume_blacklist.clear();
                }
            }

            let resume_trace_status = if stuck_triggered {
                StepStatus::Ok
            } else {
                status
            };

            resume_count += 1;
            if resume_count > MAX_RESUMES {
                if stuck_triggered {
                    dbg_log!(
                        "EXIT  reason=max_resumes(stuck)  step_count={}  \
                         resume_count={}  growth={:.3}  expected={:.3}  \
                         frag_total={:.3}  valid_total={:.3}",
                        stuck_detector.step_count(),
                        resume_count,
                        growth,
                        expected,
                        cleared.total_area(),
                        valid_total,
                    );
                } else {
                    dbg_log!(
                        "EXIT  reason=max_resumes(stall)  step_count={}  \
                         resume_count={}  frag_total={:.3}  valid_total={:.3}",
                        stuck_detector.step_count(),
                        resume_count,
                        cleared.total_area(),
                        valid_total,
                    );
                }
                recorder.record_exit(
                    StepStatus::Ok,
                    &tool,
                    cleared,
                    prev_pos,
                    tp_len,
                    &resume_reasons,
                    &resume_details,
                    &route_details,
                    last_resume_point,
                    &candidate_pts_as_flat(&resume_candidate_pts),
                    &hug_tracker.wall_hug_ref(),
                    &hug_tracker.segment_counts_ref(),
                );
                break;
            }

            resume_reasons = resume::ResumeReasons::default();
            let mut resume_done = false;
            'resume: loop {
                let wall_hug_flat = hug_tracker.ordered_points();
                let result = {
                    let ctx = ResumeCtx {
                        cleared: &*cleared,
                        opts,
                        valid_tool_area: &valid_tool_area,
                        mat: mat.as_ref(),
                        target_area_pd,
                        segment_start,
                        last_resume_area,
                        last_resume_pos,
                        wall_hug_points: &wall_hug_flat,
                        blacklist: &resume_blacklist,
                    };
                    try_resume(
                        &ctx,
                        &tool,
                        &mut resume_reasons,
                        &mut resume_details,
                        &mut resume_candidate_pts,
                    )
                };
                let (source, rp) = match result {
                    Some(r) => r,
                    None => break 'resume,
                };
                last_resume_point = rp.pos;
                resume_blacklist.push(rp.pos);
                match resume::emit_resume_travel(
                    &mut ops,
                    &*cleared,
                    mat.as_ref(),
                    tool.pos,
                    rp.pos,
                    opts,
                    Some(&mut route_details),
                ) {
                    Ok(route_source) => {
                        tool.pos = rp.pos;
                        tool.heading = rp.heading;
                        tool.reset_gyro();
                        tp_len = moving_count(&ops);
                        let kind = if stuck_triggered {
                            TraceKind::ResumeStuck
                        } else {
                            TraceKind::ResumeStall
                        };
                        recorder.record_resume(
                            kind,
                            resume_trace_status,
                            source as u8,
                            route_source as u8,
                            &tool,
                            cleared,
                            prev_pos,
                            tp_len,
                            &resume_reasons,
                            &resume_details,
                            &route_details,
                            rp.pos,
                            &candidate_pts_as_flat(&resume_candidate_pts),
                            &hug_tracker.wall_hug_ref(),
                            &hug_tracker.segment_counts_ref(),
                        );
                        prev_pos = tool.pos;
                        stuck_detector.reset(cleared.total_area());
                        last_resume_area = cleared.total_area();
                        last_resume_pos = tool.pos;
                        segment_start = ToolPose {
                            pos: tool.pos,
                            heading: tool.heading,
                        };
                        hug_tracker.reset();
                        stuck_detector.reset(cleared.total_area());
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
                continue;
            }

            // Commit any remaining batch so convergence check is accurate.
            if steps_since_batch > 0 {
                cleared.commit_batch_local();
            }

            // If the pocket is effectively converged, exit normally.
            // The raw remaining may overcount area outside the valid
            // tool region, so clip against the inset boundary.
            let clipped_remaining: f64 = {
                let rem = cleared.remaining();
                if rem.is_empty() {
                    0.0
                } else {
                    let clipped =
                        get_polygons_group_intersection(&rem, &valid_tool_area);
                    clipped
                        .iter()
                        .map(|p| get_polygon_signed_area(p).max(0.0))
                        .sum::<f64>()
                }
            };
            if clipped_remaining < opts.area_tolerance {
                dbg_log!(
                    "EXIT  reason=converged(close-enough)  step_count={}  \
                     resume_count={}  iter={}  frag_total={:.3}  \
                     valid_total={:.3}",
                    stuck_detector.step_count(),
                    resume_count,
                    iter,
                    cleared.total_area(),
                    valid_total,
                );
                recorder.record_exit(
                    StepStatus::Ok,
                    &tool,
                    cleared,
                    prev_pos,
                    tp_len,
                    &resume_reasons,
                    &resume_details,
                    &route_details,
                    last_resume_point,
                    &candidate_pts_as_flat(&resume_candidate_pts),
                    &hug_tracker.wall_hug_ref(),
                    &hug_tracker.segment_counts_ref(),
                );
                break;
            }

            if stuck_triggered {
                dbg_log!(
                    "EXIT  reason=resume_failed(stuck)  step_count={}  \
                     resume_count={}  growth={:.3}  expected={:.3}  \
                     frag_total={:.3}  valid_total={:.3}",
                    stuck_detector.step_count(),
                    resume_count,
                    growth,
                    expected,
                    cleared.total_area(),
                    valid_total,
                );
            } else {
                dbg_log!(
                    "EXIT  reason=resume_failed(stall)  step_count={}  \
                     resume_count={}  frag_total={:.3}  valid_total={:.3}",
                    stuck_detector.step_count(),
                    resume_count,
                    cleared.total_area(),
                    valid_total,
                );
            }
            recorder.record_exit(
                resume_trace_status,
                &tool,
                cleared,
                prev_pos,
                tp_len,
                &resume_reasons,
                &resume_details,
                &route_details,
                last_resume_point,
                &candidate_pts_as_flat(&resume_candidate_pts),
                &hug_tracker.wall_hug_ref(),
                &hug_tracker.segment_counts_ref(),
            );
            let all_blacklisted =
                resume_reasons.contains(&resume::REASON_BLACKLISTED);
            if all_blacklisted {
                return Err(RaygeoError::RoutingError(
                    "all resume candidates failed routing".into(),
                ));
            }
            return Err(RaygeoError::ResumePointNotFound(
                "all resume strategies failed".into(),
            ));
        }

        // Emit cutting move.
        ops.line_to(tool.pos.x, tool.pos.y, opts.cut_z, None);
        tp_len += 1;

        // Expand cleared area.
        if steps_since_batch == 0 {
            cleared.begin_batch();
        }
        cleared.expand_batched(prev_pos, tool.pos, opts.radius);
        steps_since_batch += 1;

        if steps_since_batch >= opts.expansion_batch_size {
            cleared.commit_batch_local();
            steps_since_batch = 0;
            cleared.compact_if_needed(opts.tolerance);
        }

        let eng = cleared.point_engagement(tool.pos, opts.radius);
        let ca = cleared.cut_area(prev_pos, tool.pos, opts.radius);
        recorder.record_cut(
            status,
            &tool,
            cleared,
            prev_pos,
            tp_len,
            result.iters as u32,
            result.iteration_angle,
            eng.angle,
            eng.area,
            eng.chord_depth,
            ca,
            &hug_tracker.wall_hug_ref(),
            &hug_tracker.segment_counts_ref(),
        );

        prev_pos = tool.pos;
        // Clear blacklist only when area growth since the last resume
        // exceeds the engagement at the current tool position.  Tiny
        // engagement-noise growth (a fraction of the tool disk overlapping
        // the boundary) will not clear it, preventing repeated same-point
        // resumes.
        let area_growth = cleared.total_area() - last_resume_area;
        if area_growth >= eng.area {
            resume_blacklist.clear();
        }
    }

    // Flush any remaining batch.
    if steps_since_batch > 0 {
        cleared.commit_batch_local();
    }

    recorder.finish(&ops);

    Ok(ops)
}

// ── Initial pose ─────────────────────────────────────────────────────

#[prof]
fn initial_pose(frontier: &[Polygon], centre: Point) -> (Point, f64) {
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
        None => return (centre, 0.0),
    };

    let pos = poly[0];
    let radial = pos - centre;
    let radial_angle = radial.y.atan2(radial.x);
    let tangent_angle =
        normalize_angle_signed(radial_angle + std::f64::consts::FRAC_PI_2);

    (pos, tangent_angle)
}
