use prof_macros::prof;

use crate::dbg_log;
use crate::geo::shape::polygon::get_polygon_signed_area;
use crate::ops::assembly::adaptive::resume::{
    probe, walk_and_probe, ResumeCtx, ResumeStrategy, WalkProbeOptions,
    DETAIL_NO_ENGAGEMENT, DETAIL_NO_HOLES,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

pub struct ResumeIsland;

impl ResumeStrategy for ResumeIsland {
    fn label(&self) -> &'static str {
        "island"
    }

    fn find_next(
        &self,
        ctx: &ResumeCtx,
        tool: &Tool,
        detail: &mut u8,
    ) -> Option<ToolPose> {
        frontier_hole_resume(ctx, tool, detail)
    }
}

/// Ray-march from the frontier boundary into cleared area until the
/// tool disk is fully inside the cleared region and the centre is inside
/// the valid tool envelope.
///
/// First checks the basic offset position; if invalid, marches further
/// along `into_cleared` at increments of `step_length * 0.25`.
fn island_ray_march(
    ctx: &ResumeCtx,
    radius: f64,
    on_boundary: Point,
    into_cleared: Point,
    offset: f64,
) -> Option<Point> {
    let dist_to_cleared =
        |p: Point| ctx.cleared.signed_boundary_distance(p.x, p.y);

    let centre = on_boundary + into_cleared * offset;
    if point_in_valid_area(centre, ctx.step_opts.valid_area)
        && dist_to_cleared(centre) <= -radius
    {
        return Some(centre);
    }

    let step = ctx.opts.step_length * 0.25;
    let max_steps = (radius * 4.0 / step).ceil() as usize;
    for s in 1..=max_steps {
        let candidate = on_boundary + into_cleared * (offset + s as f64 * step);
        if !point_in_valid_area(candidate, ctx.step_opts.valid_area) {
            continue;
        }
        if dist_to_cleared(candidate) > -radius {
            continue;
        }
        return Some(candidate);
    }
    None
}

/// Walk the CW hole boundaries of [`ClearedArea::frontier`] and probe
/// for re-engagement.
///
/// The frontier returns the unioned cleared region clipped to stock.
/// Around islands (and any uncleared bulge adjacent to them) this
/// produces **CW hole polygons** whose outline is the exact boundary
/// between cleared and uncleared material.
///
/// The strategy walks each hole boundary, placing the tool centre on
/// the cleared side at distance `radius − advance` (the normal
/// engagement depth).  At each sample point the tool probes forward
/// along the travel tangent; the first position where the stepper
/// finds productive engagement becomes the resume target.
///
/// Travel direction respects `cut_direction`: for CCW cutting the
/// tool walks CW around holes (storage order), keeping uncut material
/// on the right — matching the stepper's one-sided deflection bounds.
#[prof]
fn frontier_hole_resume(
    ctx: &ResumeCtx,
    tool: &Tool,
    detail: &mut u8,
) -> Option<ToolPose> {
    let frontier = ctx.cleared.frontier(0.001);

    let holes: Vec<Polygon> = frontier
        .iter()
        .filter(|p| p.len() >= 3 && get_polygon_signed_area(p) < -0.5)
        .cloned()
        .collect();

    if holes.is_empty() {
        *detail = DETAIL_NO_HOLES;
        return None;
    }

    dbg_log!(
        "  ISLAND  {} frontier hole(s) ({} total frontier polys)",
        holes.len(),
        frontier.len(),
    );

    let radius = tool.radius;
    let advance = ctx.opts.advance;
    let offset = (radius - advance).max(0.0);

    let result = walk_and_probe(
        ctx,
        radius,
        &holes,
        "ISLAND",
        WalkProbeOptions {
            walk_all: true,
            ref_pos: Some(tool.pos),
            offset: Some(offset),
            cleared_on_left: true,
            ray_march: Some(island_ray_march),
            centered_samples: true,
            sample_spacing_mult: 1.0,
        },
        probe,
    );

    if result.is_none() {
        dbg_log!("  ISLAND  no engagement found on any frontier hole");
        *detail = DETAIL_NO_ENGAGEMENT;
    }
    result
}
