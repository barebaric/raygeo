//! Stall / re-engagement logic for [`super::adaptive_clearing`].
//!
//! When the forward-stepping solver stalls (lost engagement, boundary
//! hit, or stuck oscillation), the strategies in this module reposition
//! the tool.

use prof_macros::prof;

use crate::dbg_log;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::algo::smooth::build_smoothed_path;
use crate::geo::shape::compute_polygon_bounds;
use crate::geo::shape::does_line_cross_polygon;
use crate::geo::shape::get_polygon_signed_area;
use crate::geo::shape::is_point_in_polygon;
use crate::ops::container::Ops;
use crate::ops::cut::step_adaptive;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::StepStatus;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

use super::tool::Tool;
use super::AdaptiveClearingOptions;

pub use super::resume_boundary::ResumeBoundary;
pub use super::resume_island::ResumeIsland;
pub use super::resume_mat::{
    find_all_mat_crossings, mat_resume_from_crossing, ResumeMat,
};
pub use super::resume_segment::ResumeSegment;
pub use super::resume_wall_hug::ResumeWallHug;

// ── Resume constants ─────────────────────────────────────────────────

/// Maximum number of resume / re-engagement attempts before giving up.
pub(super) const MAX_RESUMES: usize = 500;

/// Frontier vertex this close to the pocket boundary (outer wall or any
/// island) counts as a wall collision (mm).
///
/// The frontier is the unioned boundary of the cleared region, built from
/// tool-disk sweeps.  Its vertices sit a small epsilon off the true wall
/// — a typical polygon-union offset of ~0.05 mm that comfortably exceeds
/// the naive "on the wall" threshold of 0.01 mm.  The threshold must be
/// loose enough to reliably catch wall-adjacent frontier vertices (so the
/// `mat_resume_from_crossing` backward walk finds a wall hit) without
/// matching interior frontier points.
pub(super) const WALL_PROXIMITY: f64 = 0.3;

// ── Helpers ───────────────────────────────────────────────────────────

/// Check whether a straight-line travel from `from` to `to` avoids
/// crossing uncleared obstacles.
///
/// Returns `true` when the segment centreline does not cross any
/// obstacle polygon boundary (remaining-stock frontier or pocket
/// envelope).  Endpoint touches (the resume position at the cutting
/// edge) are *not* counted as crossings, so paths that merely
/// approach the frontier are accepted.
fn direct_path_safe(from: Point, to: Point, obstacles: &[Polygon]) -> bool {
    if obstacles.is_empty() {
        return true;
    }
    let bounds = compute_polygon_bounds(obstacles);

    // Precompute winding sign for each obstacle (+1 CCW, −1 CW).
    let signs: Vec<i8> = obstacles
        .iter()
        .map(|obs| {
            if get_polygon_signed_area(obs) > 0.0 {
                1
            } else {
                -1
            }
        })
        .collect();

    // Winding-number point-in-region test using NonZero rule.
    let in_remaining = |p: Point| -> bool {
        let mut winding = 0i32;
        for ((obs, b), &sign) in obstacles.iter().zip(&bounds).zip(&signs) {
            if obs.len() < 3 {
                continue;
            }
            if p.x < b.min.x || p.x > b.max.x || p.y < b.min.y || p.y > b.max.y
            {
                continue;
            }
            if is_point_in_polygon(p, obs) {
                winding += sign as i32;
            }
        }
        winding > 0
    };

    // Start point MUST be in cleared territory.
    if in_remaining(from) {
        return false;
    }

    // Check that the segment does not properly cross any obstacle
    // polygon boundary (does_line_cross_polygon uses an interior-point
    // test — t in (0, 1) — so endpoint touches are not flagged).
    obstacles
        .iter()
        .all(|obs| !does_line_cross_polygon(from, to, obs))
}

/// Ground-truth engagement check: run `step_adaptive` at a candidate
/// position with **one-sided deflection bounds** derived from
/// `opts.cut_direction`.  This vets that the stepper can cut in the
/// chosen rotational direction from `pos` — a position that only
/// finds engagement on the wrong side is rejected.
///
/// The main loop uses symmetric bounds for stability; `probe_step`
/// is the gatekeeper that prevents resume from placing the tool
/// where it would immediately cut the wrong way.
///
/// Returns `Some(ToolPose)` if the stepper would produce
/// `StepStatus::Ok`, `None` otherwise.
///
/// The returned position is the original `pos` — the caller places the
/// tool there and then the main loop calls `step_adaptive` again on the
/// next iteration to move forward.
#[prof]
pub fn probe_step(
    ctx: &ResumeCtx,
    radius: f64,
    pos: Point,
    heading: f64,
) -> Option<ToolPose> {
    let max_deflection = ctx.opts.max_deflection_deg.to_radians();
    let dir_sign = ctx.opts.cut_direction.sign();
    let (angle_min, angle_max) =
        ctx.opts.cut_direction.angle_bounds(max_deflection);
    probe_step_impl(
        ctx,
        radius,
        pos,
        heading,
        angle_min,
        angle_max,
        dir_sign,
        "one-sided",
    )
}

/// Like [`probe_step`] but only checks that engagement exists (cut_area
/// above floor), without enforcing the engagement ceiling.  Used for
/// stepover-targeted resume positions where the first step naturally
/// has high engagement (the tool plunges into material at the correct
/// lateral offset) but subsequent steps will have normal engagement.
#[prof]
pub fn probe_step_sym(
    ctx: &ResumeCtx,
    radius: f64,
    pos: Point,
    heading: f64,
) -> Option<ToolPose> {
    let max_deflection = std::f64::consts::FRAC_PI_4;
    let max_def = max_deflection;
    let dir_sign = ctx.opts.cut_direction.sign();
    let result = step_adaptive(
        ctx.cleared,
        pos,
        heading,
        0.0,
        ctx.target_area_pd,
        ctx.opts.step_length,
        radius,
        max_def,
        ctx.valid_tool_area,
        -max_def,
        max_def,
        dir_sign,
    );
    // Accept the position if the solver found ANY engagement (not
    // LostEngagement due to under-engagement).  Over-engagement on the
    // first step is expected at the stepover offset — the main loop
    // will handle it on subsequent steps.
    let accept = result.status == StepStatus::Ok
        || (result.cut_area > 0.0
            && result.status == StepStatus::LostEngagement
            && result.engagement.angle > std::f64::consts::FRAC_PI_4);
    if accept {
        dbg_log!(
            "  PROBE[stepover]  pos=({:.3},{:.3})  heading_in={:.4}  \
             heading_out={:.4}  iters={}  cut_area={:.4}  eng_angle={:.4}  \
             status={:?}",
            pos.x,
            pos.y,
            heading,
            result.heading,
            result.iters,
            result.cut_area,
            result.engagement.angle,
            result.status,
        );
        Some(ToolPose {
            pos,
            heading: result.heading,
        })
    } else {
        dbg_log!(
            "  PROBE[stepover]  MISS  pos=({:.3},{:.3})  heading={:.4}  \
             status={:?}  cut_area={:.4}  eng_angle={:.4}",
            pos.x,
            pos.y,
            heading,
            result.status,
            result.cut_area,
            result.engagement.angle,
        );
        None
    }
}

#[allow(clippy::too_many_arguments)]
#[prof]
fn probe_step_impl(
    ctx: &ResumeCtx,
    radius: f64,
    pos: Point,
    heading: f64,
    angle_min: f64,
    angle_max: f64,
    dir_sign: f64,
    mode: &str,
) -> Option<ToolPose> {
    let result = step_adaptive(
        ctx.cleared,
        pos,
        heading,
        0.0,
        ctx.target_area_pd,
        ctx.opts.step_length,
        radius,
        angle_max.max(angle_min.abs()),
        ctx.valid_tool_area,
        angle_min,
        angle_max,
        dir_sign,
    );
    if result.status == StepStatus::Ok {
        dbg_log!(
            "  PROBE[{}]  pos=({:.3},{:.3})  heading_in={:.4}  \
             heading_out={:.4}  iters={}  bounds=({:.4},{:.4})",
            mode,
            pos.x,
            pos.y,
            heading,
            result.heading,
            result.iters,
            angle_min,
            angle_max,
        );
        Some(ToolPose {
            pos,
            heading: result.heading,
        })
    } else {
        dbg_log!(
            "  PROBE[{}]  MISS  pos=({:.3},{:.3})  heading={:.4}  \
             status={:?}  bounds=({:.4},{:.4})",
            mode,
            pos.x,
            pos.y,
            heading,
            result.status,
            angle_min,
            angle_max,
        );
        None
    }
}

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
#[prof]
pub fn smooth_travel_path(
    from: Point,
    raw: &[Point],
    obstacles: &[Polygon],
    clearance: f64,
) -> Vec<Point> {
    if raw.is_empty() {
        return vec![from];
    }
    let last = raw[raw.len() - 1];
    let waypoints: Vec<Point> = if raw.len() > 2 {
        raw[1..raw.len() - 1].to_vec()
    } else {
        Vec::new()
    };
    let obs_bounds = compute_polygon_bounds(obstacles);
    let smoothed = build_smoothed_path(
        from,
        last,
        &waypoints,
        obstacles,
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

/// Emit a resume travel from `from` to `to`, avoiding uncleared obstacles
/// when possible via the medial axis.
#[prof]
pub fn emit_resume_travel(
    ops: &mut Ops,
    cleared: &ClearedArea,
    mat: Option<&MedialAxis>,
    from: Point,
    to: Point,
    opts: &AdaptiveClearingOptions,
) {
    let obstacles = cleared.remaining();

    // Fast path: direct move when no obstacles or the straight line is safe.
    if obstacles.is_empty() || direct_path_safe(from, to, &obstacles) {
        ops.move_to(to.x, to.y, opts.cut_z + 0.5, None);
        return;
    }

    // Medial-axis guided path through cleared territory.
    if let Some(axis) = mat {
        let raw = axis
            .path_between_cleared(from, to, cleared.fragments())
            .or_else(|| axis.path_between(from, to));
        if let Some(ref path) = raw {
            let smoothed =
                smooth_travel_path(from, path, &obstacles, opts.radius);
            for pt in &smoothed {
                ops.move_to(pt.x, pt.y, opts.cut_z + 0.5, None);
            }
            return;
        }
    }

    // Fallback: direct move.
    ops.move_to(to.x, to.y, opts.cut_z + 0.5, None);
}

// ── ResumeSource enum (renamed) ─────────────────────────────────────

/// Which resume mechanism succeeded, in priority order.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResumeSource {
    /// Resume from the envelope-departure point (wall-hug).
    ResumeWallHug = 1,
    /// Walk forward from segment_start probing for engagement.
    ResumeSegment = 2,
    /// MAT-guided walk to pocket-wall meeting point.
    ResumeMat = 3,
    /// Find engagement on the cleared-area frontier.
    ResumeBoundary = 4,
    /// Walk the island perimeter looking for productive engagement.
    ResumeIsland = 5,
}

// ── ResumeContext ───────────────────────────────────────────────────

/// Read-only snapshot of everything a resume strategy may need.
/// Mutable values (`segment_start`, `last_resume_*`) are copied in by
/// value. The caller updates the originals after `try_resume` returns.
pub struct ResumeCtx<'a> {
    pub cleared: &'a ClearedArea,
    pub opts: &'a AdaptiveClearingOptions,
    pub valid_tool_area: &'a [Polygon],
    pub mat: Option<&'a MedialAxis>,
    pub target_area_pd: f64,
    /// Position and heading where the current cutting segment began.
    /// ResumeSegment probes forward from here using the stored heading.
    pub segment_start: ToolPose,
    pub last_resume_area: f64,
    pub last_resume_pos: Point,
    /// Last pose where the tool was on the envelope before departing
    /// into the interior.  Populated by the main loop while
    /// `near_envelope` tracking is active; consumed by
    /// [`ResumeWallHug`].
    pub last_wall_hug: Option<ToolPose>,
    /// Positions that have already been tried and led to immediate stalls.
    /// Any strategy that produces a position too close to one of these is
    /// rejected.
    pub blacklist: &'a [Point],
}

// ── ResumeStrategy trait ────────────────────────────────────────────

/// A strategy that finds a new tool position when the stepper stalls.
pub trait ResumeStrategy {
    fn find_next(&self, ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose>;
    fn label(&self) -> &'static str;
}

// ── try_resume orchestrator ──────────────────────────────────────────

/// Try each strategy in priority order.  Returns the winning strategy
/// and the tool pose to apply.  Pure query — the caller handles all
/// mutation (emit travel, set tool, update resume state).
#[prof]
pub fn try_resume(
    ctx: &ResumeCtx,
    tool: &Tool,
) -> Option<(ResumeSource, ToolPose)> {
    let area_grew = ctx.cleared.total_area() > ctx.last_resume_area + 1e-9;
    let tool_dx = tool.pos.x - ctx.last_resume_pos.x;
    let tool_dy = tool.pos.y - ctx.last_resume_pos.y;
    let tool_moved = (tool_dx * tool_dx + tool_dy * tool_dy)
        > (ctx.opts.step_length * 0.25).powi(2);

    dbg_log!(
        "RESUME  from=({:.3},{:.3})  heading={:.4}  R={:.1}  \
         advance={:.3}  step_len={:.3}  last_area={:.3}  cur_area={:.3}",
        tool.pos.x,
        tool.pos.y,
        tool.heading,
        ctx.opts.radius,
        ctx.opts.advance,
        ctx.opts.step_length,
        ctx.last_resume_area,
        ctx.cleared.total_area(),
    );

    if !area_grew && !tool_moved {
        dbg_log!("  RESUME  no change since last resume — giving up");
        return None;
    }

    // Try each strategy in priority order.  Each strategy decides for
    // itself whether it is responsible (e.g. WallHug returns None when
    // there is no stored wall-hug pose).
    let strategies: [(&dyn ResumeStrategy, ResumeSource); 5] = [
        (&ResumeWallHug, ResumeSource::ResumeWallHug),
        (&ResumeSegment, ResumeSource::ResumeSegment),
        (&ResumeMat, ResumeSource::ResumeMat),
        (&ResumeBoundary, ResumeSource::ResumeBoundary),
        (&ResumeIsland, ResumeSource::ResumeIsland),
    ];
    let sq_tol = (ctx.opts.step_length * 0.25).powi(2);
    'strategy: for (s, source) in &strategies {
        if let Some(rp) = s.find_next(ctx, tool) {
            // Reject any position that has already been tried and led to
            // an immediate stall, regardless of which strategy produced it.
            for bl_pos in ctx.blacklist {
                let dx = bl_pos.x - rp.pos.x;
                let dy = bl_pos.y - rp.pos.y;
                if dx * dx + dy * dy < sq_tol {
                    dbg_log!(
                        "  RESUME  {}={}  → ({:.3},{:.3})  BLACKLISTED",
                        *source as u8,
                        s.label(),
                        rp.pos.x,
                        rp.pos.y,
                    );
                    continue 'strategy;
                }
            }
            dbg_log!(
                "  RESUME  {}={}  → ({:.3},{:.3})  heading={:.4}",
                *source as u8,
                s.label(),
                rp.pos.x,
                rp.pos.y,
                rp.heading,
            );
            return Some((*source, rp));
        }
    }

    None
}
