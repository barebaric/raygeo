use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::shape::polygon::{offset_polygon, JoinStyle};
use crate::ops::assembly::profile::engine::{
    loop_from_polygon, run_profile, ProfileCommon,
};
use crate::ops::assembly::profile::ProfileSpec;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::Tracelet;
use crate::ops::part::FaceState;
use crate::ops::state::State;

use super::trace_helpers as th;

/// Profile the outer boundary of a pocket, extracting geometry from
/// `face`.
///
/// With `prefer_exact` and curve-true geometry on the face, the boundary
/// comes from the exact offsetter and arcs are emitted as arcs. The exact
/// attempt falls back to the polygon path when it yields nothing
/// machinable.
pub fn profile_outer(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &ProfileSpec,
    cut_state: &State,
    prefer_exact: bool,
) -> RaygeoResult<AssemblyMeta> {
    let offset_dist = opts.tool_radius + opts.wall_margin + opts.stock_to_leave;
    if prefer_exact {
        if let Some(geometry) = face.geometry.clone() {
            // Grow the region by the offset distance; islands are ignored,
            // like the polygon path below. Nothing machinable falls back.
            if let Some(meta) = super::exact::run_exact_loops(
                face,
                trace,
                &geometry,
                offset_dist,
                opts,
                cut_state,
                true,
            )? {
                return Ok(meta);
            }
        }
    }
    let (boundary_opt, _islands) = face.extract_boundary();
    let boundary = boundary_opt.ok_or_else(|| {
        RaygeoError::DegenerateGeometry(
            "Part has no extractable boundary geometry".into(),
        )
    })?;

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
        feed_reduction_factor: opts.feed_reduction_factor,
        stock_to_leave: opts.stock_to_leave,
    };

    let walk_order: Vec<u32> = (0..offset_polys.len() as u32).collect();
    trace.set_attrs(th::build_attrs(&offset_polys, &walk_order));

    let outer_loop = loop_from_polygon(&offset_polys[0], opts.target_z)
        .ok_or_else(|| {
            RaygeoError::DegenerateGeometry("offset polygon too small".into())
        })?;
    run_profile(face, trace, &outer_loop, &common, cut_state, 0)
}
