use prof_macros::prof;

use crate::ops::assembly::adaptive::resume::{
    probe, walk_and_probe, ResumeCtx, ResumeStrategy, WalkProbeOptions,
    DETAIL_NO_ENGAGEMENT, DETAIL_NO_ENVELOPE,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::ToolPose;

pub struct ResumeEnvelope;

impl ResumeStrategy for ResumeEnvelope {
    fn label(&self) -> &'static str {
        "envelope"
    }

    fn find_next(
        &self,
        ctx: &ResumeCtx,
        tool: &Tool,
        detail: &mut u8,
    ) -> Option<ToolPose> {
        envelope_resume(ctx, tool, detail)
    }
}

/// Walk the tool-centre **envelope** (the pocket inset by the tool
/// radius) and find a re-engagement point.
///
/// The envelope is a *tool-centre* boundary: it is the locus of legal
/// tool-centre positions, so the tool centre sits directly on the
/// envelope edge — no inward offset is applied.  Each sample is checked
/// with [`probe`].
#[prof]
fn envelope_resume(
    ctx: &ResumeCtx,
    tool: &Tool,
    detail: &mut u8,
) -> Option<ToolPose> {
    let envelope = ctx.cleared.envelope(tool.radius);
    if envelope.is_empty() {
        *detail = DETAIL_NO_ENVELOPE;
        return None;
    }
    let result = walk_and_probe(
        ctx,
        tool.radius,
        &envelope,
        "ENVELOPE",
        WalkProbeOptions::default(),
        probe,
    );
    if result.is_none() {
        *detail = DETAIL_NO_ENGAGEMENT;
    }
    result
}
