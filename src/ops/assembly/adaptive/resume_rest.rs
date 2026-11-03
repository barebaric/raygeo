//! Rest resume strategy: machine the remainder the pruned mat cannot see.
//!
//! The medial axis is pruned to tool clearance, so a remainder **narrower
//! than the tool** — the crescent between the entry seed and the first
//! spiral turn, the taper at the end of a cleared core — has no spine to
//! follow and is invisible to the mat-based strategies. The tool can
//! still machine it from the cleared space around it: its boundary is
//! the sweep, not a pocket wall, so a tool centred on the clump engages
//! only the clump's thin cross-section. This strategy proposes the
//! remaining clumps' own points, nearest the tool first, and lets the
//! stepper's engagement check decide — a wall-band clump fails that
//! check and is skipped, as does a clump the tool genuinely cannot
//! reach.

use prof_macros::prof;

use super::resume::{offset_and_probe, ResumeCtx, ResumeStrategy};
use super::tool::Tool;
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_centroid, get_polygons_closest_point,
    is_point_in_polygon,
};
use crate::geo::types::{Point, Point3D, Polygon};
use crate::ops::types::ToolPose;

/// Resume inside the remaining material itself.
pub struct ResumeRest;

impl ResumeStrategy for ResumeRest {
    fn label(&self) -> &'static str {
        "rest"
    }

    fn find_next(
        &self,
        ctx: &ResumeCtx,
        tool: &Tool,
        detail: &mut u8,
    ) -> Option<ToolPose> {
        rest_resume(ctx, tool, detail)
    }
}

/// Detail codes for the rest strategy.
const DETAIL_REST_NO_CANDIDATE: u8 = 42;

/// Propose a point on the remaining material nearest the tool.
///
/// Clumps smaller than the leave-behind tolerance are the convergence
/// check's business, not a resume's. Candidate points are the clumps'
/// vertices and edge midpoints (plus the centroid when it falls inside),
/// keeping only points that keep tool clearance from the pocket walls.
/// A standing clump narrower than the tool cannot be entered — centred
/// in it the tool over-engages — so each candidate is probed through
/// [`offset_and_probe`] like every other strategy's: the probe marches
/// the candidate into the cleared space beside the clump and grazes the
/// clump's edge, eroding it a bite at a time. Headings run along the
/// clump's edge (both ways) before pointing into it.
#[prof]
fn rest_resume(
    ctx: &ResumeCtx,
    tool: &Tool,
    detail: &mut u8,
) -> Option<ToolPose> {
    let remaining = ctx.part.cleared.remaining(&ctx.part.stock_region);
    let clumps: Vec<&Polygon> = remaining
        .iter()
        .filter(|p| get_polygon_area(p) > ctx.opts.area_tolerance)
        .collect();
    if clumps.is_empty() {
        *detail = DETAIL_REST_NO_CANDIDATE;
        return None;
    }

    let tool_2d = Point::new(tool.pos.x, tool.pos.y);
    let wall_polys: Vec<Polygon> =
        std::iter::once(ctx.part.stock_region.boundary.clone())
            .chain(ctx.part.stock_region.islands.iter().cloned())
            .collect();
    let clearance2 = (ctx.opts.tool_radius - 1e-9).powi(2);

    // Boundary points of the standing clumps with their edge's tangent
    // headings, nearest the tool first: (point, tangent, centroid).
    let mut candidates: Vec<(Point, f64, Point)> = Vec::new();
    for clump in &clumps {
        let centroid = get_polygon_centroid(clump);
        let n = clump.len();
        for i in 0..n {
            let a = clump[i];
            let b = clump[(i + 1) % n];
            let edge = (b.y - a.y).atan2(b.x - a.x);
            candidates.push((a, edge, centroid));
            candidates.push((
                Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5),
                edge,
                centroid,
            ));
        }
        if is_point_in_polygon(centroid, clump) {
            candidates.push((centroid, 0.0, centroid));
        }
    }
    candidates.retain(|(p, _, _)| {
        p.distance(tool_2d) > 0.0
            && get_polygons_closest_point(&wall_polys, *p)
                .is_none_or(|(_, _, _, d2)| d2 >= clearance2)
    });
    candidates.sort_by(|a, b| {
        a.0.distance(tool_2d).total_cmp(&b.0.distance(tool_2d))
    });

    let blacklisted = |p: Point| {
        ctx.blacklist.iter().any(|bl| {
            Point::new(bl.x, bl.y).distance(p) < ctx.step_length * 0.25
        })
    };
    for (p, edge, centroid) in &candidates {
        if blacklisted(*p) {
            continue;
        }
        // Graze first: eroding the clump's edge from the cleared side is
        // what a narrower-than-tool clump admits. Heading into the
        // clump is the last resort for clumps wide enough to enter.
        let to_centroid = (centroid.y - p.y).atan2(centroid.x - p.x);
        for heading in [*edge, *edge + std::f64::consts::PI, to_centroid] {
            if let Some(probed) = offset_and_probe(
                ctx,
                tool.radius,
                Point3D::new(p.x, p.y, ctx.opts.target_z),
                heading,
            ) {
                return Some(probed);
            }
        }
    }
    *detail = DETAIL_REST_NO_CANDIDATE;
    None
}
