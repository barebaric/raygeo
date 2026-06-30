use crate::ops::assembly::adaptive::resume::{
    probe_step, ResumeCtx, ResumeStrategy,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ToolPose;

pub struct ResumeSegment;

impl ResumeStrategy for ResumeSegment {
    const NAME: &'static str = "ResumeSegment";

    fn find_next(&self, ctx: &ResumeCtx, _tool: &Tool) -> Option<ToolPose> {
        if ctx.cleared.fragments().is_empty() {
            return None;
        }

        let pos = ctx.segment_start.pos;
        if !point_in_valid_area(pos, ctx.valid_tool_area) {
            return None;
        }

        probe_step(ctx, ctx.opts.radius, pos, ctx.segment_start.heading)
    }
}
