use prof_macros::prof;

use crate::geo::shape::polygon::{
    get_polygon_signed_area, get_polygons_group_intersection,
};
use crate::ops::assembly::adaptive::resume::{
    offset_and_probe, walk_and_probe, ResumeCtx, ResumeStrategy,
    WalkProbeOptions, DETAIL_NO_ENVELOPE, DETAIL_NO_FRONTIER,
    DETAIL_NO_POLYGONS,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::ToolPose;
use crate::types::Polygon;

pub struct ResumeFrontier;

impl ResumeStrategy for ResumeFrontier {
    fn label(&self) -> &'static str {
        "frontier"
    }

    fn find_next(
        &self,
        ctx: &ResumeCtx,
        tool: &Tool,
        detail: &mut u8,
    ) -> Option<ToolPose> {
        frontier_resume(ctx, tool, detail)
    }
}

/// Walk the cleared-area **frontier** (the material boundary between
/// cleared and uncleared stock), preserving CW hole polygons that
/// wrap around island exclusion zones.
///
/// CCW outer polygons are clipped to the tool-centre **envelope** so
/// only the portion where the tool centre is legally allowed is
/// walked.  CW holes are kept unclipped — they lie inside the expanded
/// island zone (outside the envelope) but their boundary is where
/// cleared area meets remaining stock; [`offset_and_probe`] handles
/// offsetting the tool centre into valid space before verifying
/// engagement.
///
/// Contrast with [`super::resume_envelope::ResumeEnvelope`], which
/// walks the tool-centre envelope directly (no offset needed).
#[prof]
fn frontier_resume(
    ctx: &ResumeCtx,
    tool: &Tool,
    detail: &mut u8,
) -> Option<ToolPose> {
    let envelope = ctx.cleared.envelope(tool.radius);
    if envelope.is_empty() {
        *detail = DETAIL_NO_ENVELOPE;
        return None;
    }
    let frontier = ctx.cleared.frontier(0.001);
    if frontier.is_empty() {
        *detail = DETAIL_NO_FRONTIER;
        return None;
    }

    // Separate CCW outer polygons from CW holes.
    // CCW outer polygons are clipped to the envelope so we only walk
    // reachable tool-centre positions.
    // CW holes (island-adjacent material boundaries) are kept unclipped
    // — they may lie outside the envelope but their boundary is where
    // cleared area meets remaining stock.  offset_and_probe handles
    // offsetting the tool centre into valid space before verifying
    // engagement.
    let mut polys: Vec<Polygon> = Vec::new();
    for poly in &frontier {
        if poly.len() < 3 {
            continue;
        }
        if get_polygon_signed_area(poly) >= 0.0 {
            let clipped = get_polygons_group_intersection(
                std::slice::from_ref(poly),
                &envelope,
            );
            polys.extend(clipped);
        } else {
            polys.push(poly.clone());
        }
    }
    if polys.is_empty() {
        *detail = DETAIL_NO_POLYGONS;
        return None;
    }

    walk_and_probe(
        ctx,
        tool.radius,
        &polys,
        "FRONTIER",
        WalkProbeOptions::default(),
        offset_and_probe,
    )
}
