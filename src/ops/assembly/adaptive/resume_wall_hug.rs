use prof_macros::prof;

use crate::ops::assembly::adaptive::resume::{
    probe_step, ResumeCtx, ResumeStrategy,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ToolPose;

/// Resume from the point where the tool last departed the envelope
/// wall.
///
/// After a [`ResumeBoundary`] places the tool on the envelope edge, the
/// main loop tracks the tool's distance to the envelope.  As soon as a
/// step carries the tool off the envelope, the pre-step pose is recorded
/// as `last_wall_hug` in [`ResumeCtx`].
///
/// This strategy probes at `last_wall_hug` — the last pose where the tool
/// was still on the wall — using the heading the tool had at that moment.
/// The heading has already been adjusted by the stepper for the local
/// material engagement, so the probe is more likely to find productive
/// cutting than re-probing from the original boundary resume point with
/// the raw envelope tangent.
///
/// Tried before [`ResumeSegment`] so that, when available, the tool
/// resumes from the departure point rather than re-cutting from the
/// original envelope position.
pub struct ResumeWallHug;

impl ResumeStrategy for ResumeWallHug {
    const NAME: &'static str = "ResumeWallHug";

    #[prof]
    fn find_next(&self, ctx: &ResumeCtx, _tool: &Tool) -> Option<ToolPose> {
        if ctx.cleared.fragments().is_empty() {
            return None;
        }

        let pose = ctx.last_wall_hug?;
        if !point_in_valid_area(pose.pos, ctx.valid_tool_area) {
            return None;
        }

        probe_step(ctx, ctx.opts.radius, pose.pos, pose.heading)
    }
}
