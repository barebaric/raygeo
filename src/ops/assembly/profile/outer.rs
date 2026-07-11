use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::shape::polygon::{
    get_polygon_heading_at, offset_polygon, JoinStyle,
};
use crate::ops::assembly::profile::engine::{run_profile, ProfileCommon};
use crate::ops::assembly::profile::ProfileOuterOptions;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::Tracelet;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::Part;
use crate::ops::state::State;

use super::trace_helpers as th;

/// Profile the outer boundary of a pocket, extracting geometry from
/// `part`.
pub fn profile_outer(
    part: &Part,
    trace: &mut Tracelet,
    cleared: &mut ClearedArea,
    opts: &ProfileOuterOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let (boundary_opt, _islands) = part.extract_boundary();
    let boundary = boundary_opt.ok_or_else(|| {
        RaygeoError::DegenerateGeometry(
            "Part has no extractable boundary geometry".into(),
        )
    })?;

    let offset_dist = opts.tool_radius + opts.wall_margin + opts.stock_to_leave;
    let offset_polys = offset_polygon(&boundary, offset_dist, JoinStyle::Round);
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
    let _heading = get_polygon_heading_at(&offset_polys[0], offset_polys[0][0])
        + std::f64::consts::FRAC_PI_2;

    let walk_order: Vec<u32> = (0..offset_polys.len() as u32).collect();
    trace.set_attrs(th::build_attrs(&offset_polys, &walk_order));

    run_profile(trace, cleared, &offset_polys[0], &common, cut_state, 0)
}
