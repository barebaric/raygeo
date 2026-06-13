//! Offset: Polygon offsetting (growing/shrinking) for geometry data.
//!
//! Provides functions for offsetting closed contours using Clipper2.
//! Handles containment hierarchies (holes within solids) correctly by
//! offsetting solids and holes independently and subtracting holes from solids.

use crate::geo::algo::intersect::check_intersection_from_array;
use crate::geo::algo::topology::{
    build_hierarchy, get_valid_contours_data, group_solids_and_holes,
    split_into_contours,
};
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::{get_polygons_difference, offset_polygon};
use crate::types::{Point, Point3D, Polygon};

#[derive(Clone, Debug)]
struct ContourItem {
    path: Polygon,
    #[allow(dead_code)]
    area: f64,
    id: usize,
}

fn prepare_contour_items(
    contour_data: &[(&Geometry, Vec<Point>, bool)],
) -> Vec<ContourItem> {
    let mut items = Vec::new();
    for (i, (_geo, vertices, _is_closed)) in contour_data.iter().enumerate() {
        if vertices.len() < 2 {
            continue;
        }
        let mut verts = vertices.clone();
        let first = verts[0];
        let last = verts[verts.len() - 1];
        if (first.0 - last.0).abs() < 1e-9 && (first.1 - last.1).abs() < 1e-9 {
            verts.pop();
        }
        if verts.len() < 3 {
            continue;
        }
        let mut area = 0.0;
        let n = verts.len();
        for j in 0..n {
            let k = (j + 1) % n;
            area += verts[j].0 * verts[k].1;
            area -= verts[k].0 * verts[j].1;
        }
        area = area.abs() / 2.0;
        items.push(ContourItem {
            path: verts,
            area,
            id: i,
        });
    }
    items
}

fn offset_contour_group(
    solid_path: &Polygon,
    hole_paths: &[Polygon],
    offset: f64,
) -> Vec<Polygon> {
    if solid_path.len() < 3 {
        return vec![];
    }
    if hole_paths.is_empty() {
        return offset_polygon(solid_path, offset);
    }
    // Offset solid and holes separately, then subtract holes from solid.
    // For positive offset (grow): solid expands outward, hole contracts (inward).
    // For negative offset (shrink): solid contracts, hole expands.
    // The hole offset direction is always opposite to the solid offset.
    let offset_solids = offset_polygon(solid_path, offset);
    let mut final_polys = offset_solids;
    for hole in hole_paths {
        let offset_holes = offset_polygon(hole, -offset);
        for offset_hole in &offset_holes {
            let mut new_result = Vec::new();
            for poly in final_polys.drain(..) {
                new_result.extend(get_polygons_difference(&poly, offset_hole));
            }
            final_polys = new_result;
        }
    }
    final_polys
}

/// Offsets the closed contours of a Geometry object by a given amount.
///
/// This function grows (positive offset) or shrinks (negative offset) the
/// area enclosed by closed paths.
///
/// This implementation processes logically distinct shapes (islands)
/// independently. Holes are associated with their enclosing solids and
/// offset together. Adjacent or overlapping solids remain separate, preserving
/// distinct toolpaths.
pub fn grow_geometry(geometry: &Geometry, offset: f64) -> Geometry {
    let raw_contours = split_into_contours(geometry);
    if raw_contours.is_empty() {
        return Geometry::new();
    }
    let contour_data = get_valid_contours_data(&raw_contours);
    if contour_data.is_empty() {
        return Geometry::new();
    }
    let closed_items = prepare_contour_items(&contour_data);
    if closed_items.is_empty() {
        return Geometry::new();
    }

    let hierarchy_geoms: Vec<&Geometry> =
        contour_data.iter().map(|(g, _, _)| *g).collect();
    let mut hierarchy = build_hierarchy(&hierarchy_geoms);
    let hierarchy_info = hierarchy.info.clone();
    hierarchy.filter_parents(&hierarchy_info, |child, parent| {
        !check_intersection_from_array(
            &hierarchy_geoms[child].data,
            &hierarchy_geoms[parent].data,
            false,
        )
    });
    let solid_groups = group_solids_and_holes(&hierarchy);

    let contour_to_item: std::collections::HashMap<usize, usize> = closed_items
        .iter()
        .enumerate()
        .map(|(item_idx, item)| (item.id, item_idx))
        .collect();

    let mut new_geo = Geometry::new();
    for (solid_contour_idx, hole_contour_indices) in solid_groups.iter() {
        let solid_item_idx = match contour_to_item.get(solid_contour_idx) {
            Some(&idx) => idx,
            None => continue,
        };
        let solid_item = &closed_items[solid_item_idx];
        let hole_paths: Vec<Polygon> = hole_contour_indices
            .iter()
            .filter_map(|&h_idx| {
                contour_to_item
                    .get(&h_idx)
                    .map(|&idx| closed_items[idx].path.clone())
            })
            .collect();
        let offset_contours =
            offset_contour_group(&solid_item.path, &hole_paths, offset);
        for new_vertices in offset_contours {
            let points: Vec<Point3D> = new_vertices
                .iter()
                .map(|p| Point3D(p.0, p.1, 0.0))
                .collect();
            let new_contour_geo = Geometry::from_points(&points, true);
            if !new_contour_geo.is_empty() {
                new_geo.extend(&new_contour_geo);
            }
        }
    }
    new_geo
}
