use prof_macros::prof;

use crate::ops::assembly::adaptive::resume::{
    probe_step, ResumeCtx, ResumeStrategy, DETAIL_NO_FRAGMENTS,
    DETAIL_NO_WALL_HUG_POINT,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ToolPose;

/// Resume from wall-hug points recorded during the current and
/// previous cut segments.
///
/// The main loop tracks each envelope visit while cutting: when the
/// tool centre comes within one radius of the pocket wall, the
/// minimum-distance pose of that visit is recorded.  Points are grouped
/// by cut segment and preserved across resumes so earlier segments can
/// serve as fallback candidates.
///
/// The points are supplied in resume order via [`ResumeCtx::wall_hug_points`]:
/// current segment points first (FIFO, most conservative), then previous
/// segments from oldest to newest.  This strategy probes at each point in
/// that order; the first that passes [`point_in_valid_area`] and
/// [`probe_step`] is returned.
///
/// Tried before [`ResumeSegment`] so that, when available, the tool
/// resumes from a wall-departure point rather than re-cutting from
/// the original envelope position.
pub struct ResumeWallHug;

impl ResumeStrategy for ResumeWallHug {
    fn label(&self) -> &'static str {
        "wall_hug"
    }

    #[prof]
    fn find_next(
        &self,
        ctx: &ResumeCtx,
        _tool: &Tool,
        detail: &mut u8,
    ) -> Option<ToolPose> {
        if ctx.cleared.fragments().is_empty() {
            *detail = DETAIL_NO_FRAGMENTS;
            return None;
        }

        for pose in ctx.wall_hug_points {
            if !point_in_valid_area(pose.pos, ctx.valid_tool_area) {
                continue;
            }
            if let Some(rp) =
                probe_step(ctx, ctx.opts.radius, pose.pos, pose.heading)
            {
                return Some(rp);
            }
        }

        *detail = DETAIL_NO_WALL_HUG_POINT;
        None
    }
}
