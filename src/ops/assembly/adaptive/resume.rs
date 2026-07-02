//! Stall / re-engagement logic for [`super::adaptive_clearing`].
//!
//! When the forward-stepping solver stalls (lost engagement, boundary
//! hit, or stuck oscillation), the strategies in this module reposition
//! the tool.

use prof_macros::prof;

use crate::dbg_log;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::shape::compute_polygon_bounds;
use crate::ops::container::Ops;
use crate::ops::cut::step;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::StepStatus;
use crate::ops::cut::StepperOptions;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

use super::routing;
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

/// Ground-truth engagement check: run [`step`] at a candidate
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
/// tool there and then the main loop calls [`step`] again on the
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
    let probe_opts = StepperOptions {
        target_area_pd: ctx.target_area_pd,
        step_length: ctx.opts.step_length,
        radius,
        max_deflection: angle_max.max(angle_min.abs()),
        valid_area: ctx.valid_tool_area,
        angle_min,
        angle_max,
        dir_sign,
    };
    let result = step(ctx.cleared, pos, heading, 0.0, &probe_opts);
    if result.status == StepStatus::Ok {
        dbg_log!(
            "  PROBE[one-sided]  pos=({:.3},{:.3})  heading_in={:.4}  \
             heading_out={:.4}  iters={}  bounds=({:.4},{:.4})",
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
            "  PROBE[one-sided]  MISS  pos=({:.3},{:.3})  heading={:.4}  \
             status={:?}  bounds=({:.4},{:.4})",
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

/// Emit a resume travel from `from` to `to` using the routing strategies.
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
    let obs_bounds = compute_polygon_bounds(&obstacles);

    let ctx = routing::RouteCtx {
        cleared,
        opts,
        mat,
        obstacles: &obstacles,
        obstacle_bounds: &obs_bounds,
    };

    if let Some((source, path)) = routing::optimize_route(&ctx, from, to) {
        dbg_log!(
            "  EMIT  route={}:{}  n={}",
            source as u8,
            routing::source_label(source),
            path.len(),
        );
        for pt in &path {
            ops.move_to(pt.x, pt.y, opts.cut_z + 0.5, None);
        }
        return;
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
