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
pub mod tool;
mod trace;

use crate::dbg_log;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::shape::arc::normalize_angle_signed;
use crate::geo::shape::polygon::get_polygon_area;
use crate::geo::shape::polygon::get_polygon_centroid;
use crate::ops::container::Ops;
use crate::ops::cut::step_adaptive;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::StepStatus;
use crate::ops::state::State;
use crate::prof::prof_report;
use crate::types::{Point, Polygon};
use prof_macros::prof;

use std::path::PathBuf;

#[cfg(debug_assertions)]
use crate::trace::Tracer;
use resume::{try_resume, MAX_RESUMES};
use tool::Tool;
#[cfg(debug_assertions)]
use trace::{RecordBuf, TraceKind};

// ── Named constants ────────────────────────────────────────────────────

/// Floor fraction of target cut-area-per-distance below which we treat
/// engagement as lost.
const ENGAGEMENT_FLOOR_FRAC: f64 = 0.01;
/// Maximum total steps before giving up (safety valve).
const MAX_TOTAL_STEPS: usize = 100_000;
/// Check progress every N successful steps.
const STUCK_CHECK_INTERVAL: usize = 100;
/// Minimum fraction of theoretical target cut-area throughput that the
/// cleared area must grow by during a progress window.
const STUCK_MIN_GROWTH_FACTOR: f64 = 0.15;

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
    let target_area_pd =
        target_area_per_distance(opts.radius, opts.advance, opts.step_length);

    // Medial Axis Transform of the pocket, used by the resume fallback
    // to route through cleared territory to the nearest uncleared region
    // (e.g. around an island).  Computed once; failures fall back to the
    // legacy centroid jump.
    let mat = MedialAxis::compute(
        &opts.pocket_boundary,
        &opts.islands,
        opts.radius * 0.5,
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
    // the toolpath file written by the tracer.
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
    let mut trace_step_idx: u32 = 1;
    #[cfg(debug_assertions)]
    {
        if let Some(ref mut tr) = tracer {
            let mut buf = RecordBuf::default();
            buf.status(StepStatus::Ok);
            buf.step_idx(0);
            buf.pos(tool.pos);
            buf.heading(tool.heading);
            buf.smoothed_heading(tool.smoothed_heading());
            buf.predicted_angle(tool.raw_predictor());
            buf.total_area(cleared.total_area());
            buf.remaining_area(
                cleared.remaining().iter().map(get_polygon_area).sum(),
            );
            buf.prev_pos(tool.pos);
            buf.ops_len(tp_len);
            tr.write(TraceKind::Init as u8, buf.pack());
        }
    }

    let mut prev_pos = tool.pos;
    let mut steps_since_batch: usize = 0;

    // Track the start of the current cutting segment.  Used by the
    // D-shape resume: on stall, travel back to segment_start, then
    // search for reengagement in the direction normal to the segment.
    let mut segment_start = tool.pos;
    let mut in_segment = false;

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

    let target_eng =
        2.0 * std::f64::consts::PI - 2.0 * (opts.advance / opts.radius).acos();
    let min_cut_area =
        opts.step_length * target_area_pd * ENGAGEMENT_FLOOR_FRAC;

    let mut iter: usize = 0;
    for _ in 0..MAX_TOTAL_STEPS {
        iter += 1;
        // Convergence: check that remaining uncut area is below
        // tolerance.  Use an inexpensive fragment-sum check first
        // and only pay for the full union+diff when it looks close.
        let frag_total = cleared.total_area();
        if frag_total >= valid_total - opts.area_tolerance && {
            let rem: f64 =
                cleared.remaining().iter().map(get_polygon_area).sum();
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
                    buf.remaining_area(
                        cleared.remaining().iter().map(get_polygon_area).sum(),
                    );
                    buf.prev_pos(prev_pos);
                    buf.ops_len(tp_len);
                    tr.write(TraceKind::Exit as u8, buf.pack());
                }
            }
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
        );
        let status = result.status;
        if result.status == StepStatus::Ok {
            let dir = Point::new(result.heading.cos(), result.heading.sin());
            tool.pos = result.next;
            tool.heading = result.heading;
            tool.push_gyro(dir);
            tool.push_angle(result.iteration_angle);
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
                                buf.remaining_area(
                                    cleared
                                        .remaining()
                                        .iter()
                                        .map(get_polygon_area)
                                        .sum(),
                                );
                                buf.prev_pos(prev_pos);
                                buf.ops_len(tp_len);
                                tr.write(TraceKind::Exit as u8, buf.pack());
                            }
                        }
                        break;
                    }
                    if try_resume(
                        cleared,
                        &mut ops,
                        &mut tool,
                        opts,
                        &valid_tool_area,
                        target_area_pd,
                        max_def,
                        target_eng,
                        min_cut_area,
                        mat.as_ref(),
                        last_resume_area,
                        segment_start,
                    ) {
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
                                buf.remaining_area(
                                    cleared
                                        .remaining()
                                        .iter()
                                        .map(get_polygon_area)
                                        .sum(),
                                );
                                buf.prev_pos(prev_pos);
                                buf.ops_len(tp_len);
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
                        step_count = 0;
                        in_segment = false;
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
                            buf.remaining_area(
                                cleared
                                    .remaining()
                                    .iter()
                                    .map(get_polygon_area)
                                    .sum(),
                            );
                            buf.prev_pos(prev_pos);
                            buf.ops_len(tp_len);
                            tr.write(TraceKind::Exit as u8, buf.pack());
                        }
                    }
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
                        buf.remaining_area(
                            cleared
                                .remaining()
                                .iter()
                                .map(get_polygon_area)
                                .sum(),
                        );
                        buf.prev_pos(prev_pos);
                        buf.ops_len(tp_len);
                        tr.write(TraceKind::Exit as u8, buf.pack());
                    }
                }
                break;
            }

            if try_resume(
                cleared,
                &mut ops,
                &mut tool,
                opts,
                &valid_tool_area,
                target_area_pd,
                max_def,
                target_eng,
                min_cut_area,
                mat.as_ref(),
                last_resume_area,
                segment_start,
            ) {
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
                        buf.remaining_area(
                            cleared
                                .remaining()
                                .iter()
                                .map(get_polygon_area)
                                .sum(),
                        );
                        buf.prev_pos(prev_pos);
                        buf.ops_len(tp_len);
                        tr.write(TraceKind::ResumeStall as u8, buf.pack());
                    }
                }
                trace_step_idx += 1;
                prev_pos = tool.pos;
                last_check_area = cleared.total_area();
                last_resume_area = cleared.total_area();
                step_count = 0;
                in_segment = false;
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
                    buf.remaining_area(
                        cleared.remaining().iter().map(get_polygon_area).sum(),
                    );
                    buf.prev_pos(prev_pos);
                    buf.ops_len(tp_len);
                    tr.write(TraceKind::Exit as u8, buf.pack());
                }
            }
            break;
        }

        // Emit cutting move.
        if !in_segment {
            segment_start = prev_pos;
            in_segment = true;
        }
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
                buf.remaining_area(
                    cleared.remaining().iter().map(get_polygon_area).sum(),
                );
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
