//! Stall / re-engagement logic for [`super::adaptive_clearing`].
//!
//! When the forward-stepping solver stalls (lost engagement, boundary
//! hit, or stuck oscillation), the strategies in this module reposition
//! the tool.

use prof_macros::prof;

use crate::dbg_log;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::shape::compute_polygon_bounds;
use crate::geo::shape::polygon::{
    get_polygon_signed_area, get_polygons_closest_point,
};
use crate::ops::container::Ops;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::search::walk_polygon_samples;
use crate::ops::cut::step;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::CutDirection;
use crate::ops::cut::StepStatus;
use crate::ops::cut::StepperOptions;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

use super::routing;
use super::tool::Tool;
use super::AdaptiveClearingOptions;

pub use super::resume_envelope::ResumeEnvelope;
pub use super::resume_frontier::ResumeFrontier;
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

// ── Shared probe function ──────────────────────────────────────────

/// Lightweight forward-step engagement check used by both
/// [`ResumeFrontier`] and [`ResumeEnvelope`].
///
/// Projects `pos` forward by `step_length` along `heading`, then
/// checks cut-area split, rotational direction (correct side ≥ wrong
/// side), and destination engagement angle (≤ 2.7 rad).  Returns
/// `Some(ToolPose)` when all conditions pass — same logic as the
/// original boundary-walk probe.
#[prof]
pub(super) fn boundary_probe(
    ctx: &ResumeCtx,
    radius: f64,
    pos: Point,
    heading: f64,
) -> Option<ToolPose> {
    let step_length = ctx.opts.step_length;
    let dir_sign = ctx.opts.cut_direction.sign();
    let min_cut_area = step_length * ctx.target_area_pd * 0.5;
    let dir = Point::new(heading.cos(), heading.sin());

    let probe = pos + dir * step_length;
    if !point_in_valid_area(probe, ctx.valid_tool_area) {
        return None;
    }
    let (area, left) = ctx.cleared.cut_area_split(pos, probe, radius);
    if area < min_cut_area {
        return None;
    }
    let right = area - left;
    let correct_side = if dir_sign < 0.0 { right } else { left };
    let wrong_side = if dir_sign < 0.0 { left } else { right };
    if correct_side < wrong_side {
        return None;
    }
    let dest_eng = ctx.cleared.point_engagement(probe, radius).angle;
    if dest_eng > 2.7 {
        return None;
    }
    Some(ToolPose { pos, heading })
}

// ── Shared walk engine ──────────────────────────────────────────────

/// Walk a group of boundary polygons starting from the one nearest
/// `segment_start`, in the cutting rotational direction, calling
/// `probe` at each sample.  Returns the first engaging `ToolPose`.
///
/// `probe` receives the sample point (on the boundary polygon) and the
/// walk-direction tangent (`heading`); it decides how to place the tool
/// centre and whether the position engages.  This is where the
/// frontier/envelope offset policy differs — see [`ResumeFrontier`] and
/// [`ResumeEnvelope`].
pub(super) fn walk_and_probe(
    ctx: &ResumeCtx,
    radius: f64,
    polys: &[Polygon],
    log_tag: &str,
    mut probe: impl FnMut(&ResumeCtx, f64, Point, f64) -> Option<ToolPose>,
) -> Option<ToolPose> {
    let ref_pos = ctx.segment_start.pos;

    let (closest_poly_idx, _t, _closest_pt, _d2) =
        get_polygons_closest_point(polys, ref_pos)?;
    let poly = &polys[closest_poly_idx];
    let n = poly.len();
    if n < 3 {
        return None;
    }

    let mut start_idx = 0usize;
    let mut start_frac = 0.0f64;
    {
        let mut best_d2 = f64::MAX;
        for i in 0..n {
            let j = (i + 1) % n;
            let edge = poly[j] - poly[i];
            let elen2 = edge.length_squared();
            if elen2 < 1e-18 {
                continue;
            }
            let tt = ((ref_pos.x - poly[i].x) * edge.x
                + (ref_pos.y - poly[i].y) * edge.y)
                / elen2;
            let tt = tt.clamp(0.0, 1.0);
            let cp = poly[i] + edge * tt;
            let d2 = cp.distance_squared(ref_pos);
            if d2 < best_d2 {
                best_d2 = d2;
                start_idx = i;
                start_frac = tt;
            }
        }
    }

    let is_ccw = get_polygon_signed_area(poly) > 0.0;
    let actual_forward =
        (ctx.opts.cut_direction == CutDirection::Ccw) == is_ccw;

    let sample_spacing = ctx.opts.step_length;
    let bl_sq_tol = (ctx.opts.step_length * 0.25).powi(2);

    walk_polygon_samples(
        poly,
        start_idx,
        actual_forward,
        sample_spacing,
        false,
        start_frac,
        |pt, heading| {
            for bl_pos in ctx.blacklist {
                let dx = bl_pos.x - pt.x;
                let dy = bl_pos.y - pt.y;
                if dx * dx + dy * dy < bl_sq_tol {
                    return None;
                }
            }
            if let Some(probed) = probe(ctx, radius, pt, heading) {
                dbg_log!(
                    "  {}  resume=({:.3},{:.3})  heading={:.4}  \
                     sample=({:.3},{:.3})",
                    log_tag,
                    probed.pos.x,
                    probed.pos.y,
                    probed.heading,
                    pt.x,
                    pt.y,
                );
                Some(probed)
            } else {
                None
            }
        },
    )
    .or_else(|| {
        dbg_log!("  {}  no suitable point found", log_tag);
        None
    })
}

/// Emit a resume travel from `from` to `to` using the routing strategies.
///
/// Returns the [`RouteSource`] that was used for the travel path.
#[prof]
pub fn emit_resume_travel(
    ops: &mut Ops,
    cleared: &ClearedArea,
    mat: Option<&MedialAxis>,
    from: Point,
    to: Point,
    opts: &AdaptiveClearingOptions,
) -> routing::RouteSource {
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
        return source;
    }

    // Fallback: direct move.
    ops.move_to(to.x, to.y, opts.cut_z + 0.5, None);
    routing::RouteSource::RoutingDirect
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
    /// Walk the cleared-area frontier (material boundary), probing for engagement.
    ResumeFrontier = 4,
    /// Walk the island perimeter looking for productive engagement.
    ResumeIsland = 5,
    /// Walk the tool-centre envelope (pocket-wall boundary), tool centre
    /// on the edge, probing for engagement.
    ResumeEnvelope = 6,
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
    let strategies: [(&dyn ResumeStrategy, ResumeSource); 6] = [
        (&ResumeWallHug, ResumeSource::ResumeWallHug),
        (&ResumeSegment, ResumeSource::ResumeSegment),
        (&ResumeMat, ResumeSource::ResumeMat),
        (&ResumeFrontier, ResumeSource::ResumeFrontier),
        (&ResumeEnvelope, ResumeSource::ResumeEnvelope),
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
