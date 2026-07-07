use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::shape::polygon::{
    get_polygon_heading_at, offset_polygon, JoinStyle,
};
use crate::ops::assembly::profile::engine::{run_profile, ProfileCommon};
use crate::ops::assembly::profile::trace::TraceRecorder;
use crate::ops::assembly::profile::ProfileOuterOptions;
use crate::ops::assembly::result::AssemblyResult;
use crate::ops::cut::ClearedArea;
use crate::ops::state::State;
use crate::types::Point3D;

pub fn profile_outer(
    cleared: &mut ClearedArea,
    opts: &ProfileOuterOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyResult> {
    if opts.boundary.is_empty() {
        return Err(RaygeoError::DegenerateGeometry(
            "boundary polygon is empty".into(),
        ));
    }
    let offset_dist = opts.tool_radius + opts.wall_margin + opts.stock_to_leave;
    let offset_polys =
        offset_polygon(&opts.boundary, offset_dist, JoinStyle::Round);
    if offset_polys.is_empty() {
        return Err(RaygeoError::DegenerateGeometry(
            "offset produced no polygons".into(),
        ));
    }
    let common = ProfileCommon {
        step_length: opts.step_length,
        target_z: opts.target_z,
        safe_z: opts.safe_z,
        tolerance: opts.tolerance,
        tool_radius: opts.tool_radius,
        cut_direction: opts.cut_direction,
        expansion_batch_size: opts.expansion_batch_size,
        cancel_check: opts.cancel_check,
        engagement_area_threshold: opts.engagement_area_threshold,
        engagement_angle_threshold: opts.engagement_angle_threshold,
        stock_to_leave: opts.stock_to_leave,
    };
    let mut recorder = TraceRecorder::new(
        opts.trace_path.as_ref(),
        opts.tool_radius,
        &opts.boundary,
        &[],
        &offset_polys,
        &[0u32],
    );
    let heading = get_polygon_heading_at(&offset_polys[0], offset_polys[0][0])
        + std::f64::consts::FRAC_PI_2;
    let init_pos =
        Point3D::new(offset_polys[0][0].x, offset_polys[0][0].y, opts.target_z);
    recorder.record_init(init_pos, heading, 0);
    let result = run_profile(
        cleared,
        &offset_polys[0],
        &common,
        cut_state,
        0,
        &mut recorder,
    )?;
    recorder.record_exit(
        result.end.pos,
        result.end.heading,
        result.ops.len() as u32,
    );
    recorder.finish(&result.ops);
    Ok(result)
}
