//! Offset: Polygon offsetting (growing/shrinking) for geometry data.
//!
//! Provides functions for offsetting closed contours using Clipper2.
//! Handles containment hierarchies (holes within solids) correctly by
//! offsetting solids and holes independently and subtracting holes from solids.

use std::collections::HashMap;

use crate::geo::algo::intersect::check_intersection_from_array;
use crate::geo::algo::topology::{get_valid_contours_data, split_into_contours};
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::is_point_inside_polygon;
use crate::geo::shape::polygon::{get_polygons_difference, offset_polygon};
use crate::types::{Point, Polygon, Rect};

const CLIPPER_SCALE: i64 = 10_000_000;

#[derive(Clone, Debug)]
struct ContourItem {
    geo: Geometry,
    verts: Vec<Point>,
    path: Vec<(i64, i64)>,
    rect: Rect,
    area: f64,
    #[allow(dead_code)]
    id: usize,
}

fn prepare_contour_items(
    contour_data: &[(Geometry, Vec<Point>, bool)],
) -> Vec<ContourItem> {
    let mut items = Vec::new();
    for (i, (geo, vertices, _is_closed)) in contour_data.iter().enumerate() {
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
        let scaled_path: Vec<(i64, i64)> = verts
            .iter()
            .map(|v| {
                (
                    (v.0 * CLIPPER_SCALE as f64) as i64,
                    (v.1 * CLIPPER_SCALE as f64) as i64,
                )
            })
            .collect();
        let rect = geo.rect();
        items.push(ContourItem {
            geo: geo.clone(),
            verts,
            path: scaled_path,
            rect,
            area,
            id: i,
        });
    }
    items
}

fn build_containment_hierarchy(items: &[ContourItem]) -> Vec<isize> {
    let n = items.len();
    let mut parent_map = vec![-1isize; n];

    for i in 0..n {
        let mut best_parent = -1isize;
        let mut best_parent_area = f64::INFINITY;

        for j in 0..n {
            if i == j {
                continue;
            }
            let r_i = &items[i].rect;
            let r_j = &items[j].rect;
            if !(r_j.0 <= r_i.0
                && r_j.1 <= r_i.1
                && r_j.2 >= r_i.2
                && r_j.3 >= r_i.3)
            {
                continue;
            }
            if items[j].area <= items[i].area {
                continue;
            }
            if check_intersection_from_array(
                &items[i].geo.data,
                &items[j].geo.data,
                false,
            ) {
                continue;
            }
            if !items[j].verts.is_empty()
                && is_point_inside_polygon(items[i].verts[0], &items[j].verts)
                && items[j].area < best_parent_area
            {
                best_parent_area = items[j].area;
                best_parent = j as isize;
            }
        }
        parent_map[i] = best_parent;
    }
    parent_map
}

fn calculate_nesting_depths(
    parent_map: &[isize],
    num_items: usize,
) -> Vec<i32> {
    let mut depths = vec![0i32; num_items];
    for i in 0..num_items {
        let mut d = 0;
        let mut curr = parent_map[i];
        let mut iterations = 0;
        while curr != -1 && iterations <= num_items {
            d += 1;
            curr = parent_map[curr as usize];
            iterations += 1;
        }
        depths[i] = d;
    }
    depths
}

fn group_solids_and_holes(
    depths: &[i32],
    parent_map: &[isize],
) -> HashMap<usize, Vec<usize>> {
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, &d) in depths.iter().enumerate() {
        if d % 2 == 0 {
            groups.entry(i).or_default();
        } else {
            let p = parent_map[i];
            if p != -1 {
                groups.entry(p as usize).or_default().push(i);
            }
        }
    }
    groups
}

fn offset_contour_group(
    solid_path: &[(i64, i64)],
    hole_paths: &[Vec<(i64, i64)>],
    offset: f64,
) -> Vec<Polygon> {
    let scale = CLIPPER_SCALE as f64;
    let solid_poly: Polygon = solid_path
        .iter()
        .map(|(x, y)| (*x as f64 / scale, *y as f64 / scale))
        .collect();
    if solid_poly.len() < 3 {
        return vec![];
    }
    if hole_paths.is_empty() {
        return offset_polygon(&solid_poly, offset);
    }
    let hole_polys: Vec<Polygon> = hole_paths
        .iter()
        .filter_map(|path| {
            let poly: Polygon = path
                .iter()
                .map(|(x, y)| (*x as f64 / scale, *y as f64 / scale))
                .collect();
            if poly.len() >= 3 { Some(poly) } else { None }
        })
        .collect();
    if hole_polys.is_empty() {
        return offset_polygon(&solid_poly, offset);
    }
    // Offset solid and holes separately, then subtract holes from solid.
    // For positive offset (grow): solid expands outward, hole contracts (inward).
    // For negative offset (shrink): solid contracts, hole expands.
    // The hole offset direction is always opposite to the solid offset.
    let offset_solids = offset_polygon(&solid_poly, offset);
    let mut final_polys = offset_solids;
    for hole in &hole_polys {
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
    let parent_map = build_containment_hierarchy(&closed_items);
    let depths = calculate_nesting_depths(&parent_map, closed_items.len());
    let solid_groups = group_solids_and_holes(&depths, &parent_map);
    let mut new_geo = Geometry::new();
    for (solid_idx, hole_indices) in solid_groups.iter() {
        let solid_item = &closed_items[*solid_idx];
        let hole_paths: Vec<Vec<(i64, i64)>> = hole_indices
            .iter()
            .map(|&h_idx| closed_items[h_idx].path.clone())
            .collect();
        let offset_contours =
            offset_contour_group(&solid_item.path, &hole_paths, offset);
        for new_vertices in offset_contours {
            let points: Vec<(f64, f64, f64)> =
                new_vertices.iter().map(|(x, y)| (*x, *y, 0.0)).collect();
            let new_contour_geo = Geometry::from_points(&points, true);
            if !new_contour_geo.is_empty() {
                new_geo.extend(&new_contour_geo);
            }
        }
    }
    new_geo
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle_geo(x: f64, y: f64, w: f64, h: f64) -> Geometry {
        let points = [
            (x, y, 0.0),
            (x + w, y, 0.0),
            (x + w, y + h, 0.0),
            (x, y + h, 0.0),
        ];
        Geometry::from_points(&points[..], true)
    }

    #[test]
    fn test_grow_geometry_positive() {
        let rect = rectangle_geo(0.0, 0.0, 10.0, 10.0);
        let grown = grow_geometry(&rect, 1.0);
        assert!(grown.area() > rect.area());
    }

    #[test]
    fn test_grow_geometry_negative() {
        let rect = rectangle_geo(0.0, 0.0, 10.0, 10.0);
        let grown = grow_geometry(&rect, -1.0);
        assert!(grown.area() < rect.area());
    }

    #[test]
    fn test_grow_geometry_empty() {
        let geo = Geometry::new();
        let grown = grow_geometry(&geo, 1.0);
        assert!(grown.is_empty());
    }

    #[test]
    fn test_grow_geometry_zero_offset() {
        let rect = rectangle_geo(0.0, 0.0, 10.0, 10.0);
        let grown = grow_geometry(&rect, 0.0);
        let diff = (grown.area() - rect.area()).abs();
        assert!(diff < 1e-6);
    }

    #[test]
    fn test_prepare_contour_items() {
        let rect = rectangle_geo(0.0, 0.0, 10.0, 10.0);
        let contours = split_into_contours(&rect);
        let contour_data = get_valid_contours_data(&contours);
        let items = prepare_contour_items(&contour_data);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].area, 100.0);
    }

    #[test]
    fn test_calculate_nesting_depths() {
        let parent_map = vec![-1isize, 0, 1];
        let depths = calculate_nesting_depths(&parent_map, 3);
        assert_eq!(depths, vec![0, 1, 2]);
    }

    #[test]
    fn test_group_solids_and_holes() {
        let depths = vec![0, 1, 2, 1];
        let parent_map = vec![-1isize, 0, 1, 0];
        let groups = group_solids_and_holes(&depths, &parent_map);
        // depth 0 (solid) -> key 0 with holes [1, 3]
        // depth 1 (hole) -> belongs to parent 0
        // depth 2 (solid) -> key 2, no holes
        assert!(groups.contains_key(&0));
        assert!(groups.contains_key(&2));
        assert_eq!(groups[&0].len(), 2);
    }
}
