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

pub mod resume;
mod resume_boundary;
mod resume_mat;
mod resume_segment;
mod resume_wall_hug;
pub mod tool;
#[cfg(debug_assertions)]
mod trace;

use crate::dbg_log;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::shape::arc::normalize_angle_signed;
use crate::geo::shape::polygon::get_polygon_area;
use crate::geo::shape::polygon::get_polygon_centroid;
use crate::geo::shape::polygon::get_polygons_closest_point;
use crate::ops::container::Ops;
use crate::ops::cut::step_adaptive;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::CutDirection;
use crate::ops::cut::StepStatus;
use crate::ops::cut::ToolPose;
use crate::ops::state::State;
use crate::prof::prof_report;
use crate::types::{Point, Polygon};
use prof_macros::prof;

use std::path::PathBuf;

#[cfg(debug_assertions)]
use crate::trace::Tracer;
use resume::{try_resume, ResumeCtx, MAX_RESUMES};
use tool::Tool;
#[cfg(debug_assertions)]
use trace::{RecordBuf, TraceKind};

// ── Named constants ────────────────────────────────────────────────────

/// Maximum total steps before giving up (safety valve).
const MAX_TOTAL_STEPS: usize = 100_000;
/// Check progress every N successful steps.
const STUCK_CHECK_INTERVAL: usize = 100;
/// Minimum fraction of theoretical target cut-area throughput that the
/// cleared area must grow by during a progress window.
const STUCK_MIN_GROWTH_FACTOR: f64 = 0.15;

/// Fraction of tool radius used as the departure threshold for wall-hug
/// tracking.  When the tool's distance to the nearest envelope boundary
/// exceeds `radius × WALL_HUG_DEPARTURE_FRAC`, the tool is considered
/// to have left the wall.
const WALL_HUG_DEPARTURE_FRAC: f64 = 0.1;

/// Minimum distance from `point` to the nearest boundary edge of any
/// polygon in `area`.  Used to detect whether the tool is "on" the
/// envelope (distance ≈ 0) or has departed into the interior.
#[prof]
fn envelope_distance(point: Point, area: &[Polygon]) -> f64 {
    get_polygons_closest_point(area, point)
        .map(|(_, _, _, d2)| d2.sqrt())
        .unwrap_or(f64::MAX)
}

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
) -> Ops {
    // ── 1. Pre-process ────────────────────────────────────────────
    let (valid_tool_area, valid_total) =
        compute_inset_region(&opts.pocket_boundary, opts.radius, &opts.islands);
    if valid_tool_area.is_empty() || valid_total <= opts.area_tolerance {
        return Ops::new();
    }
    if cleared.is_empty() {
        return Ops::new();
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

    // Counter for moving commands (move_to + line_to) only — matches
    // the toolpath file written by the tracer.  Only read inside the
    // `#[cfg(debug_assertions)]` trace-record blocks; the assignments
    // are kept unconditional for code-layout simplicity.
    let mut tp_len: u32 = 1; // the initial move_to above

    // Helper: recount moving commands from ops (used after try_resume
    // which may emit multiple move_to calls).
    fn moving_count(ops: &Ops) -> u32 {
        (0..ops.len())
            .filter(|&i| ops.is_travel(i) || ops.is_cutting(i))
            .count() as u32
    }

    #[cfg(debug_assertions)]
    let mut tracer: Option<Tracer> = match &opts.trace_path {
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
    // Trace-record step index.  Only read inside the
    // `#[cfg(debug_assertions)]` trace-record blocks; kept
    // unconditional so the surrounding code layout is identical.
    let mut trace_step_idx: u32 = 1;
    #[cfg(debug_assertions)]
    {
        if let Some(ref mut tr) = tracer {
            tr.write_mat(mat.as_ref().map(|m| m.into()));
            let mut buf = RecordBuf::default();
            buf.status(StepStatus::Ok);
            buf.step_idx(0);
            buf.pos(tool.pos);
            buf.heading(tool.heading);
            buf.smoothed_heading(tool.smoothed_heading());
            buf.predicted_angle(tool.raw_predictor());
            buf.total_area(cleared.total_area());
            buf.remaining_area(cleared.remaining_area());
            buf.prev_pos(tool.pos);
            buf.ops_len(tp_len);
            tr.write(TraceKind::Init as u8, buf.pack());
        }
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

    // Wall-hug tracking: after a boundary resume the tool sits on the
    // envelope edge.  We track when it departs so ResumeWallHug can
    // resume from the departure point instead of re-cutting from the
    // original envelope position.
    let mut near_envelope = false;
    let mut left_envelope = false;
    let mut last_wall_hug: Option<ToolPose> = None;

    // Stuck detection: every STUCK_CHECK_INTERVAL steps, verify the
    // cleared area has grown by at least the expected amount.  If not,
    // the solver is oscillating in a corner and we trigger resume /
    // re-engagement.  Using area growth (rather than displacement)
    // allows the initial inner contraction spiral to keep cutting.
    let mut step_count: usize = 0;
    let mut last_check_area: f64 = cleared.total_area();
    let mut resume_count: usize = 0;
    // Cleared area recorded at the last resume; used to detect a
    // stall-bounce where the frontier search returns a new (but
    // immediately-stalling) position without any actual cutting.
    let mut last_resume_area: f64 = -1.0;
    let mut last_resume_pos = tool.pos;

    #[cfg(debug_assertions)]
    macro_rules! write_exit_trace {
        ($status:expr) => {
            if let Some(ref mut tr) = tracer {
                let mut buf = RecordBuf::default();
                buf.status($status);
                buf.step_idx(trace_step_idx);
                buf.pos(tool.pos);
                buf.heading(tool.heading);
                buf.smoothed_heading(tool.smoothed_heading());
                buf.predicted_angle(tool.raw_predictor());
                buf.total_area(cleared.total_area());
                buf.remaining_area(cleared.remaining_area());
                buf.prev_pos(prev_pos);
                buf.ops_len(tp_len);
                tr.write(TraceKind::Exit as u8, buf.pack());
            }
        };
    }

    let mut iter: usize = 0;
    for _ in 0..MAX_TOTAL_STEPS {
        iter += 1;
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
                step_count,
                resume_count,
                iter,
                frag_total,
                valid_total,
            );
            #[cfg(debug_assertions)]
            write_exit_trace!(StepStatus::Ok);
            break;
        }

        let heading = tool.smoothed_heading();
        let predicted = tool.predicted_angle(max_def);
        let result = step_adaptive(
            cleared,
            tool.pos,
            heading,
            predicted,
            target_area_pd,
            opts.step_length,
            opts.radius,
            max_def,
            &valid_tool_area,
            -std::f64::consts::FRAC_PI_4,
            std::f64::consts::FRAC_PI_4,
            dir_sign,
        );
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
            if result.iters < 20 {
                tool.update_predictor(result.iteration_angle);
            }
        }

        // Wall-hug tracking: continuously monitor the tool's proximity
        // to the envelope edge.  When the tool departs the wall, record
        // the last on-wall pose so ResumeWallHug can resume from there
        // — regardless of how or when the tool reached the wall (not
        // only after a boundary resume).
        if status == StepStatus::Ok {
            let dist = envelope_distance(tool.pos, &valid_tool_area);
            let on_envelope = dist <= opts.radius * WALL_HUG_DEPARTURE_FRAC;
            if on_envelope {
                if left_envelope {
                    // Tool returned to the wall — reset so a new
                    // departure can be recorded.
                    left_envelope = false;
                    last_wall_hug = None;
                }
                near_envelope = true;
            } else if near_envelope && !left_envelope {
                left_envelope = true;
                last_wall_hug = Some(ToolPose {
                    pos: prev_pos,
                    heading: tool.heading,
                });
                dbg_log!(
                    "  WALL_HUG  departed at ({:.3},{:.3})  \
                     last_hug=({:.3},{:.3})  dist={:.4}  heading={:.4}",
                    tool.pos.x,
                    tool.pos.y,
                    prev_pos.x,
                    prev_pos.y,
                    dist,
                    tool.heading,
                );
            }
        }

        let stalled = status != StepStatus::Ok;

        // Stuck detection: check every STUCK_CHECK_INTERVAL steps
        // whether the tool is expanding the cleared pocket.  Area
        // growth is more robust than displacement: a contracting
        // inner spiral still removes material and should not be
        // confused with wall oscillation.
        if !stalled {
            step_count += 1;
            if step_count.is_multiple_of(STUCK_CHECK_INTERVAL) {
                let current_area = cleared.total_area();
                let growth = current_area - last_check_area;
                last_check_area = current_area;
                // Theoretical throughput: step_length * target_area_pd per step.
                let expected = STUCK_CHECK_INTERVAL as f64
                    * opts.step_length
                    * target_area_pd
                    * STUCK_MIN_GROWTH_FACTOR;
                if growth < expected {
                    // Tool is oscillating — force resume.
                    if steps_since_batch > 0 {
                        cleared.commit_batch_local();
                        steps_since_batch = 0;
                    }
                    resume_count += 1;
                    if resume_count > MAX_RESUMES {
                        dbg_log!(
                            "EXIT  reason=max_resumes(stuck)  step_count={}  \
                             resume_count={}  growth={:.3}  expected={:.3}  \
                             frag_total={:.3}  valid_total={:.3}",
                            step_count,
                            resume_count,
                            growth,
                            expected,
                            cleared.total_area(),
                            valid_total,
                        );
                        #[cfg(debug_assertions)]
                        write_exit_trace!(StepStatus::Ok);
                        break;
                    }
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
                            last_wall_hug,
                        };
                        try_resume(&ctx, &tool)
                    };
                    if let Some((_source, rp)) = result {
                        resume::emit_resume_travel(&mut ops, rp.pos, opts);
                        tool.pos = rp.pos;
                        tool.heading = rp.heading;
                        tool.reset_gyro();
                        tp_len = moving_count(&ops);
                        #[cfg(debug_assertions)]
                        {
                            if let Some(ref mut tr) = tracer {
                                let mut buf = RecordBuf::default();
                                buf.status(StepStatus::Ok);
                                buf.step_idx(trace_step_idx);
                                buf.pos(tool.pos);
                                buf.heading(tool.heading);
                                buf.smoothed_heading(tool.smoothed_heading());
                                buf.predicted_angle(tool.raw_predictor());
                                buf.total_area(cleared.total_area());
                                buf.remaining_area(cleared.remaining_area());
                                buf.prev_pos(prev_pos);
                                buf.ops_len(tp_len);
                                buf.resume_source(_source as u8);
                                tr.write(
                                    TraceKind::ResumeStuck as u8,
                                    buf.pack(),
                                );
                            }
                        }
                        trace_step_idx += 1;
                        prev_pos = tool.pos;
                        last_check_area = cleared.total_area();
                        last_resume_area = cleared.total_area();
                        last_resume_pos = tool.pos;
                        segment_start = ToolPose {
                            pos: tool.pos,
                            heading: tool.heading,
                        };
                        let ed = envelope_distance(tool.pos, &valid_tool_area);
                        near_envelope =
                            ed < opts.radius * WALL_HUG_DEPARTURE_FRAC;
                        left_envelope = false;
                        last_wall_hug = None;
                        step_count = 0;
                        continue;
                    }
                    dbg_log!(
                        "EXIT  reason=resume_failed(stuck)  step_count={}  \
                         resume_count={}  growth={:.3}  expected={:.3}  \
                         frag_total={:.3}  valid_total={:.3}",
                        step_count,
                        resume_count,
                        growth,
                        expected,
                        cleared.total_area(),
                        valid_total,
                    );
                    #[cfg(debug_assertions)]
                    write_exit_trace!(StepStatus::Ok);
                    break;
                }
            }
        }

        if stalled {
            if steps_since_batch > 0 {
                cleared.commit_batch_local();
                steps_since_batch = 0;
            }

            resume_count += 1;
            if resume_count > MAX_RESUMES {
                dbg_log!(
                    "EXIT  reason=max_resumes(stall)  step_count={}  \
                     resume_count={}  frag_total={:.3}  valid_total={:.3}",
                    step_count,
                    resume_count,
                    cleared.total_area(),
                    valid_total,
                );
                #[cfg(debug_assertions)]
                {
                    if let Some(ref mut tr) = tracer {
                        let mut buf = RecordBuf::default();
                        buf.status(StepStatus::Ok);
                        buf.step_idx(trace_step_idx);
                        buf.pos(tool.pos);
                        buf.heading(tool.heading);
                        buf.smoothed_heading(tool.smoothed_heading());
                        buf.predicted_angle(tool.raw_predictor());
                        buf.total_area(cleared.total_area());
                        buf.remaining_area(cleared.remaining_area());
                        buf.prev_pos(prev_pos);
                        buf.ops_len(tp_len);
                        tr.write(TraceKind::Exit as u8, buf.pack());
                    }
                }
                break;
            }

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
                    last_wall_hug,
                };
                try_resume(&ctx, &tool)
            };
            if let Some((_source, rp)) = result {
                resume::emit_resume_travel(&mut ops, rp.pos, opts);
                tool.pos = rp.pos;
                tool.heading = rp.heading;
                tool.reset_gyro();
                tp_len = moving_count(&ops);
                #[cfg(debug_assertions)]
                {
                    if let Some(ref mut tr) = tracer {
                        let mut buf = RecordBuf::default();
                        buf.status(status);
                        buf.step_idx(trace_step_idx);
                        buf.pos(tool.pos);
                        buf.heading(tool.heading);
                        buf.smoothed_heading(tool.smoothed_heading());
                        buf.predicted_angle(tool.raw_predictor());
                        buf.total_area(cleared.total_area());
                        buf.remaining_area(cleared.remaining_area());
                        buf.prev_pos(prev_pos);
                        buf.ops_len(tp_len);
                        buf.resume_source(_source as u8);
                        tr.write(TraceKind::ResumeStall as u8, buf.pack());
                    }
                }
                trace_step_idx += 1;
                prev_pos = tool.pos;
                last_check_area = cleared.total_area();
                last_resume_area = cleared.total_area();
                last_resume_pos = tool.pos;
                segment_start = ToolPose {
                    pos: tool.pos,
                    heading: tool.heading,
                };
                let ed = envelope_distance(tool.pos, &valid_tool_area);
                near_envelope = ed < opts.radius * WALL_HUG_DEPARTURE_FRAC;
                left_envelope = false;
                last_wall_hug = None;
                step_count = 0;
                continue;
            }

            dbg_log!(
                "EXIT  reason=resume_failed(stall)  step_count={}  \
                 resume_count={}  frag_total={:.3}  valid_total={:.3}",
                step_count,
                resume_count,
                cleared.total_area(),
                valid_total,
            );
            #[cfg(debug_assertions)]
            write_exit_trace!(status);
            break;
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
            cleared.compact_if_needed(0.5);
        }

        // Trace this cut step.
        #[cfg(debug_assertions)]
        {
            let eng = cleared.point_engagement(tool.pos, opts.radius);
            let ca = cleared.cut_area(prev_pos, tool.pos, opts.radius);
            if let Some(ref mut tr) = tracer {
                let mut buf = RecordBuf::default();
                buf.status(status);
                buf.step_idx(trace_step_idx);
                buf.iters(result.iters as u32);
                buf.pos(tool.pos);
                buf.heading(tool.heading);
                buf.smoothed_heading(tool.smoothed_heading());
                buf.predicted_angle(tool.raw_predictor());
                buf.iteration_angle(result.iteration_angle);
                buf.eng_angle(eng.angle);
                buf.eng_area(eng.area);
                buf.eng_chord(eng.chord_depth);
                buf.cut_area(ca);
                buf.total_area(cleared.total_area());
                buf.remaining_area(cleared.remaining_area());
                buf.prev_pos(prev_pos);
                buf.ops_len(tp_len);
                tr.write(TraceKind::Cut as u8, buf.pack());
            }
        }
        trace_step_idx += 1;

        prev_pos = tool.pos;
    }

    // Flush any remaining batch.
    if steps_since_batch > 0 {
        cleared.commit_batch_local();
    }

    #[cfg(debug_assertions)]
    {
        if let Some(mut t) = tracer.take() {
            t.write_toolpath(&trace::extract_toolpath(&ops));
            let _ = t.finish();
        }
    }

    ops
}

/// Wrapper around [`adaptive_clearing`] that prints a profiling report
/// to stderr when the `RAYGEO_PROFILE` environment variable is set.
#[prof]
pub fn adaptive_clearing_with_profile(
    cleared: &mut ClearedArea,
    opts: &AdaptiveClearingOptions,
    cut_state: &State,
    travel_state: &State,
) -> Ops {
    let result = adaptive_clearing(cleared, opts, cut_state);
    if std::env::var("RAYGEO_PROFILE").is_ok() {
        prof_report();
    }
    // travel_state is accepted for API compatibility but unused.
    let _ = travel_state;
    result
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
