use crate::dbg_log;
use crate::geo::shape::polygon::{
    get_polygon_signed_area, get_polygons_closest_point,
};
use crate::ops::assembly::adaptive::resume::{
    probe_step, ResumeCtx, ResumeStrategy,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::CutDirection;
use crate::ops::cut::ToolPose;
use crate::types::Point;

pub struct ResumeBoundary;

impl ResumeStrategy for ResumeBoundary {
    const NAME: &'static str = "ResumeBoundary";

    fn find_next(&self, ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
        frontier_resume(ctx, tool)
    }
}

/// Find any engagement point on the cleared-area frontier and place
/// the tool so its disk just touches the frontier at that point.
///
/// The walk direction is derived directly from `opts.cut_direction`
/// and the polygon winding: the tool walks along the frontier in the
/// cutting rotational direction, probing inward-offset positions for
/// stepper engagement.
fn frontier_resume(ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
    let polys = ctx.cleared.frontier(0.001);
    if polys.is_empty() {
        return None;
    }

    let (poly_idx, _, _, _) = get_polygons_closest_point(&polys, tool.pos)?;
    let poly = &polys[poly_idx];
    let n = poly.len();
    if n < 3 {
        return None;
    }

    let signed_area = get_polygon_signed_area(poly);
    let poly_ccw = signed_area > 0.0;

    let start_idx = poly
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(tool.pos)
                .partial_cmp(&b.distance_squared(tool.pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)?;

    // Walk in the cutting rotational direction along the frontier.
    // Increasing index is geometric CCW on a CCW-wound poly and CW on
    // a CW-wound poly.  CCW cutting walks geometric CCW → increasing
    // index when the poly is CCW, decreasing when CW.
    let walk_increasing =
        (ctx.opts.cut_direction == CutDirection::Ccw) == poly_ccw;

    let min_dist2 = (tool.radius * 2.0).powi(2);

    for offset in 0..n {
        let idx = if walk_increasing {
            (start_idx + offset) % n
        } else {
            (start_idx + n - offset) % n
        };
        let pt = poly[idx];

        let dx = pt.x - tool.pos.x;
        let dy = pt.y - tool.pos.y;
        if dx * dx + dy * dy < min_dist2 {
            continue;
        }

        let next_i = (idx + 1) % n;
        let prev_i = (idx + n - 1) % n;
        let edge = (poly[next_i] - poly[prev_i]) * 0.5;
        let elen = edge.length();
        if elen < 1e-9 {
            continue;
        }
        // Inward normal points from the frontier toward the cleared
        // area interior.  On a CCW polygon the interior is to the
        // left of the edge direction.
        let inward = if poly_ccw {
            Point::new(-edge.y, edge.x) / elen
        } else {
            Point::new(edge.y, -edge.x) / elen
        };

        let p_resume = pt + inward * tool.radius;
        // Heading: frontier tangent in the walk direction.
        let tangent_pt = if walk_increasing {
            poly[next_i]
        } else {
            poly[prev_i]
        };
        let t = tangent_pt - poly[idx];
        let heading = t.y.atan2(t.x);

        if let Some(probed) = probe_step(ctx, tool.radius, p_resume, heading) {
            dbg_log!(
                "  FRONTIER  resume=({:.3},{:.3})  heading={:.4}  \
                 pt=({:.3},{:.3})  offset={}",
                probed.pos.x,
                probed.pos.y,
                probed.heading,
                pt.x,
                pt.y,
                offset,
            );
            return Some(probed);
        }
    }

    dbg_log!("  FRONTIER  no suitable point found");
    None
}
