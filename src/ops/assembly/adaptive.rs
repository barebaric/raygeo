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

use crate::dbg_log;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::offset::compute_inset_region;
use crate::geo::algo::smooth::build_smoothed_path;
use crate::geo::shape::arc::normalize_angle_signed;
use crate::geo::shape::polygon::compute_polygon_bounds;
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

use std::path::PathBuf;

use super::trace::{write_toolpath, RecordBuf, TraceKind};
#[cfg(debug_assertions)]
use crate::trace::Tracer;

// ── Named constants ────────────────────────────────────────────────

/// Floor fraction of target cut-area-per-distance below which we treat
/// engagement as lost.
const ENGAGEMENT_FLOOR_FRAC: f64 = 0.01;
/// Maximum total steps before giving up (safety valve).
const MAX_TOTAL_STEPS: usize = 100_000;
/// Per-step decay applied to the persisted predictor.  A converged
/// deflection feeds `decay · prev + (1-decay) · new` back in, so a
/// steady curvature is tracked while a one-off correction decays to
/// zero within ~3 steps instead of seeding the next solver trial.
const PREDICTOR_DECAY: f64 = 0.5;
/// The predictor is only allowed to seed the solver with a deflection
/// up to this fraction of `max_deflection`.  Larger corrections must
/// come from the solver's own bracket search, not from feedforward —
/// this prevents a stale large predicted angle from dominating the
/// first trial and pinning `best_error` on an overshoot.
const PREDICTOR_CLAMP_FRAC: f64 = 0.5;
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
    /// Decayed predictor value.  Updated only on converged steps and
    /// multiplied by [`PREDICTOR_DECAY`] each step, so a single
    /// transient over-correction does not seed the next step's solver
    /// trial and create a multi-step steering oscillation.
    predictor: f64,
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
            predictor: 0.0,
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
        self.predictor = 0.0;
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

    /// Update the decayed predictor.  Called only when a step
    /// converged (the deflection is trustworthy signal of real
    /// curvature, not a transient correction).  The new estimate is
    /// a low-pass blend of the previous predictor and the latest
    /// deflection, so steady curvature is tracked while one-off
    /// corrections decay away within a few steps.
    fn update_predictor(&mut self, delta: f64) {
        self.predictor =
            PREDICTOR_DECAY * self.predictor + (1.0 - PREDICTOR_DECAY) * delta;
    }

    /// Predictor seed for [`step_adaptive`].  Clamped to a fraction of
    /// `max_deflection` so a stale large estimate can never dominate
    /// the first solver trial and pin `best_error` on an overshoot.
    fn predicted_angle(&self, max_deflection: f64) -> f64 {
        let clamp = max_deflection * PREDICTOR_CLAMP_FRAC;
        self.predictor.clamp(-clamp, clamp)
    }

    /// Raw (un-clamped) predictor value, exposed for trace records so
    /// the inspector can show the true internal state.
    fn raw_predictor(&self) -> f64 {
        self.predictor
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

/// Smooth and shorten a cleared-territory travel path.
///
/// `raw` is a waypoint list (e.g. from the MAT) that is known to stay
/// inside cleared territory.  It is fed through [`build_smoothed_path`]
/// against the uncleared obstacles (`islands` ∪ `remaining`) so that
/// redundant intermediate waypoints are shortcut away and any sharp
/// turns are rounded — keeping the tool disk clear of fresh stock while
/// minimising rapid travel length.
///
/// `from` is the tool's current position and is preserved verbatim as
/// the path's first point so the smoothing kernel never moves it.
fn smooth_travel_path(
    from: Point,
    raw: &[Point],
    islands: &[Polygon],
    remaining: &[Polygon],
    clearance: f64,
) -> Vec<Point> {
    if raw.is_empty() {
        return vec![from];
    }
    let last = raw[raw.len() - 1];
    // build_smoothed_path prepends `from` and appends `last`; the
    // intermediate waypoints are the MAT path minus its (already
    // supplied) endpoints to avoid duplicating them.
    let waypoints: Vec<Point> = if raw.len() > 2 {
        raw[1..raw.len() - 1].to_vec()
    } else {
        Vec::new()
    };
    let mut obstacles: Vec<Polygon> = islands.to_vec();
    obstacles.extend_from_slice(remaining);
    let obs_bounds = compute_polygon_bounds(&obstacles);
    let smoothed = build_smoothed_path(
        from,
        last,
        &waypoints,
        &obstacles,
        &obs_bounds,
        clearance,
        120,
    );
    if smoothed.is_empty() {
        vec![from, last]
    } else {
        smoothed
    }
}

/// Emit a safe resume travel from `from` to `to`.
///
/// When a Medial Axis is available, route the travel through cleared
/// territory by walking the MAT tree between the two endpoints' nearest
/// cleared nodes, then shorten and smooth the resulting waypoint list
/// via [`smooth_travel_path`] / [`build_smoothed_path`] so redundant
/// hops are shortcut away and sharp turns are rounded — producing the
/// shortest collision-free rapid path instead of the raw tree walk.
/// When no MAT is available (or no cleared path exists), fall back to a
/// single `move_to` — preserving the previous behaviour.
fn emit_resume_travel(
    ops: &mut Ops,
    cleared: &ClearedArea,
    mat: Option<&MedialAxis>,
    from: Point,
    to: Point,
    opts: &AdaptiveClearingOptions,
) {
    let z = opts.cut_z + 0.5;
    if let Some(axis) = mat {
        let fragments = cleared.fragments();
        if !fragments.is_empty() {
            if let Some(path) = axis.path_between_cleared(from, to, fragments) {
                if path.len() >= 2 {
                    let remaining = cleared.remaining();
                    let smoothed = smooth_travel_path(
                        from,
                        &path,
                        &opts.islands,
                        &remaining,
                        opts.radius,
                    );
                    for wp in &smoothed {
                        ops.move_to(wp.x, wp.y, z, None);
                    }
                    return;
                }
            }
        }
    }
    ops.move_to(to.x, to.y, z, None);
}

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
    mat: Option<&MedialAxis>,
    last_resume_area: f64,
    _segment_start: Point,
) -> bool {
    let max_cut_area = opts.step_length * target_area_pd * 1.5;
    dbg_log!(
        "RESUME  from=({:.3},{:.3})  heading={:.4}  R={:.1}  \
         advance={:.3}  step_len={:.3}  max_cut_area={:.4}  \
         last_area={:.3}  cur_area={:.3}",
        tool.pos.x,
        tool.pos.y,
        tool.heading,
        opts.radius,
        opts.advance,
        opts.step_length,
        max_cut_area,
        last_resume_area,
        cleared.total_area(),
    );

    // ── Primary: frontier engagement search ──────────────────────
    // Skip the frontier search when the previous resume produced no
    // cleared-area growth (the tool stalled again immediately).  This
    // breaks the degenerate bounce between nearby frontier vertices
    // against an island wall: instead of re-trying the same local
    // frontier, fall straight through to the MAT walk that routes
    // around the island to fresh material.
    let area_grew = cleared.total_area() > last_resume_area + 1e-9;
    if area_grew {
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
            let moved = (rp.pos.x - tool.pos.x).powi(2)
                + (rp.pos.y - tool.pos.y).powi(2);
            if moved > (opts.step_length * 0.25).powi(2) {
                dbg_log!(
                    "  RESUME  path=search_frontier  → ({:.3},{:.3})  \
                     heading={:.4}",
                    rp.pos.x,
                    rp.pos.y,
                    rp.heading,
                );
                // The frontier search returns only the destination pose;
                // emit a *safe* travel that routes through cleared
                // territory (via the MAT when available) rather than a
                // straight-line jump that could cross an island.
                emit_resume_travel(ops, cleared, mat, tool.pos, rp.pos, opts);
                tool.pos = rp.pos;
                tool.heading = rp.heading;
                tool.reset_gyro();
                return true;
            }
            dbg_log!(
                "  RESUME  frontier_search returned no-progress \
                 ({:.3},{:.3}) ≈ current; skipping to fallback",
                rp.pos.x,
                rp.pos.y,
            );
        }
    } else {
        dbg_log!(
            "  RESUME  skipping frontier_search (no area growth since \
             last resume)"
        );
    }

    // ── Fallback: Medial Axis walk ─────────────────────────────────
    // Route from the stuck tool position along the medial axis of the
    // pocket (through cleared territory) to the nearest uncleared MAT
    // node, then resume cutting there.  The MAT path is shortened and
    // smoothed via [`smooth_travel_path`] so the tool traces the
    // shortest collision-free route through already-cut material rather
    // than following every tree node.
    if let Some(axis) = mat {
        if let Some((path, heading)) =
            mat_resume_target(axis, cleared, tool.pos, valid_tool_area)
        {
            dbg_log!(
                "  RESUME  path=mat_walk  {} waypoints  → ({:.3},{:.3})  \
                 heading={:.4}",
                path.len(),
                path[path.len() - 1].x,
                path[path.len() - 1].y,
                heading,
            );
            // Travel to the MAT target through cleared territory.
            // `mat_resume_target` already returned a path restricted to
            // cleared MAT nodes; `smooth_travel_path` shortcuts redundant
            // waypoints (collision-checked against the islands and
            // remaining stock) and rounds sharp turns so the tool follows
            // the shortest safe route instead of the raw tree walk.
            let remaining = cleared.remaining();
            let smoothed = smooth_travel_path(
                tool.pos,
                &path,
                &opts.islands,
                &remaining,
                opts.radius,
            );
            let dest = smoothed[smoothed.len() - 1];
            for wp in &smoothed {
                ops.move_to(wp.x, wp.y, opts.cut_z + 0.5, None);
            }
            tool.pos = dest;
            tool.heading = heading;
            tool.reset_gyro();
            return true;
        }
    }

    dbg_log!("  RESUME  path=centroid_fallback");

    // ── Last-resort: nearest valid-area centroid of a remaining region
    // (legacy behaviour, retained for when MAT is unavailable).
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
        emit_resume_travel(ops, cleared, mat, tool.pos, centroid, opts);
        tool.pos = centroid;
        tool.heading = 0.0;
        tool.reset_gyro();
        return true;
    }

    false
}

/// Pick a resume target by walking the Medial Axis Transform of the
/// pocket from the tool's current position to the nearest uncleared MAT
/// node.
///
/// The MAT is a tree of maximally-inscribed-circle centres.  Each node
/// carries a clearance radius.  A node is "cleared" when it lies inside
/// one of the cleared fragments; "uncleared" nodes mark pockets of fresh
/// material still to be cut.
///
/// The strategy:
///  1. Build a cleared/uncleared mask over the MAT nodes.
///  2. Find the nearest *uncleared* node to the tool whose clearance
///     circle fits inside the valid tool-centre region (so the tool can
///     actually sit there).
///  3. Route from the tool's nearest cleared node to that target along
///     the MAT tree, staying within cleared nodes (the tool travels
///     through already-cut material at cut depth).
///  4. Return the full travel path (MAT nodes through cleared territory,
///     plus the final step into the uncleared target) and a heading that
///     faces fresh material so the solver immediately engages.
fn mat_resume_target(
    axis: &MedialAxis,
    cleared: &ClearedArea,
    tool_pos: Point,
    valid_tool_area: &[Polygon],
) -> Option<(Vec<Point>, f64)> {
    let fragments = cleared.fragments();
    if fragments.is_empty() {
        return None;
    }
    let is_cleared = axis.build_cleared_mask(fragments);

    // Nearest MAT node to the tool (the routing start).
    let from_idx = axis.nearest_node(tool_pos)?;
    // If the tool is already on an uncleared node there is nothing to
    // route to — engagement should have been available.
    if !is_cleared[from_idx] {
        return None;
    }

    // Search for the nearest uncleared node (BFS over the MAT tree) that
    // lies inside the valid tool-centre region.  BFS over the tree from
    // `from_idx` visits nodes in order of tree-distance, which is a good
    // proxy for travel distance through cleared territory.
    let target_idx = nearest_uncleared_node(axis, from_idx, &is_cleared)
        .filter(|&idx| {
            point_in_valid_area(axis.nodes[idx].point, valid_tool_area)
        })?;

    // Route through cleared nodes only.  The path ends at the cleared
    // ancestor of the (uncleared) target — the last cleared position
    // adjacent to fresh material.
    let mut path =
        axis.path_between_indices_cleared(from_idx, target_idx, &is_cleared)?;
    if path.len() < 2 {
        return None;
    }
    let last_cleared = path[path.len() - 1];
    // Append the uncleared target node as the final waypoint — the tool
    // travels the cleared path, then takes one final step into fresh
    // material.  Heading faces from the last cleared node toward the
    // target so the solver immediately engages.
    let target_pt = axis.nodes[target_idx].point;
    path.push(target_pt);
    let heading =
        (target_pt.y - last_cleared.y).atan2(target_pt.x - last_cleared.x);
    Some((path, heading))
}

/// BFS over the Medial Axis tree from `start`, returning the index of the
/// nearest (fewest hops) node that is **not** cleared.
fn nearest_uncleared_node(
    axis: &MedialAxis,
    start: usize,
    is_cleared: &[bool],
) -> Option<usize> {
    use std::collections::VecDeque;
    let n = axis.nodes.len();
    if n == 0 {
        return None;
    }
    let mut visited = vec![false; n];
    let mut queue: VecDeque<usize> = VecDeque::new();
    visited[start] = true;
    queue.push_back(start);
    // Build adjacency from the edge list.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in &axis.edges {
        adj[a].push(b);
        adj[b].push(a);
    }
    while let Some(idx) = queue.pop_front() {
        if idx != start && !is_cleared[idx] {
            return Some(idx);
        }
        for &nb in &adj[idx] {
            if !visited[nb] {
                visited[nb] = true;
                queue.push_back(nb);
            }
        }
    }
    None
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
        Some(path) => match Tracer::open(path) {
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
            let _ = t.finish();
            write_toolpath(t.path(), &ops);
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
