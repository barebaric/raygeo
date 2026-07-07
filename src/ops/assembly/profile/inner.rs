use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_centroid, get_polygon_heading_at,
    get_polygons_group_difference, is_point_in_polygon, offset_polygon,
    JoinStyle,
};
use crate::ops::assembly::profile::engine::{run_profile, ProfileCommon};
use crate::ops::assembly::profile::trace::TraceRecorder;
use crate::ops::assembly::profile::ProfileInnerOptions;
use crate::ops::assembly::result::{self, AssemblyResult};
use crate::ops::container::Ops;
use crate::ops::cut::ClearedArea;
use crate::ops::cut::ToolPose;
use crate::ops::state::State;
use crate::types::{Point3D, Polygon};
use glam::Vec3Swizzles;

#[allow(unused_variables, dead_code)]
pub fn profile_inner(
    cleared: &mut ClearedArea,
    opts: &ProfileInnerOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyResult> {
    if opts.boundary.is_empty() {
        return Err(RaygeoError::DegenerateGeometry(
            "boundary polygon is empty".into(),
        ));
    }

    let offset_dist = opts.radius + opts.wall_margin + opts.stock_to_leave;

    // Round joins at convex corners of the inset boundary produce arcs that
    // the tool tip follows; concave corners stay sharp (clipper2 only adds
    // vertices where a convex offset would need to bridge a gap).
    let grown_islands: Vec<Polygon> = opts
        .islands
        .iter()
        .flat_map(|isl| offset_polygon(isl, offset_dist, JoinStyle::Round))
        .collect();
    let inset_polys =
        offset_polygon(&opts.boundary, -offset_dist, JoinStyle::Round);
    let valid_region: Vec<Polygon> = if grown_islands.is_empty() {
        inset_polys
    } else {
        get_polygons_group_difference(&inset_polys, &grown_islands)
    };

    let outer_poly = valid_region.iter().max_by(|a, b| {
        get_polygon_area(a)
            .partial_cmp(&get_polygon_area(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let outer_poly = match outer_poly {
        Some(p) if p.len() >= 3 => p,
        _ => {
            let zero = Point3D::ZERO;
            return Ok(AssemblyResult {
                ops: Ops::new(),
                cleared_polygons: vec![],
                start: ToolPose {
                    pos: zero,
                    heading: 0.0,
                },
                end: ToolPose {
                    pos: zero,
                    heading: 0.0,
                },
            });
        }
    };

    let accessible_indices: Vec<usize> = grown_islands
        .iter()
        .enumerate()
        .filter(|(_, grown)| {
            if grown.len() < 3 {
                return false;
            }
            let centroid = get_polygon_centroid(grown);
            valid_region
                .iter()
                .any(|vp| is_point_in_polygon(centroid, vp))
        })
        .map(|(i, _)| i)
        .collect();

    let walk_order: Vec<u32> = (0..=accessible_indices.len() as u32).collect();
    let all_offset_polys: Vec<Polygon> = {
        let mut polys = vec![outer_poly.clone()];
        for &idx in &accessible_indices {
            polys.push(grown_islands[idx].clone());
        }
        polys
    };

    let mut recorder = TraceRecorder::new(
        opts.trace_path.as_ref(),
        opts.radius,
        &opts.boundary,
        &opts.islands,
        &all_offset_polys,
        &walk_order,
    );

    let heading = get_polygon_heading_at(outer_poly, outer_poly[0])
        + std::f64::consts::FRAC_PI_2;
    let init_pos = Point3D::new(outer_poly[0].x, outer_poly[0].y, opts.cut_z);
    recorder.record_init(init_pos, heading, 0);

    let common = ProfileCommon {
        step_length: opts.step_length,
        cut_z: opts.cut_z,
        safe_z: opts.safe_z,
        tolerance: opts.tolerance,
        radius: opts.radius,
        cut_direction: opts.cut_direction,
        expansion_batch_size: opts.expansion_batch_size,
        cancel_check: opts.cancel_check,
        engagement_area_threshold: opts.engagement_area_threshold,
        engagement_angle_threshold: opts.engagement_angle_threshold,
        stock_to_leave: opts.stock_to_leave,
    };

    let mut result =
        run_profile(cleared, outer_poly, &common, cut_state, 0, &mut recorder)?;

    if !accessible_indices.is_empty() {
        let mut remaining: Vec<usize> = accessible_indices;
        let mut last_end = result.end.pos;

        while !remaining.is_empty() {
            let nearest_idx = remaining
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let ca = get_polygon_centroid(&grown_islands[**a]);
                    let cb = get_polygon_centroid(&grown_islands[**b]);
                    let da = (ca - last_end.xy()).length_squared();
                    let db = (cb - last_end.xy()).length_squared();
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap();

            let idx = remaining.remove(nearest_idx);
            let island_idx = 1 + idx as u32;
            let island_result = run_profile(
                cleared,
                &grown_islands[idx],
                &common,
                cut_state,
                island_idx,
                &mut recorder,
            )?;
            last_end = island_result.end.pos;
            result = result::chain(result, island_result);
        }
    }

    recorder.record_exit(
        result.end.pos,
        result.end.heading,
        result.ops.len() as u32,
    );
    recorder.finish(&result.ops);
    Ok(result)
}
