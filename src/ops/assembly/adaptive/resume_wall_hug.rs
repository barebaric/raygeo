use prof_macros::prof;

use crate::geo::shape::polygon::get_polygons_closest_point;
use crate::ops::assembly::adaptive::resume::{
    probe, require_fragments, ResumeCtx, ResumeStrategy,
    DETAIL_NO_WALL_HUG_POINT,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Point3D};

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
/// [`probe`] is returned.
///
/// For each wall-hug pose, an offset candidate one [`step_length`] toward
/// the nearest pocket boundary point is tried first.  Using the boundary
/// normal (via [`get_polygons_closest_point`]) rather than a heading-
/// perpendicular ensures the offset correctly points toward the wall
/// surface on both concave and convex wall segments.  If the offset probe
/// fails or the offset point leaves the valid area, the original wall-hug
/// point is used as fallback.
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
        require_fragments(ctx, detail)?;

        for pose in ctx.wall_hug_points {
            let pt = Point::new(pose.pos.x, pose.pos.y);

            // Try offset toward the nearest pocket boundary first.
            if let Some((_, _, closest, d2)) =
                get_polygons_closest_point(ctx.step_opts.valid_area, pt)
            {
                let d = d2.sqrt();
                if d > 1e-9 {
                    let offset_mag = ((d * 0.75) - ctx.step_length * 0.05)
                        .max(0.0)
                        .min(ctx.step_length * 1.5);
                    let toward_x = (closest.x - pt.x) / d;
                    let toward_y = (closest.y - pt.y) / d;
                    let offset_x = pose.pos.x + offset_mag * toward_x;
                    let offset_y = pose.pos.y + offset_mag * toward_y;
                    let offset_pos =
                        Point3D::new(offset_x, offset_y, pose.pos.z);

                    if point_in_valid_area(
                        Point::new(offset_x, offset_y),
                        ctx.step_opts.valid_area,
                    ) {
                        if let Some(rp) = probe(
                            ctx,
                            ctx.opts.tool_radius,
                            offset_pos,
                            pose.heading,
                        ) {
                            return Some(rp);
                        }
                    }
                }
            }

            // Fall back to original wall-hug point.
            if !point_in_valid_area(pt, ctx.step_opts.valid_area) {
                continue;
            }
            if let Some(rp) =
                probe(ctx, ctx.opts.tool_radius, pose.pos, pose.heading)
            {
                return Some(rp);
            }
        }

        *detail = DETAIL_NO_WALL_HUG_POINT;
        None
    }
}
