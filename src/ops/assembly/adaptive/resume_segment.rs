use crate::dbg_log;
use crate::ops::assembly::adaptive::resume::{
    probe_step, ResumeCtx, ResumeStrategy,
};
use crate::ops::assembly::adaptive::tool::Tool;
use crate::ops::cut::interp::point_in_valid_area;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::ToolPose;
use crate::types::{Point, Polygon};

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

/// SegmentResume: walk forward from `segment_start` along
/// `cut_direction` until probing finds a position where the tool can
/// re-engage uncut material.
///
/// The tool was cutting a segment that started at `segment_start`.
/// When it stalls, we jump back to the segment start and walk
/// forward.  At each position, probe straight ahead and at several
/// outward angles.  The first position where any probe finds
/// engagement above `min_cut_area` becomes the resume target.
///
/// Positions outside `valid_tool_area` are skipped — the tool can
/// never operate there.
#[allow(clippy::too_many_arguments)]
pub fn search_reengagement(
    cleared: &ClearedArea,
    segment_start: Point,
    cut_direction: Point,
    radius: f64,
    step_length: f64,
    _advance: f64,
    min_cut_area: f64,
    valid_tool_area: &[Polygon],
) -> Option<ToolPose> {
    if cleared.fragments().is_empty() {
        return None;
    }

    let tangent = cut_direction;

    // Probe directions relative to the cutting tangent.
    const PROBE_ANGLES: [f64; 7] = [
        0.0,
        std::f64::consts::FRAC_PI_6,
        -std::f64::consts::FRAC_PI_6,
        std::f64::consts::FRAC_PI_3,
        -std::f64::consts::FRAC_PI_3,
        std::f64::consts::FRAC_PI_2,
        -std::f64::consts::FRAC_PI_2,
    ];

    let max_steps = 2000;
    let mut pos = segment_start;

    for step in 0..max_steps {
        if !point_in_valid_area(pos, valid_tool_area) {
            pos += tangent * step_length;
            continue;
        }

        for &angle in &PROBE_ANGLES {
            let dir = Point::new(
                tangent.x * angle.cos() - tangent.y * angle.sin(),
                tangent.x * angle.sin() + tangent.y * angle.cos(),
            );
            let probe = pos + dir * step_length;
            let area = cleared.cut_area(pos, probe, radius);
            if area >= min_cut_area {
                let heading = dir.y.atan2(dir.x);
                dbg_log!(
                    "  REENGAGE  step={}  pos=({:.3},{:.3})  \
                     area={:.4}  angle={:.1}°  heading={:.4}",
                    step,
                    pos.x,
                    pos.y,
                    area,
                    angle.to_degrees(),
                    heading,
                );
                return Some(ToolPose { pos, heading });
            }
        }
        pos += tangent * step_length;
    }

    dbg_log!("  REENGAGE  no engagement within {} steps", max_steps);
    None
}
