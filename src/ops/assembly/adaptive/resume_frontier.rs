use prof_macros::prof;

use crate::geo::shape::polygon::get_polygons_group_intersection;
use crate::ops::assembly::adaptive::resume::{
    boundary_probe, walk_and_probe, ResumeCtx, ResumeStrategy,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::ToolPose;

pub struct ResumeFrontier;

impl ResumeStrategy for ResumeFrontier {
    fn label(&self) -> &'static str {
        "frontier"
    }

    fn find_next(&self, ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
        frontier_resume(ctx, tool)
    }
}

/// Walk the cleared-area **frontier** (the material boundary between
/// cleared and uncleared stock) clipped to the tool-centre envelope,
/// probing for re-engagement.
///
/// The frontier is intersected with the tool-centre **envelope** so
/// only the portion where the tool centre is legally allowed is
/// walked.  Each sample is checked with [`boundary_probe`].
///
/// Contrast with [`super::resume_envelope::ResumeEnvelope`], which
/// walks the tool-centre envelope directly rather than the frontier.
#[prof]
fn frontier_resume(ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
    let envelope = ctx.cleared.envelope(tool.radius);
    if envelope.is_empty() {
        return None;
    }
    let frontier = ctx.cleared.frontier(0.001);
    if frontier.is_empty() {
        return None;
    }
    let polys = get_polygons_group_intersection(&frontier, &envelope);
    if polys.is_empty() {
        return None;
    }
    walk_and_probe(ctx, tool.radius, &polys, "FRONTIER", boundary_probe)
}
