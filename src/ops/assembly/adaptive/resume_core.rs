//! Core resume strategy: propose the spine of the uncleared remainder.
//!
//! When the wavefront stalls on a remaining core thinner than the advance,
//! the other strategies propose wall-adjacent or frontier points — but the
//! core itself can be machined by following its spine: the medial axis,
//! pruned to tool clearance, runs down the middle of the remainder, so a
//! tool centred on it fits and engages the core walls. A mat **edge**
//! belongs to the core when its midpoint is uncleared — the wavefront may
//! have cleared the junctions at either end while the ridge between them
//! stands.

use prof_macros::prof;

use super::resume::{offset_and_probe, ResumeCtx, ResumeStrategy};
use super::tool::Tool;
use crate::geo::algo::medial_axis::MedialAxis;
use crate::geo::types::{Point, Point3D};
use crate::ops::types::ToolPose;

/// Resume from the spine of the uncleared core.
pub struct ResumeCore;

impl ResumeStrategy for ResumeCore {
    fn label(&self) -> &'static str {
        "core"
    }

    fn find_next(
        &self,
        ctx: &ResumeCtx,
        tool: &Tool,
        detail: &mut u8,
    ) -> Option<ToolPose> {
        core_resume(ctx, tool, detail)
    }
}

/// Detail codes for the core strategy.
const DETAIL_CORE_NO_CANDIDATE: u8 = 41;

/// Propose the point on the uncleared core spine nearest the tool.
///
/// A mat edge is a core edge when both endpoints keep tool clearance (the
/// mat is pruned to tool clearance, so this only guards the degenerate
/// case) and its midpoint is uncleared — the wavefront may have cleared
/// the junctions at either end while the ridge between them stands.
/// Candidates are the core-edge midpoints, nearest the tool first; each is
/// probed through [`offset_and_probe`] like every other strategy, falling
/// back to the edge's endpoints. The heading runs along the edge, toward
/// the side the tool is on.
#[prof]
fn core_resume(
    ctx: &ResumeCtx,
    tool: &Tool,
    detail: &mut u8,
) -> Option<ToolPose> {
    let axis: &MedialAxis = ctx.mat?;
    let fragments = ctx.part.cleared.fragments();
    let is_cleared = axis.build_cleared_mask(fragments);
    let tool_2d = Point::new(tool.pos.x, tool.pos.y);

    // Core-edge candidates nearest the tool first: (midpoint, ends, dist).
    let mut candidates: Vec<(Point, (Point, Point), f64)> = axis
        .edges
        .iter()
        .filter_map(|&(a, b)| {
            let (na, nb) = (&axis.nodes[a], &axis.nodes[b]);
            // Both ends keep tool clearance and the edge's middle is
            // uncleared.
            if na.clearance < ctx.opts.tool_radius - 1e-9
                || nb.clearance < ctx.opts.tool_radius - 1e-9
            {
                return None;
            }
            let mid = Point::new(
                (na.point.x + nb.point.x) * 0.5,
                (na.point.y + nb.point.y) * 0.5,
            );
            let (ia, ib) = (
                is_cleared.get(a).copied().unwrap_or(true),
                is_cleared.get(b).copied().unwrap_or(true),
            );
            if ia || ib {
                return None;
            }
            let ends = (na.point, nb.point);
            Some((mid, ends, mid.distance(tool_2d)))
        })
        .collect();
    if candidates.is_empty() {
        *detail = DETAIL_CORE_NO_CANDIDATE;
        return None;
    }
    candidates.sort_by(|a, b| a.2.total_cmp(&b.2));

    let blacklisted = |p: Point| {
        ctx.blacklist.iter().any(|bl| {
            Point::new(bl.x, bl.y).distance(p) < ctx.step_length * 0.25
        })
    };

    for (mid, ends, _) in &candidates {
        // Head along the edge, toward the side the tool is on.
        let ab = Point::new(ends.1.x - ends.0.x, ends.1.y - ends.0.y);
        let mut heading = ab.y.atan2(ab.x);
        let to_mid = Point::new(mid.x - tool_2d.x, mid.y - tool_2d.y);
        if ab.dot(to_mid) < 0.0 {
            heading += std::f64::consts::PI;
        }
        if blacklisted(*mid) {
            continue;
        }
        if let Some(probed) = offset_and_probe(
            ctx,
            tool.radius,
            Point3D::new(mid.x, mid.y, ctx.opts.target_z),
            heading,
        ) {
            return Some(probed);
        }
        // The midpoint probe can fail when the core material near the
        // middle is thinner than the engagement check tolerates, while the
        // spine's ends still machine.
        for p in [ends.0, ends.1] {
            if blacklisted(p) {
                continue;
            }
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
    *detail = DETAIL_CORE_NO_CANDIDATE;
    None
}
