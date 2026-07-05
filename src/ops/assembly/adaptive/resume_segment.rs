use prof_macros::prof;

use crate::ops::assembly::adaptive::resume::{
    probe, require_fragments, ResumeCtx, ResumeStrategy, DETAIL_NO_ENGAGEMENT,
    DETAIL_NO_GROWTH, DETAIL_OUTSIDE_VALID,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Point3D};

pub struct ResumeSegment;

impl ResumeStrategy for ResumeSegment {
    fn label(&self) -> &'static str {
        "segment"
    }

    fn find_next(
        &self,
        ctx: &ResumeCtx,
        _tool: &Tool,
        detail: &mut u8,
    ) -> Option<ToolPose> {
        require_fragments(ctx, detail)?;

        let area_grew = ctx.cleared.total_area() > ctx.last_resume_area + 1e-9;
        if !area_grew {
            *detail = DETAIL_NO_GROWTH;
            return None;
        }

        let pos_2d = crate::types::Point::new(
            ctx.segment_start.pos.x,
            ctx.segment_start.pos.y,
        );
        if !point_in_valid_area(pos_2d, ctx.step_opts.valid_area) {
            *detail = DETAIL_OUTSIDE_VALID;
            return None;
        }

        if let Some(tp) = probe(
            ctx,
            ctx.opts.radius,
            ctx.segment_start.pos,
            ctx.segment_start.heading,
        ) {
            return Some(tp);
        }

        nudge_to_frontier(ctx, detail)
    }
}

/// Engagement angle above which the disc is considered tangent to the
/// frontier — the disc edge is touching uncleared material.
const TANGENCY_ANGLE: f64 = 0.1;

/// March from `segment_start` along its heading in small steps until
/// the tool disc becomes tangent to the frontier (engagement angle
/// exceeds [`TANGENCY_ANGLE`]), then probe from that position.
///
/// This handles the case where the tool stalled well inside the
/// cleared area and `probe` at `segment_start` finds no
/// engagement — nudging forward brings the disc edge back into
/// contact with the material boundary.
#[prof]
fn nudge_to_frontier(ctx: &ResumeCtx, detail: &mut u8) -> Option<ToolPose> {
    let radius = ctx.opts.radius;
    let step = ctx.opts.step_length * 0.25;
    let max_steps = (radius * 3.0 / step).ceil() as usize;
    let dir = Point::new(
        ctx.segment_start.heading.cos(),
        ctx.segment_start.heading.sin(),
    );
    let start_2d = crate::types::Point::new(
        ctx.segment_start.pos.x,
        ctx.segment_start.pos.y,
    );
    let cut_z = ctx.opts.cut_z;

    let mut tangent_pos: Option<Point> = None;
    for s in 1..=max_steps {
        let pos = start_2d + dir * (s as f64 * step);
        if !point_in_valid_area(pos, ctx.step_opts.valid_area) {
            break;
        }
        let eng = ctx.cleared.get_point_engagement(pos, radius);
        if eng.angle >= TANGENCY_ANGLE {
            tangent_pos = Some(pos);
            break;
        }
    }

    let pos = tangent_pos?;
    if let Some(tp) = probe(
        ctx,
        radius,
        Point3D::new(pos.x, pos.y, cut_z),
        ctx.segment_start.heading,
    ) {
        return Some(tp);
    }

    *detail = DETAIL_NO_ENGAGEMENT;
    None
}
