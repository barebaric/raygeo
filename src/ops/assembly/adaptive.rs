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

use crate::geo::algo::offset::compute_inset_region;
use crate::geo::shape::arc::normalize_angle_signed;
use crate::geo::shape::polygon::get_polygon_area;
use crate::geo::shape::polygon::get_polygon_centroid;
use crate::ops::container::Ops;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::StepStatus;
use crate::ops::cut::ToolPose;
use crate::ops::cut::{search_frontier_engagement, step_adaptive};
use crate::ops::state::State;
use crate::prof::prof_report;
use crate::types::{Point, Polygon};
use prof_macros::prof;

/// Set to `true` to enable verbose adaptive clearing debug logging.
const ADAPTIVE_DEBUG: bool = false;

macro_rules! dbg_log {
    ($($arg:tt)*) => {
        if ADAPTIVE_DEBUG {
            eprintln!($($arg)*);
        }
    };
}

// ── Named constants ────────────────────────────────────────────────

/// Floor fraction of target cut-area-per-distance below which we treat
/// engagement as lost.
const ENGAGEMENT_FLOOR_FRAC: f64 = 0.01;
/// Maximum total steps before giving up (safety valve).
const MAX_TOTAL_STEPS: usize = 100_000;
/// Number of recent direction vectors to average for heading smoothing.
const GYRO_BUFFER_LEN: usize = 5;
/// Number of recent iteration-angle deltas stored for the predictor.
const ANGLE_HISTORY_LEN: usize = 4;
/// Check progress every N successful steps.
const STUCK_CHECK_INTERVAL: usize = 100;
/// Minimum fraction of theoretical target cut-area throughput that the
/// cleared area must grow by during a progress window.
const STUCK_MIN_GROWTH_FACTOR: f64 = 0.15;
/// Maximum number of resume / re-engagement attempts before giving up.
const MAX_RESUMES: usize = 500;

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
    /// of slightly stale engagement queries.  Leave at 1 (default) for
    /// best path quality; increase to 5+ for faster roughing passes.
    pub expansion_batch_size: usize,
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
            expansion_batch_size: 1,
        }
    }
}

// ── Tool ─────────────────────────────────────────────────────────────

/// A cutting tool with persistent position and heading.
///
/// A short gyroscope buffer averages recent direction vectors so that
/// small engagement wiggles do not jerk the tool path.  A separate
/// history of recent solver deltas serves as a predictor.
#[derive(Clone, Copy, Debug)]
pub struct Tool {
    /// Tool centre position.
    pub pos: Point,
    /// Current heading angle (radians).
    pub heading: f64,
    /// Tool radius.
    pub radius: f64,
    /// Recent direction vectors used for heading smoothing.
    gyro: [Point; GYRO_BUFFER_LEN],
    /// Number of valid entries in `gyro` (0..GYRO_BUFFER_LEN).
    gyro_count: usize,
    /// Recent solver-angle deltas for the predictor (ring buffer).
    angle_history: [f64; ANGLE_HISTORY_LEN],
    /// Number of valid entries in `angle_history`.
    angle_hist_count: usize,
}

impl Tool {
    /// Create a new tool, initializing the gyroscope with the initial
    /// heading.
    pub fn new(pos: Point, heading: f64, radius: f64) -> Self {
        let dir = Point::new(heading.cos(), heading.sin());
        Self {
            pos,
            heading,
            radius,
            gyro: [dir; GYRO_BUFFER_LEN],
            gyro_count: GYRO_BUFFER_LEN,
            angle_history: [0.0; ANGLE_HISTORY_LEN],
            angle_hist_count: 0,
        }
    }

    fn smoothed_heading(&self) -> f64 {
        if self.gyro_count == 0 {
            return self.heading;
        }
        let mut sum = Point::ZERO;
        for i in 0..self.gyro_count {
            sum += self.gyro[i];
        }
        let avg = sum / self.gyro_count as f64;
        let len = avg.length();
        if len < 1e-9 {
            return self.heading;
        }
        avg.y.atan2(avg.x)
    }

    fn push_gyro(&mut self, dir: Point) {
        if GYRO_BUFFER_LEN == 0 {
            return;
        }
        for i in (1..GYRO_BUFFER_LEN).rev() {
            self.gyro[i] = self.gyro[i - 1];
        }
        self.gyro[0] = dir;
        if self.gyro_count < GYRO_BUFFER_LEN {
            self.gyro_count += 1;
        }
    }

    fn reset_gyro(&mut self) {
        let dir = Point::new(self.heading.cos(), self.heading.sin());
        self.gyro = [dir; GYRO_BUFFER_LEN];
        self.gyro_count = 1;
        self.angle_history = [0.0; ANGLE_HISTORY_LEN];
        self.angle_hist_count = 0;
    }

    fn push_angle(&mut self, delta: f64) {
        for i in (1..ANGLE_HISTORY_LEN).rev() {
            self.angle_history[i] = self.angle_history[i - 1];
        }
        self.angle_history[0] = delta;
        if self.angle_hist_count < ANGLE_HISTORY_LEN {
            self.angle_hist_count += 1;
        }
    }

    fn predicted_angle(&self) -> f64 {
        if self.angle_hist_count == 0 {
            return 0.0;
        }
        let sum: f64 =
            self.angle_history.iter().take(self.angle_hist_count).sum();
        sum / self.angle_hist_count as f64
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

// ── Resume helper ────────────────────────────────────────────────────

/// Try to recover after the tool stalls or is detected as stuck.
///
/// 1. Backward wall-hugging resume via [`search_frontier_engagement`].
/// 2. Fallback: travel to the nearest uncut frontier via
///    [`ClearedArea::remaining`].
///
/// Returns `true` if the tool was repositioned (caller should update
/// `prev_pos` and continue the main loop).
#[allow(clippy::too_many_arguments)]
fn try_resume(
    cleared: &mut ClearedArea,
    ops: &mut Ops,
    tool: &mut Tool,
    opts: &AdaptiveClearingOptions,
    valid_tool_area: &[Polygon],
    target_area_pd: f64,
    _max_def: f64,
    _target_eng: f64,
    min_cut_area: f64,
) -> bool {
    let max_cut_area = opts.step_length * target_area_pd * 1.5;
    dbg_log!(
        "RESUME  from=({:.3},{:.3})  heading={:.4}  R={:.1}  \
         advance={:.3}  step_len={:.3}  max_cut_area={:.4}",
        tool.pos.x,
        tool.pos.y,
        tool.heading,
        opts.radius,
        opts.advance,
        opts.step_length,
        max_cut_area,
    );
    if let Some(rp) = search_frontier_engagement(
        cleared,
        ToolPose {
            pos: tool.pos,
            heading: tool.heading,
        },
        opts.radius,
        opts.step_length,
        opts.advance,
        min_cut_area,
        max_cut_area,
    ) {
        dbg_log!(
            "  RESUME  path=search_frontier  → ({:.3},{:.3})  heading={:.4}",
            rp.pos.x, rp.pos.y, rp.heading,
        );
        ops.move_to(rp.pos.x, rp.pos.y, opts.cut_z, None);
        tool.pos = rp.pos;
        tool.heading = rp.heading;
        tool.reset_gyro();
        return true;
    }

    dbg_log!("  RESUME  path=centroid_fallback");

    // Fallback: jump to the centroid of the nearest remaining
    // (uncut) region.
    let remaining = cleared.remaining();
    let mut best_centroid: Option<(f64, Point)> = None;
    for poly in &remaining {
        if poly.len() < 3 {
            continue;
        }
        let area = get_polygon_area(poly).abs();
        if area < 0.3 {
            continue;
        }
        let centroid = get_polygon_centroid(poly);
        if !point_in_valid_area(centroid, valid_tool_area) {
            continue;
        }
        let dist = (centroid.x - tool.pos.x).powi(2)
            + (centroid.y - tool.pos.y).powi(2);
        if best_centroid.is_none_or(|(bd, _)| dist < bd) {
            best_centroid = Some((dist, centroid));
        }
    }

    if let Some((_, centroid)) = best_centroid {
        ops.move_to(centroid.x, centroid.y, opts.cut_z, None);
        tool.pos = centroid;
        tool.heading = 0.0;
        tool.reset_gyro();
        return true;
    }

    false
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

    // ── 3. Continuous spiral: step → expand → repeat ─────────────

    let mut ops = Ops::new();
    ops.apply_state(cut_state);
    ops.move_to(tool.pos.x, tool.pos.y, opts.cut_z, None);

    let mut prev_pos = tool.pos;
    let mut steps_since_batch: usize = 0;

    // Stuck detection: every STUCK_CHECK_INTERVAL steps, verify the
    // cleared area has grown by at least the expected amount.  If not,
    // the solver is oscillating in a corner and we trigger resume /
    // re-engagement.  Using area growth (rather than displacement)
    // allows the initial inner contraction spiral to keep cutting.
    let mut step_count: usize = 0;
    let mut last_check_area: f64 = cleared.total_area();
    let mut resume_count: usize = 0;

    let target_eng =
        2.0 * std::f64::consts::PI - 2.0 * (opts.advance / opts.radius).acos();
    let min_cut_area =
        opts.step_length * target_area_pd * ENGAGEMENT_FLOOR_FRAC;

    for _ in 0..MAX_TOTAL_STEPS {
        // Convergence: check that remaining uncut area is below
        // tolerance.  Use an inexpensive fragment-sum check first
        // and only pay for the full union+diff when it looks close.
        let frag_total = cleared.total_area();
        if frag_total >= valid_total - opts.area_tolerance && {
            let rem: f64 =
                cleared.remaining().iter().map(get_polygon_area).sum();
            rem < opts.area_tolerance
        } {
            break;
        }

        let heading = tool.smoothed_heading();
        let predicted = tool.predicted_angle();
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
                    ) {
                        prev_pos = tool.pos;
                        last_check_area = cleared.total_area();
                        step_count = 0;
                        continue;
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
                dbg_log!("max resumes reached");
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
            ) {
                prev_pos = tool.pos;
                last_check_area = cleared.total_area();
                step_count = 0;
                continue;
            }

            break;
        }

        // Emit cutting move.
        ops.line_to(tool.pos.x, tool.pos.y, opts.cut_z, None);

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

        prev_pos = tool.pos;
    }

    // Flush any remaining batch.
    if steps_since_batch > 0 {
        cleared.commit_batch_local();
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
