//! Topology: Contour and component splitting for geometry data.
//!
//! Provides functions for splitting a Geometry into individual contours
//! (subpaths delimited by MOVE commands) and for separating logically
//! connected components using point-in-polygon containment and BFS.
//! Also provides contour analysis functions like reverse_contour,
//! normalize_winding_orders, filter_to_external_contours, etc.

use rstar::{RTree, RTreeObject, AABB};

use crate::geo::algo::analysis::{get_subpath_area_from_array, is_closed};
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::is_point_inside_polygon;
use crate::geo::types::{Command, Point, Point3D, Rect};

/// Preprocessed contour data for hierarchy analysis.
#[derive(Clone, Debug)]
pub struct ContourInfo {
    pub vertices: Vec<Point>,
    pub rect: Rect,
    pub test_point: Point,
}

/// Result of building a containment hierarchy over contours.
#[derive(Clone, Debug)]
pub struct ContourHierarchy {
    /// Preprocessed contour info (None for contours that are open/empty/degenerate).
    pub info: Vec<Option<ContourInfo>>,
    /// Nesting depth for each contour (0 = outermost, -1 = not applicable).
    pub nesting_depths: Vec<i32>,
    /// Direct parent index for each contour (-1 = no parent / root).
    pub parent_map: Vec<isize>,
}

impl ContourHierarchy {
    pub fn filter_parents<F>(
        &mut self,
        info: &[Option<ContourInfo>],
        should_keep: F,
    ) where
        F: Fn(usize, usize) -> bool,
    {
        let n = self.parent_map.len();
        for i in 0..n {
            if self.parent_map[i] != -1 {
                let parent = self.parent_map[i] as usize;
                if !should_keep(i, parent) {
                    self.parent_map[i] = -1;
                    self.nesting_depths[i] -= 1;

                    let current = match &info[i] {
                        Some(ci) => ci,
                        None => continue,
                    };
                    let tx = current.test_point.x;
                    let ty = current.test_point.y;
                    let mut best_parent: isize = -1;
                    let mut best_parent_area = f64::INFINITY;

                    for (j, info_j) in info.iter().enumerate() {
                        if i == j {
                            continue;
                        }
                        let other = match info_j {
                            Some(ci) => ci,
                            None => continue,
                        };
                        if self.nesting_depths[j] < 0 {
                            continue;
                        }
                        if tx < other.rect.min.x
                            || tx > other.rect.max.x
                            || ty < other.rect.min.y
                            || ty > other.rect.max.y
                        {
                            continue;
                        }
                        if !is_point_inside_polygon(
                            current.test_point,
                            &other.vertices,
                        ) {
                            continue;
                        }
                        if !should_keep(i, j) {
                            continue;
                        }
                        let other_bbox_area = (other.rect.max.x
                            - other.rect.min.x)
                            * (other.rect.max.y - other.rect.min.y);
                        if other_bbox_area < best_parent_area {
                            best_parent_area = other_bbox_area;
                            best_parent = j as isize;
                        }
                    }
                    self.parent_map[i] = best_parent;
                }
            }
        }

        for i in 0..n {
            if self.nesting_depths[i] < 0 {
                continue;
            }
            let mut d = 0i32;
            let mut curr = self.parent_map[i];
            let mut iterations = 0;
            while curr != -1 && iterations <= n {
                d += 1;
                curr = self.parent_map[curr as usize];
                iterations += 1;
            }
            self.nesting_depths[i] = d;
        }
    }
}

/// Wrapper for contour bounding boxes, used to build an R-tree spatial index.
struct ContourEnvelope {
    index: usize,
    rect: Rect,
}

impl RTreeObject for ContourEnvelope {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.rect.min.x, self.rect.min.y],
            [self.rect.max.x, self.rect.max.y],
        )
    }
}

/// Build a containment hierarchy from a list of geometries.
pub fn build_hierarchy(contours: &[&Geometry]) -> ContourHierarchy {
    let count = contours.len();
    let mut info: Vec<Option<ContourInfo>> = Vec::with_capacity(count);

    for c in contours {
        if c.is_empty() || c.data.is_empty() {
            info.push(None);
            continue;
        }

        let c_is_closed = is_closed(&c.data, 1e-6);
        if !c_is_closed {
            info.push(None);
            continue;
        }

        let segments = c.segments();
        if segments.is_empty() {
            info.push(None);
            continue;
        }

        let verts_3d = &segments[0];
        let verts_2d: Vec<Point> =
            verts_3d.iter().map(|p| Point::new(p.x, p.y)).collect();
        let rect = c.rect();
        let test_point = if verts_2d.is_empty() {
            Point::new(0.0, 0.0)
        } else {
            verts_2d[0]
        };

        let area = get_subpath_area_from_array(&c.data, 0);
        if area.abs() < 1e-9 {
            info.push(None);
            continue;
        }

        info.push(Some(ContourInfo {
            vertices: verts_2d,
            rect,
            test_point,
        }));
    }

    let mut nesting_depths = vec![-1i32; count];
    let mut parent_map = vec![-1isize; count];

    // Build R-tree over contour bounding boxes for efficient containment queries.
    let envelopes: Vec<ContourEnvelope> = info
        .iter()
        .enumerate()
        .filter_map(|(i, ci)| {
            ci.as_ref().map(|c| ContourEnvelope {
                index: i,
                rect: c.rect,
            })
        })
        .collect();
    let rtree = RTree::bulk_load(envelopes);

    for (i, info_i) in info.iter().enumerate() {
        let current = match info_i {
            Some(ci) => ci,
            None => continue,
        };

        let mut depth = 0i32;
        let mut best_parent: isize = -1;
        let mut best_parent_area = f64::INFINITY;
        let tp = [current.test_point.x, current.test_point.y];

        // O(log n + k) query: only check contours whose bounding boxes contain the test point.
        let point_envelope = AABB::from_point(tp);
        for candidate in rtree.locate_in_envelope_intersecting(&point_envelope)
        {
            let j = candidate.index;
            if i == j {
                continue;
            }
            let other = match &info[j] {
                Some(ci) => ci,
                None => continue,
            };

            if is_point_inside_polygon(current.test_point, &other.vertices) {
                depth += 1;
                let other_bbox_area = (other.rect.max.x - other.rect.min.x)
                    * (other.rect.max.y - other.rect.min.y);
                if other_bbox_area < best_parent_area {
                    best_parent_area = other_bbox_area;
                    best_parent = j as isize;
                }
            }
        }

        nesting_depths[i] = depth;
        parent_map[i] = best_parent;
    }

    ContourHierarchy {
        info,
        nesting_depths,
        parent_map,
    }
}

/// Group contours into solids (even nesting depth) and their associated holes
/// (odd nesting depth). Returns a map from solid index to its hole indices.
pub fn group_solids_and_holes(
    hierarchy: &ContourHierarchy,
) -> std::collections::HashMap<usize, Vec<usize>> {
    let mut groups: std::collections::HashMap<usize, Vec<usize>> =
        std::collections::HashMap::new();
    for (i, &depth) in hierarchy.nesting_depths.iter().enumerate() {
        if depth < 0 {
            continue;
        }
        if depth % 2 == 0 {
            groups.entry(i).or_default();
        } else {
            let p = hierarchy.parent_map[i];
            if p != -1 {
                groups.entry(p as usize).or_default().push(i);
            }
        }
    }
    groups
}

/// Split a Geometry into individual contour geometries.
pub fn split_into_contours(geometry: &Geometry) -> Vec<Geometry> {
    if geometry.data.is_empty() {
        return vec![];
    }
    let data = &geometry.data;

    let move_indices: Vec<usize> = data
        .iter()
        .enumerate()
        .filter(|(_, cmd)| matches!(cmd, Command::Move { .. }))
        .map(|(i, _)| i)
        .collect();

    if move_indices.is_empty() {
        let mut new_geo = Geometry::new();
        new_geo.data = data.to_vec();
        new_geo.last_move_to = data[0].end_point();
        return vec![new_geo];
    }

    let mut contours: Vec<Geometry> = Vec::new();

    if move_indices[0] != 0 {
        let mut new_geo = Geometry::new();
        new_geo.data = data[..move_indices[0]].to_vec();
        if !new_geo.data.is_empty() {
            new_geo.last_move_to = new_geo.data[0].end_point();
        }
        contours.push(new_geo);
    }

    for i in 0..move_indices.len() {
        let start = move_indices[i];
        let end = if i + 1 < move_indices.len() {
            move_indices[i + 1]
        } else {
            data.len()
        };
        let slice = &data[start..end];
        if !slice.is_empty() {
            let mut new_geo = Geometry::new();
            new_geo.data = slice.to_vec();
            new_geo.last_move_to = slice[0].end_point();
            contours.push(new_geo);
        }
    }

    contours.retain(|g| !g.is_empty());
    contours
}

/// Find connected components in an adjacency graph using BFS.
fn find_connected_components_bfs(
    num_contours: usize,
    adj: &[Vec<usize>],
) -> Vec<Vec<usize>> {
    let mut visited = vec![false; num_contours];
    let mut components: Vec<Vec<usize>> = Vec::new();

    for i in 0..num_contours {
        if visited[i] {
            continue;
        }
        let mut component = Vec::new();
        let mut queue = vec![i];
        visited[i] = true;
        while let Some(u) = queue.pop() {
            component.push(u);
            for &v in &adj[u] {
                if !visited[v] {
                    visited[v] = true;
                    queue.push(v);
                }
            }
        }
        components.push(component);
    }
    components
}

/// Extract valid contour data from a list of contour geometries.
pub fn get_valid_contours_data(
    contour_geometries: &[Geometry],
) -> Vec<(&Geometry, Vec<Point>, bool)> {
    let mut result = Vec::new();
    for geo in contour_geometries {
        if geo.is_empty() {
            continue;
        }
        if geo.data.len() < 2 {
            continue;
        }
        if !matches!(&geo.data[0], Command::Move { .. }) {
            continue;
        }

        let closed = geo.is_closed(1e-6);
        let bbox = geo.rect();
        let bbox_area = (bbox.max.x - bbox.min.x) * (bbox.max.y - bbox.min.y);
        let is_closed_flag = closed && bbox_area > 1e-9;

        if !is_closed_flag {
            continue;
        }

        let vertices =
            crate::geo::algo::analysis::get_subpath_vertices_from_array(
                &geo.data, 0,
            );

        result.push((geo, vertices, is_closed_flag));
    }
    result
}

/// Split a Geometry into logically connected components.
pub fn split_into_components(geometry: &Geometry) -> Vec<Geometry> {
    if geometry.is_empty() {
        return vec![];
    }

    let contour_geometries = split_into_contours(geometry);
    if contour_geometries.len() <= 1 {
        return vec![geometry.copy()];
    }

    let all_contour_data = get_valid_contours_data(&contour_geometries);
    if all_contour_data.is_empty() {
        return vec![];
    }

    let any_closed = all_contour_data.iter().any(|(_, _, closed)| *closed);
    if !any_closed {
        return vec![geometry.copy()];
    }

    let num_contours = all_contour_data.len();
    let mut adj: Vec<Vec<usize>> = vec![vec![]; num_contours];

    for i in 0..num_contours {
        let (_, ref vertices_i, closed_i) = &all_contour_data[i];
        if !closed_i {
            continue;
        }
        for j in 0..num_contours {
            if i == j {
                continue;
            }
            let (_, ref vertices_j, _) = &all_contour_data[j];
            if vertices_j.is_empty() || vertices_i.is_empty() {
                continue;
            }
            if is_point_inside_polygon(vertices_j[0], vertices_i) {
                adj[i].push(j);
                adj[j].push(i);
            }
        }
    }

    let component_indices_list =
        find_connected_components_bfs(num_contours, &adj);

    let mut final_geometries: Vec<Geometry> = Vec::new();
    let mut stray_open = Geometry::new();
    stray_open.uniform_scalable = geometry.uniform_scalable;

    for indices in &component_indices_list {
        let mut component_geo = Geometry::new();
        component_geo.uniform_scalable = geometry.uniform_scalable;
        let mut has_closed = false;

        for &idx in indices {
            let (geo_data, _, closed) = &all_contour_data[idx];
            component_geo.extend(geo_data);
            if *closed {
                has_closed = true;
            }
        }

        if has_closed {
            final_geometries.push(component_geo);
        } else {
            stray_open.extend(&component_geo);
        }
    }

    if !stray_open.is_empty() {
        final_geometries.push(stray_open);
    }

    final_geometries
}

/// Reverse the direction of a single-contour Geometry.
pub fn reverse_contour(contour: &Geometry) -> Geometry {
    let data = &contour.data;
    if data.is_empty() {
        return contour.copy();
    }
    if !matches!(&data[0], Command::Move { .. }) {
        return contour.copy();
    }

    let mut new_rows: Vec<Command> = Vec::with_capacity(data.len());

    let last_point = data[data.len() - 1].end_point();
    new_rows.push(Command::Move { end: last_point });
    let mut last_point = last_point;

    for i in (1..data.len()).rev() {
        let end_cmd = &data[i];
        let start_point = data[i - 1].end_point();

        match end_cmd {
            Command::Line { .. } => {
                new_rows.push(Command::Line { end: start_point });
            }
            Command::Arc {
                center_offset,
                normal,
                ..
            } => {
                let center_abs_x = start_point.x + center_offset.x;
                let center_abs_y = start_point.y + center_offset.y;
                let new_offset_x = center_abs_x - last_point.x;
                let new_offset_y = center_abs_y - last_point.y;
                new_rows.push(Command::Arc {
                    end: start_point,
                    center_offset: Point3D::new(
                        new_offset_x,
                        new_offset_y,
                        0.0,
                    ),
                    normal: Point3D::new(-normal.x, -normal.y, -normal.z),
                });
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                new_rows.push(Command::Bezier {
                    end: start_point,
                    control1: *control2,
                    control2: *control1,
                });
            }
            Command::Move { .. } => {}
        }

        last_point = start_point;
    }

    let mut new_geo = Geometry::new();
    new_geo.data = new_rows;
    new_geo.last_move_to = data[0].end_point();
    new_geo
}

/// Split contours into inner and outer groups based on the even-odd rule.
pub fn split_inner_and_outer_contours(
    contours: &[&Geometry],
) -> (Vec<usize>, Vec<usize>) {
    if contours.is_empty() {
        return (vec![], vec![]);
    }

    let hierarchy = build_hierarchy(contours);

    let mut internal_indices: Vec<usize> = Vec::new();
    let mut external_indices: Vec<usize> = Vec::new();

    for (i, _) in contours.iter().enumerate() {
        let depth = hierarchy.nesting_depths[i];
        if depth < 0 {
            continue;
        }
        if depth % 2 == 0 {
            external_indices.push(i);
        } else {
            internal_indices.push(i);
        }
    }

    (internal_indices, external_indices)
}

/// Close all contours in a geometry.
pub fn close_all_contours(geometry: &Geometry) -> Geometry {
    if geometry.is_empty() {
        return geometry.copy();
    }

    let contours = split_into_contours(geometry);
    if contours.is_empty() {
        return geometry.copy();
    }

    let mut result = Geometry::new();
    for mut contour in contours {
        if !contour.is_closed(1e-6) {
            contour.close_path();
        }
        result.extend(&contour);
    }

    result.last_move_to = geometry.last_move_to;
    result
}

/// Normalize winding orders of contours (CCW for solids, CW for holes).
pub fn normalize_winding_orders(contours: &[&Geometry]) -> Vec<Geometry> {
    if contours.is_empty() {
        return vec![];
    }

    let hierarchy = build_hierarchy(contours);
    let mut normalized_contours: Vec<Geometry> = Vec::new();

    for (i, c) in contours.iter().enumerate() {
        let depth = hierarchy.nesting_depths[i];
        if depth < 0 {
            if !c.is_empty() {
                normalized_contours.push(c.copy());
            }
            continue;
        }

        let signed_area = get_subpath_area_from_array(&c.data, 0);
        let is_ccw = signed_area > 0.0;
        let is_nested_odd = depth % 2 != 0;

        if (is_nested_odd && is_ccw) || (!is_nested_odd && !is_ccw) {
            normalized_contours.push(reverse_contour(c));
        } else {
            normalized_contours.push(c.copy());
        }
    }

    normalized_contours
}

/// Filter to only external contours (solid filled areas).
pub fn filter_to_external_contours(contours: &[&Geometry]) -> Vec<Geometry> {
    if contours.is_empty() {
        return vec![];
    }

    let normalized_contours = normalize_winding_orders(contours);

    let mut final_contours: Vec<Geometry> = Vec::new();
    for c in &normalized_contours {
        if !c.data.is_empty() {
            let area = get_subpath_area_from_array(&c.data, 0);
            if area > 1e-9 {
                final_contours.push(c.copy());
            }
        }
    }
    final_contours
}

/// Remove inner edges (holes) from a geometry, keeping only external contours.
pub fn remove_inner_edges(geometry: &Geometry) -> Geometry {
    if geometry.is_empty() {
        return Geometry::new();
    }

    let all_contours = split_into_contours(geometry);
    if all_contours.is_empty() {
        return Geometry::new();
    }

    let mut closed_contours: Vec<Geometry> = Vec::new();
    let mut open_contours: Vec<Geometry> = Vec::new();

    for contour in all_contours {
        if contour.is_closed(1e-6) {
            closed_contours.push(contour);
        } else {
            open_contours.push(contour);
        }
    }

    let closed_refs: Vec<&Geometry> = closed_contours.iter().collect();
    let external_closed = filter_to_external_contours(&closed_refs);

    let mut final_geo = Geometry::new();
    for contour in &external_closed {
        final_geo.extend(contour);
    }
    for contour in &open_contours {
        final_geo.extend(contour);
    }

    final_geo.last_move_to = geometry.last_move_to;
    final_geo
}
