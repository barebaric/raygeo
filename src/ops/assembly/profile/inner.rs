use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_centroid, get_polygon_heading_at,
    get_polygons_group_difference, is_point_in_polygon, offset_polygon,
    JoinStyle,
};
use crate::ops::assembly::profile::engine::{run_profile, ProfileCommon};
use crate::ops::assembly::profile::ProfileInnerOptions;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::Tracelet;

use super::trace_helpers as th;
use crate::ops::part::Part;
use crate::ops::state::State;
use crate::ops::types::ToolPose;
use crate::types::{Point3D, Polygon};
use glam::Vec3Swizzles;

/// Profile the inner boundary of a pocket, extracting geometry from
/// `part`.
pub fn profile_inner(
    part: &mut Part,
    trace: &mut Tracelet,
    opts: &ProfileInnerOptions,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let (boundary_opt, islands) = part.extract_boundary();
    let boundary = boundary_opt.ok_or_else(|| {
        RaygeoError::DegenerateGeometry(
            "Part has no extractable boundary geometry".into(),
        )
    })?;

    let offset_dist = opts.tool_radius + opts.wall_margin + opts.stock_to_leave;

    let grown_islands: Vec<Polygon> = islands
        .iter()
        .flat_map(|isl| offset_polygon(isl, offset_dist, JoinStyle::Round))
        .collect();
    let inset_polys = offset_polygon(&boundary, -offset_dist, JoinStyle::Round);
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
            return Ok(AssemblyMeta {
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

    let _heading = get_polygon_heading_at(outer_poly, outer_poly[0])
        + std::f64::consts::FRAC_PI_2;

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

    trace.set_attrs(th::build_attrs(&all_offset_polys, &walk_order));

    let outer_meta =
        run_profile(part, trace, outer_poly, &common, cut_state, 0)?;
    let start_pose = outer_meta.start;
    let mut end_pose = outer_meta.end;

    if !accessible_indices.is_empty() {
        let mut remaining: Vec<usize> = accessible_indices;
        let mut last_end = end_pose.pos;

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
            let island_meta = run_profile(
                part,
                trace,
                &grown_islands[idx],
                &common,
                cut_state,
                island_idx,
            )?;
            last_end = island_meta.end.pos;
            end_pose = island_meta.end;
        }
    }

    Ok(AssemblyMeta {
        start: start_pose,
        end: end_pose,
    })
}
