//! Stall / re-engagement logic for [`super::adaptive_clearing`].
//!
//! When the forward-stepping solver stalls (lost engagement, boundary
//! hit, or stuck oscillation), the strategies in this module reposition
//! the tool.

use prof_macros::prof;

use super::chain::StrategyChain;
use crate::dbg_log;
use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::shape::compute_polygon_bounds;
use crate::geo::shape::polygon::{
    get_polygons_closest_point, get_polygons_group_intersection,
    walk_polygon_vertices,
};
use crate::ops::assembly::Tracelet;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::search::walk_polygon_samples;
use crate::ops::cut::step;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::CutDirection;
use crate::ops::cut::StepStatus;
use crate::ops::cut::StepperOptions;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Point3D, Polygon};

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

/// Canonical probing function — single source of truth for "can the
/// tool move forward from `pos` with `heading`?"
///
/// Calls `step` with the exact same [`StepperOptions`] the main loop
/// uses (passed via [`ResumeCtx::step_opts`]), so the probing decision
/// is architecturally guaranteed to match what the main loop would
/// decide.  `predicted_angle` is 0.0 because the tool has no heading
/// momentum when placed at a resume point (gyro is reset).
///
/// Returns `Some(ToolPose)` when the stepper finds valid engagement
/// with positive cut area, `None` otherwise.
#[prof]
pub fn probe(
    ctx: &ResumeCtx,
    _radius: f64,
    pos: Point3D,
    heading: f64,
) -> Option<ToolPose> {
    let result = step(
        ctx.cleared,
        Point::new(pos.x, pos.y),
        heading,
        0.0,
        ctx.step_opts,
    );
    if result.status != StepStatus::Ok || result.cut_area <= 0.0 {
        return None;
    }
    Some(ToolPose {
        pos,
        heading: result.heading,
    })
}

// ── Frontier offset helpers ──────────────────────────────────────────

/// Engagement-angle tolerance below which a candidate's disk is treated
/// as fully inside the cleared area.
const CLEARANCE_ANGLE_EPS: f64 = 1e-9;

/// Derive the unit direction deeper into the cleared area from the
/// nearest cleared-boundary point.
///
/// Returns `None` when the candidate sits essentially on the boundary
/// (the offset vector is degenerate).
#[prof]
fn inward_from_boundary(
    cleared: &ClearedArea,
    candidate: Point,
) -> Option<Point> {
    let fragments = cleared.fragments();
    let (_, _, cp, _) = get_polygons_closest_point(fragments, candidate)?;
    let to_cand = candidate - cp;
    let dist = to_cand.length();
    if dist < 1e-6 {
        return None;
    }
    let outward = to_cand / dist;
    let signed = cleared.signed_boundary_distance(candidate.x, candidate.y);
    Some(if signed < 0.0 {
        outward
    } else {
        Point::new(-outward.x, -outward.y)
    })
}

/// March from `candidate` along unit `dir` until the disk is fully
/// inside the cleared area, without leaving the valid tool area.
///
/// Returns the first position where the disc is clear, or `None` if the
/// march exits the envelope or exceeds the step bound.
#[prof]
fn march_to_clear(
    ctx: &ResumeCtx,
    cleared: &ClearedArea,
    radius: f64,
    candidate: Point,
    dir: Point,
    step: f64,
    max_steps: usize,
) -> Option<Point> {
    for s in 1..=max_steps {
        let pos = candidate + dir * (s as f64 * step);
        if !point_in_valid_area(pos, ctx.step_opts.valid_area) {
            return None;
        }
        if cleared.get_point_engagement(pos, radius).angle
            <= CLEARANCE_ANGLE_EPS
        {
            return Some(pos);
        }
    }
    None
}

/// Frontier probe: ensure the disc sits fully in cleared material,
/// then verify engagement with the full stepper solver.
///
/// The sample point lies on the boundary of the `frontier ∩ envelope`
/// polygon.  When the disc overlaps stock there, the candidate is
/// offset inward until the disc is clear.  The original tangent heading
/// is preserved — the stepper probes ±max_deflection from the tangent
/// and finds stock at the extreme deflection angle.
///
/// When no cleared offset exists (narrow band), the candidate is
/// rejected.
#[prof]
pub(super) fn offset_and_probe(
    ctx: &ResumeCtx,
    radius: f64,
    candidate: Point3D,
    heading: f64,
) -> Option<ToolPose> {
    let cleared = ctx.cleared;
    let candidate_2d = Point::new(candidate.x, candidate.y);

    if cleared.get_point_engagement(candidate_2d, radius).angle
        <= CLEARANCE_ANGLE_EPS
    {
        return probe(ctx, radius, candidate, heading);
    }

    let step = ctx.step_length * 0.25;
    let max_steps = (radius / step).ceil() as usize;

    if let Some(dir) = inward_from_boundary(cleared, candidate_2d) {
        if let Some(pos) = march_to_clear(
            ctx,
            cleared,
            radius,
            candidate_2d,
            dir,
            step,
            max_steps,
        ) {
            if let Some(probed) =
                probe(ctx, radius, Point3D::new(pos.x, pos.y, 0.0), heading)
            {
                return Some(probed);
            }
        }
    }

    let h = Point::new(heading.cos(), heading.sin());
    for dir in [Point::new(-h.y, h.x), Point::new(h.y, -h.x)] {
        if let Some(pos) = march_to_clear(
            ctx,
            cleared,
            radius,
            candidate_2d,
            dir,
            step,
            max_steps,
        ) {
            if let Some(probed) =
                probe(ctx, radius, Point3D::new(pos.x, pos.y, 0.0), heading)
            {
                return Some(probed);
            }
        }
    }

    None
}

// ── Shared walk engine ──────────────────────────────────────────────

/// Ray-march fallback type: when the offset position fails validity,
/// called as `(ctx, radius, on_boundary, into_cleared, offset)`.
/// Must return `Some(centre)` or `None` to skip the sample.
pub(super) type RayMarchFn =
    fn(&ResumeCtx, f64, Point, Point, f64) -> Option<Point>;

/// Configuration for [`walk_and_probe`].
pub(super) struct WalkProbeOptions {
    /// Walk **all** polygons (sorted by distance from `ref_pos`) instead
    /// of only the closest polygon. When `true`, `centered_samples` should
    /// typically also be `true`.
    pub walk_all: bool,
    /// Reference position for polygon distance sorting and start-point
    /// selection. `None` → use `ctx.segment_start.pos`.
    pub ref_pos: Option<Point3D>,
    /// Perpendicular offset distance from the sample point into cleared
    /// area. When `Some(d)`, each sample is offset by `d` along the
    /// perpendicular direction before being passed to `probe`.
    pub offset: Option<f64>,
    /// When `offset` is `Some`, which side of the walk tangent is "into
    /// cleared material".  `true` → left perpendicular (CW holes);
    /// `false` → right perpendicular (CCW outer polygons).
    pub cleared_on_left: bool,
    /// Ray-march fallback: when the offset position fails validity,
    /// the function is called to find a valid centre position.
    pub ray_march: Option<RayMarchFn>,
    /// Use centered edge-only samples (no vertex visits) instead of the
    /// default vertex + sub-edge sampling via [`walk_polygon_samples`].
    pub centered_samples: bool,
    /// Sample spacing multiplier (>1 = fewer samples, <1 = more samples).
    /// Default 1.0 uses `ctx.step_length` directly.
    pub sample_spacing_mult: f64,
}

impl Default for WalkProbeOptions {
    fn default() -> Self {
        Self {
            walk_all: false,
            ref_pos: None,
            offset: None,
            cleared_on_left: true,
            ray_march: None,
            centered_samples: false,
            sample_spacing_mult: 1.0,
        }
    }
}

/// Walk a group of boundary polygons, calling `probe` at each sample,
/// and return the first engaging `ToolPose`.
///
/// `probe` receives the sample point (on the boundary polygon, or the
/// offset position when `WalkProbeOptions::offset` is set) and the
/// walk-direction tangent (`heading`); it decides whether the position
/// engages.  This is where the frontier/envelope/island offset policy
/// differs — see [`ResumeFrontier`], [`ResumeEnvelope`], and
/// [`ResumeIsland`].
#[prof]
pub(super) fn walk_and_probe(
    ctx: &ResumeCtx,
    radius: f64,
    polys: &[Polygon],
    log_tag: &str,
    opts: WalkProbeOptions,
    mut probe: impl FnMut(&ResumeCtx, f64, Point3D, f64) -> Option<ToolPose>,
) -> Option<ToolPose> {
    let ref_pos = opts.ref_pos.unwrap_or(ctx.segment_start.pos);
    let ref_pos_2d = Point::new(ref_pos.x, ref_pos.y);
    let actual_forward = ctx.opts.cut_direction == CutDirection::Ccw;
    let sample_spacing = ctx.step_length * opts.sample_spacing_mult;
    let bl_sq_tol = (ctx.step_length * 0.25).powi(2);
    let has_offset = opts.offset.is_some();

    let poly_indices: Vec<usize> = if opts.walk_all {
        let mut indexed: Vec<(usize, f64)> = polys
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let min_d = p
                    .iter()
                    .map(|pt| {
                        (pt.x - ref_pos.x).powi(2) + (pt.y - ref_pos.y).powi(2)
                    })
                    .fold(f64::MAX, f64::min);
                (i, min_d)
            })
            .collect();
        indexed.sort_by(|a, b| {
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        indexed.into_iter().map(|(i, _)| i).collect()
    } else {
        match get_polygons_closest_point(polys, ref_pos_2d) {
            Some((idx, _, _, _)) => vec![idx],
            None => {
                dbg_log!("  {}  no suitable point found", log_tag);
                return None;
            }
        }
    };

    for poly_idx in poly_indices {
        let poly = &polys[poly_idx];
        let n = poly.len();
        if n < 3 {
            continue;
        }

        let result = if opts.centered_samples {
            walk_centered(
                poly,
                ref_pos_2d,
                actual_forward,
                sample_spacing,
                opts.cleared_on_left,
                |on_boundary, heading, into_cleared| {
                    let probe_pt = apply_offset(
                        ctx,
                        radius,
                        on_boundary,
                        into_cleared,
                        &opts,
                    )?;
                    probe(
                        ctx,
                        radius,
                        Point3D::new(probe_pt.x, probe_pt.y, 0.0),
                        heading,
                    )
                },
            )
        } else {
            let (start_idx, start_frac) = closest_edge_point(poly, ref_pos_2d);
            walk_polygon_samples(
                poly,
                start_idx,
                actual_forward,
                sample_spacing,
                false,
                start_frac,
                |pt, heading| {
                    if !has_offset {
                        for bl_pos in ctx.blacklist {
                            let dx = bl_pos.x - pt.x;
                            let dy = bl_pos.y - pt.y;
                            if dx * dx + dy * dy < bl_sq_tol {
                                return None;
                            }
                        }
                    }
                    probe(ctx, radius, Point3D::new(pt.x, pt.y, 0.0), heading)
                },
            )
        };

        if let Some(probed) = result {
            dbg_log!(
                "  {}  resume=({:.3},{:.3})  heading={:.4}",
                log_tag,
                probed.pos.x,
                probed.pos.y,
                probed.heading,
            );
            return Some(probed);
        }
    }

    dbg_log!("  {}  no suitable point found", log_tag);
    None
}

/// Find the closest edge point on `poly` to `ref_pos`, returning the
/// vertex index and fractional position along that edge.
fn closest_edge_point(poly: &Polygon, ref_pos: Point) -> (usize, f64) {
    let n = poly.len();
    let mut start_idx = 0usize;
    let mut start_frac = 0.0f64;
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
    (start_idx, start_frac)
}

/// Walk a polygon starting from the vertex closest to `ref_pos`,
/// sampling each edge at **centred** sub-intervals (no vertex visits).
///
/// `accept` receives `(on_boundary, heading, into_cleared)` where
/// `into_cleared` is the unit perpendicular pointing into cleared
/// material according to `cleared_on_left`.
fn walk_centered<F>(
    poly: &Polygon,
    ref_pos: Point,
    forward: bool,
    sample_spacing: f64,
    cleared_on_left: bool,
    mut accept: F,
) -> Option<ToolPose>
where
    F: FnMut(Point, f64, Point) -> Option<ToolPose>,
{
    let n = poly.len();
    let start_idx = poly
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(ref_pos)
                .partial_cmp(&b.distance_squared(ref_pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    walk_polygon_vertices(poly, start_idx, forward, 1, |idx, _pt| {
        let next = (idx + 1) % n;
        let edge = poly[next] - poly[idx];
        let elen = edge.length();
        if elen < 1e-12 {
            return None;
        }
        let into_cleared = if cleared_on_left {
            Point::new(-edge.y, edge.x) / elen
        } else {
            Point::new(edge.y, -edge.x) / elen
        };
        let heading = if forward {
            edge.y.atan2(edge.x)
        } else {
            (-edge.y).atan2(-edge.x)
        };
        let n_samples = ((elen / sample_spacing).ceil() as usize).max(1);
        for si in 0..n_samples {
            let frac = (si as f64 + 0.5) / n_samples as f64;
            let on_boundary = poly[idx] + edge * frac;
            if let Some(r) = accept(on_boundary, heading, into_cleared) {
                return Some(r);
            }
        }
        None
    })
}

/// Apply the perpendicular offset from `on_boundary` into cleared area,
/// using the `ray_march` fallback when configured.
#[prof]
fn apply_offset(
    ctx: &ResumeCtx,
    radius: f64,
    on_boundary: Point,
    into_cleared: Point,
    opts: &WalkProbeOptions,
) -> Option<Point> {
    let offset_dist = opts.offset?;
    if let Some(ray_march) = opts.ray_march {
        ray_march(ctx, radius, on_boundary, into_cleared, offset_dist)
    } else {
        Some(on_boundary + into_cleared * offset_dist)
    }
}

/// Emit a resume travel from `from` to `to` using the routing strategies.
///
/// Returns `Ok(RouteSource)` on success, `Err(RaygeoError::RoutingError)`
/// when no collision-free path could be found.
#[prof]
pub fn emit_resume_travel(
    trace: &mut Tracelet,
    cleared: &ClearedArea,
    mat: Option<&MedialAxis>,
    from: Point3D,
    to: Point3D,
    opts: &AdaptiveClearingOptions,
    out_route_details: Option<&mut [u8; 4]>,
) -> RaygeoResult<routing::RouteSource> {
    // Obstacles = remaining (uncut) material + islands (permanent no-go zones).
    // Clip remaining to the tool-centre envelope so that thin wall slivers
    // (which are unreachable by the tool centre and too narrow to matter)
    // are not treated as routing obstacles.  Interior uncut islands are
    // kept and correctly avoided.
    let envelope = cleared.envelope(opts.tool_radius);
    let mut obstacles = if envelope.is_empty() {
        cleared.remaining()
    } else {
        get_polygons_group_intersection(&cleared.remaining(), &envelope)
    };
    obstacles.extend(opts.islands.iter().cloned());
    let obs_bounds = compute_polygon_bounds(&obstacles);

    let ctx = routing::RouteCtx {
        cleared,
        opts,
        mat,
        obstacles: &obstacles,
        obstacle_bounds: &obs_bounds,
    };

    let mut route_details = [0u8; 4];
    if let Some((source, path)) =
        routing::optimize_route(&ctx, from, to, &mut route_details)
    {
        dbg_log!(
            "  EMIT  route={}:{}  n={}",
            source as u8,
            routing::source_label(source),
            path.len(),
        );
        for pt in &path[1..] {
            trace.move_to(pt.x, pt.y, pt.z, None);
        }
        if let Some(out) = out_route_details {
            *out = route_details;
        }
        Ok(source)
    } else {
        if let Some(out) = out_route_details {
            *out = route_details;
        }
        let detail_str = route_details
            .iter()
            .enumerate()
            .map(|(i, &d)| {
                format!(
                    "{}({})",
                    ["direct", "frontier", "mat", "astar", "zhop"][i],
                    routing::route_detail_label(d),
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        Err(RaygeoError::RoutingError(format!(
            "cannot route from ({:.3},{:.3}) to ({:.3},{:.3})  \
             ({})",
            from.x, from.y, to.x, to.y, detail_str,
        )))
    }
}

// ── ResumeSource enum (renamed) ─────────────────────────────────────

/// Which resume mechanism succeeded, in priority order.
#[derive(Clone, Copy, PartialEq, Eq, num_enum::TryFromPrimitive)]
#[repr(u8)]
pub enum ResumeSource {
    /// No strategy was tried / applicable (trace-record sentinel).
    None = 0,
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
    pub step_opts: &'a StepperOptions<'a>,
    /// Radial step into material (derived from `step_over`).
    pub advance: f64,
    /// Forward step length (derived from `step_over`).
    pub step_length: f64,
    pub mat: Option<&'a MedialAxis>,
    /// Position and heading where the current cutting segment began.
    /// ResumeSegment probes forward from here using the stored heading.
    pub segment_start: ToolPose,
    pub last_resume_area: f64,
    pub last_resume_pos: Point3D,
    /// Wall-hug points accumulated across envelope visits in the
    /// current cut segment.  Each entry is the minimum-distance pose
    /// recorded during one envelope visit.  Consumed by
    /// [`ResumeWallHug`] which tries them in order (FIFO).
    pub wall_hug_points: &'a [ToolPose],
    /// Positions that have already been tried and led to immediate stalls.
    /// Any strategy that produces a position too close to one of these is
    /// rejected.
    pub blacklist: &'a [Point3D],
}

// ── ResumeStrategy trait ────────────────────────────────────────────

/// A strategy that finds a new tool position when the stepper stalls.
pub trait ResumeStrategy {
    fn find_next(
        &self,
        ctx: &ResumeCtx,
        tool: &Tool,
        detail: &mut u8,
    ) -> Option<ToolPose>;
    fn label(&self) -> &'static str;
}

// ── try_resume orchestrator ──────────────────────────────────────────

/// Per-strategy outcome codes emitted via [`try_resume`].
/// Index 0-5 match the priority order (WallHug, Segment, Mat,
/// Frontier, Envelope, Island).
pub type ResumeReasons = [u8; 6];

/// Reason codes for resume-strategy outcomes stored in trace records.
/// 0 = not tried, 1 = find_next returned None (no candidate),
/// 2 = candidate was blacklisted (already stalled this position).
pub const REASON_NOT_TRIED: u8 = 0;
pub const REASON_NO_CANDIDATE: u8 = 1;
pub const REASON_BLACKLISTED: u8 = 2;

/// Detail codes for resume-strategy failure, stored in a parallel
/// `[u8; 6]` next to the reason codes.  Each byte gives context for
/// *why* the strategy returned `None`.
///
/// Values are shared across strategies — meaning depends on which
/// strategy index they appear in.
pub const DETAIL_NOT_TRIED: u8 = 0;
pub const DETAIL_NO_FRAGMENTS: u8 = 1;
pub const DETAIL_NO_GROWTH: u8 = 2;
pub const DETAIL_OUTSIDE_VALID: u8 = 3;
pub const DETAIL_NO_WALL_HUG_POINT: u8 = 4;
pub const DETAIL_NODE_NOT_CLEARED: u8 = 5;
pub const DETAIL_NO_CROSSING: u8 = 6;
pub const DETAIL_NO_ENVELOPE: u8 = 7;
pub const DETAIL_NO_FRONTIER: u8 = 8;
pub const DETAIL_NO_POLYGONS: u8 = 9;
pub const DETAIL_NO_HOLES: u8 = 10;
pub const DETAIL_NO_ENGAGEMENT: u8 = 11;
pub const DETAIL_BLACKLISTED: u8 = 12;
pub const DETAIL_NO_WALL_HIT: u8 = 13;

/// Check that the cleared area has at least one fragment polygon.
/// Returns `Some(fragments)` when non-empty, or `None` with
/// `*detail = DETAIL_NO_FRAGMENTS` when empty.
pub(super) fn require_fragments<'a>(
    ctx: &'a ResumeCtx<'a>,
    detail: &mut u8,
) -> Option<&'a [Polygon]> {
    let f = ctx.cleared.fragments();
    if f.is_empty() {
        *detail = DETAIL_NO_FRAGMENTS;
        None
    } else {
        Some(f)
    }
}

/// Per-strategy candidate positions emitted via [`try_resume`].
/// Index 0-5 match the strategy priority order (WallHug, Segment,
/// Mat, Frontier, Envelope, Island).  `None` if the strategy did not
/// produce a candidate.
pub type ResumeCandidatePoints = [Option<Point3D>; 6];

/// Try each strategy in priority order.  Returns the winning strategy
/// and the tool pose to apply.  When no strategy succeeds, `reasons`
/// is filled with one byte per strategy
/// (0 = not tried, 1 = no candidate, 2 = blacklisted) and `details`
/// is filled with a per-strategy detail code.
/// `candidate_pts` is filled with the position each strategy
/// produced (if any).
/// Pure query — the caller handles all mutation.
#[prof]
pub fn try_resume(
    ctx: &ResumeCtx,
    tool: &Tool,
    reasons: &mut ResumeReasons,
    details: &mut ResumeReasons,
    candidate_pts: &mut ResumeCandidatePoints,
) -> Option<(ResumeSource, ToolPose)> {
    let area_grew = ctx.cleared.total_area() > ctx.last_resume_area + 1e-9;
    let tool_dx = tool.pos.x - ctx.last_resume_pos.x;
    let tool_dy = tool.pos.y - ctx.last_resume_pos.y;
    let tool_moved = (tool_dx * tool_dx + tool_dy * tool_dy)
        > (ctx.step_length * 0.25).powi(2);

    dbg_log!(
        "RESUME  from=({:.3},{:.3})  heading={:.4}  R={:.1}  \
         advance={:.3}  step_len={:.3}  last_area={:.3}  cur_area={:.3}",
        tool.pos.x,
        tool.pos.y,
        tool.heading,
        ctx.opts.tool_radius,
        ctx.advance,
        ctx.step_length,
        ctx.last_resume_area,
        ctx.cleared.total_area(),
    );

    if !area_grew && !tool_moved {
        dbg_log!("  RESUME  no change since last resume — giving up");
        return None;
    }

    let sq_tol = (ctx.step_length * 0.25).powi(2);
    *candidate_pts = [None; 6];

    // Try each strategy in priority order.  Each strategy decides for
    // itself whether it is responsible (e.g. WallHug returns None when
    // there is no stored wall-hug pose).
    let mut chain: StrategyChain<&dyn ResumeStrategy, ResumeSource, 6> =
        StrategyChain::new([
            (&ResumeWallHug, ResumeSource::ResumeWallHug),
            (&ResumeSegment, ResumeSource::ResumeSegment),
            (&ResumeMat, ResumeSource::ResumeMat),
            (&ResumeFrontier, ResumeSource::ResumeFrontier),
            (&ResumeEnvelope, ResumeSource::ResumeEnvelope),
            (&ResumeIsland, ResumeSource::ResumeIsland),
        ]);

    let result = chain.run(
        |idx, s, source, detail| {
            let outcome = s.find_next(ctx, tool, detail);
            let Some(rp) = outcome else {
                reasons[idx] = REASON_NO_CANDIDATE;
                return None;
            };
            candidate_pts[idx] = Some(rp.pos);
            // Reject any position that has already been tried and led to
            // an immediate stall, regardless of which strategy produced it.
            for bl_pos in ctx.blacklist {
                let dx = bl_pos.x - rp.pos.x;
                let dy = bl_pos.y - rp.pos.y;
                if dx * dx + dy * dy < sq_tol {
                    reasons[idx] = REASON_BLACKLISTED;
                    *detail = DETAIL_BLACKLISTED;
                    dbg_log!(
                        "  RESUME  {}={}  → ({:.3},{:.3})  BLACKLISTED",
                        source as u8,
                        s.label(),
                        rp.pos.x,
                        rp.pos.y,
                    );
                    return None;
                }
            }
            dbg_log!(
                "  RESUME  {}={}  → ({:.3},{:.3})  heading={:.4}",
                source as u8,
                s.label(),
                rp.pos.x,
                rp.pos.y,
                rp.heading,
            );
            Some(rp)
        },
        None::<
            fn(
                usize,
                &dyn ResumeStrategy,
                ResumeSource,
                &mut u8,
                ToolPose,
            ) -> Option<ToolPose>,
        >,
    );

    *details = *chain.details();
    result
}
