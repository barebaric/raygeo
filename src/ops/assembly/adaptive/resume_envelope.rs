use prof_macros::prof;

use crate::ops::assembly::adaptive::resume::{
    boundary_probe, walk_and_probe, ResumeCtx, ResumeStrategy,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::ToolPose;

pub struct ResumeEnvelope;

impl ResumeStrategy for ResumeEnvelope {
    fn label(&self) -> &'static str {
        "envelope"
    }

    fn find_next(&self, ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
        envelope_resume(ctx, tool)
    }
}

/// Walk the tool-centre **envelope** (the pocket inset by the tool
/// radius) and find a re-engagement point.
///
/// The envelope is a *tool-centre* boundary: it is the locus of legal
/// tool-centre positions, so the tool centre sits directly on the
/// envelope edge — no inward offset is applied.  Each sample is checked
/// with [`boundary_probe`].
#[prof]
fn envelope_resume(ctx: &ResumeCtx, tool: &Tool) -> Option<ToolPose> {
    let envelope = ctx.cleared.envelope(tool.radius);
    if envelope.is_empty() {
        return None;
    }
    walk_and_probe(ctx, tool.radius, &envelope, "ENVELOPE", boundary_probe)
}
