//! Contour preparation: connected components, chain orientation and
//! sampled chain vertices for offset and boolean operations.

use crate::sketch::union_find::UnionFind;

use std::collections::HashMap;

use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_closest_point, get_polygon_convex_hull,
    is_point_inside_polygon,
};
use crate::geo::types::{Point, Polygon};
use crate::geo::Geometry;
use crate::sketch::registry::EntityRegistry;

const JUNCTION_EPS: f64 = 1e-6;

/// One offset/boolean result: an outer ring plus its holes.
pub type Solid = (Vec<(f64, f64)>, Vec<Vec<(f64, f64)>>);

/// Partitions endpoint-bearing entities into connected components.
///
/// Returns `None` when a junction connects more than two segments,
/// matching the reference offset preprocessing.
pub fn partition_components(
    registry: &EntityRegistry,
    entity_ids: &[u64],
    coincident_pairs: &[(u64, u64)],
) -> Option<Vec<Vec<u64>>> {
    let edges = endpoint_edges(registry, entity_ids).ok()?;
    let mut parent =
        UnionFind::with_ids(edges.iter().flat_map(|e| [e.p1, e.p2]));
    for &(a, b) in coincident_pairs {
        parent.union_if_present(a, b);
    }

    let mut node_edges: HashMap<u64, Vec<usize>> = HashMap::new();
    for (index, edge) in edges.iter().enumerate() {
        node_edges
            .entry(parent.find(edge.p1))
            .or_default()
            .push(index);
        node_edges
            .entry(parent.find(edge.p2))
            .or_default()
            .push(index);
    }
    if node_edges.values().any(|edges| edges.len() > 2) {
        return None;
    }

    let mut visited = vec![false; edges.len()];
    let mut components = Vec::new();
    for start in 0..edges.len() {
        if visited[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut component = Vec::new();
        while let Some(index) = stack.pop() {
            if visited[index] {
                continue;
            }
            visited[index] = true;
            component.push(edges[index].entity_id);
            let edge = &edges[index];
            for node in [parent.find(edge.p1), parent.find(edge.p2)] {
                if let Some(neighbors) = node_edges.get(&node) {
                    stack.extend(
                        neighbors
                            .iter()
                            .copied()
                            .filter(|other| !visited[*other]),
                    );
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    Some(components)
}

/// Orders one component into a contiguous chain.
pub fn walk_component(
    registry: &EntityRegistry,
    entity_ids: &[u64],
    coincident_pairs: &[(u64, u64)],
) -> (Vec<(u64, bool)>, bool) {
    let Ok(edges) = endpoint_edges(registry, entity_ids) else {
        return (Vec::new(), false);
    };
    if edges.is_empty() {
        return (Vec::new(), false);
    }

    let mut parent =
        UnionFind::with_ids(edges.iter().flat_map(|e| [e.p1, e.p2]));
    for &(a, b) in coincident_pairs {
        parent.union_if_present(a, b);
    }
    let nodes: Vec<(u64, u64)> = edges
        .iter()
        .map(|edge| (parent.find(edge.p1), parent.find(edge.p2)))
        .collect();
    let mut node_edges: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut node_order: Vec<u64> = Vec::new();
    for (index, (a, b)) in nodes.iter().enumerate() {
        for node in [*a, *b] {
            if !node_order.contains(&node) {
                node_order.push(node);
            }
            node_edges.entry(node).or_default().push(index);
        }
    }

    let start = node_order
        .iter()
        .copied()
        .find(|node| node_edges.get(node).is_some_and(|e| e.len() == 1))
        .unwrap_or(nodes[0].0);
    let closed = node_edges.get(&start).map(Vec::len) == Some(2);

    let mut used = vec![false; edges.len()];
    let mut ordered = Vec::new();
    let mut current = start;
    loop {
        let next = node_edges.get(&current).and_then(|indices| {
            indices.iter().copied().find(|index| !used[*index])
        });
        let Some(index) = next else {
            break;
        };
        let (a, b) = nodes[index];
        let forward = a == current;
        ordered.push((edges[index].entity_id, forward));
        used[index] = true;
        current = if forward { b } else { a };
    }
    (ordered, closed)
}

/// Samples an ordered chain into one deduplicated vertex list.
pub fn chain_vertices(
    registry: &EntityRegistry,
    edges: &[(u64, bool)],
    closed: bool,
    tolerance: f64,
) -> Vec<(f64, f64)> {
    let mut vertices = Vec::new();
    for &(entity_id, forward) in edges {
        let Some(entity) = registry.get_entity(entity_id) else {
            continue;
        };
        let mut points = entity.to_polyline(registry, tolerance);
        if !forward {
            points.reverse();
        }
        for point in points {
            if vertices.last().is_some_and(|last: &(f64, f64)| {
                (last.0 - point.0).abs() < JUNCTION_EPS
                    && (last.1 - point.1).abs() < JUNCTION_EPS
            }) {
                continue;
            }
            vertices.push(point);
        }
    }
    if closed && vertices.len() > 1 {
        let first = vertices[0];
        let last = *vertices.last().unwrap();
        if (first.0 - last.0).abs() < JUNCTION_EPS
            && (first.1 - last.1).abs() < JUNCTION_EPS
        {
            vertices.pop();
        }
    }
    vertices
}

/// Builds the frame `(center, handle)` for an outline.
pub fn outline_frame(
    vertices: &[(f64, f64)],
    closed: bool,
) -> ((f64, f64), (f64, f64)) {
    if vertices.is_empty() {
        return ((0.0, 0.0), (1.0, 0.0));
    }
    let xs: Vec<f64> = vertices.iter().map(|v| v.0).collect();
    let ys: Vec<f64> = vertices.iter().map(|v| v.1).collect();
    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut center = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
    if closed {
        let polygon: Polygon =
            vertices.iter().map(|&(x, y)| Point::new(x, y)).collect();
        if let Some(inner) = find_largest_circle(&polygon, &[], 0.5) {
            center = (inner.0.x, inner.0.y);
        }
    }

    let mut handle = None;
    let mut best_dist_sq = 1e-12;
    for (rx, ry) in short_edge_midpoints(vertices) {
        let Some((point, _dist_sq)) =
            closest_outline_point(vertices, closed, rx, ry)
        else {
            continue;
        };
        let dx = point.0 - center.0;
        let dy = point.1 - center.1;
        let frame_dist_sq = dx * dx + dy * dy;
        if frame_dist_sq > best_dist_sq {
            best_dist_sq = frame_dist_sq;
            handle = Some(point);
        }
    }
    if let Some(handle) = handle {
        return (center, handle);
    }
    let mut scale = (max_x - min_x) / 2.0;
    if scale <= 1e-9 {
        scale = ((max_y - min_y) / 2.0).max(1.0);
    }
    (center, (center.0 + scale, center.1))
}

fn short_edge_midpoints(vertices: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let hull: Vec<(f64, f64)> = get_polygon_convex_hull(
        &vertices
            .iter()
            .map(|&(x, y)| Point::new(x, y))
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .map(|p| (p.x, p.y))
    .collect();
    if hull.len() < 2 {
        return Vec::new();
    }
    type BestEdge = (f64, (f64, f64), (f64, f64));
    let mut best: Option<BestEdge> = None;
    for i in 0..hull.len() {
        let p1 = hull[i];
        let p2 = hull[(i + 1) % hull.len()];
        let (ex, ey) = (p2.0 - p1.0, p2.1 - p1.1);
        let edge_len = ex.hypot(ey);
        if edge_len < 1e-12 {
            continue;
        }
        let (ux, uy) = (ex / edge_len, ey / edge_len);
        let mut us = Vec::new();
        let mut vs = Vec::new();
        for &(px, py) in &hull {
            let (dx, dy) = (px - p1.0, py - p1.1);
            us.push(dx * ux + dy * uy);
            vs.push(-dx * uy + dy * ux);
        }
        let min_u = us.iter().copied().fold(f64::INFINITY, f64::min);
        let max_u = us.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_v = vs.iter().copied().fold(f64::INFINITY, f64::min);
        let max_v = vs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let (w, h) = (max_u - min_u, max_v - min_v);
        let area = w * h;
        if best.is_some_and(|b| area >= b.0) {
            continue;
        }
        let u_mid = (max_u + min_u) / 2.0;
        let v_mid = (max_v + min_v) / 2.0;
        let center = (
            p1.0 + u_mid * ux - v_mid * uy,
            p1.1 + u_mid * uy + v_mid * ux,
        );
        let (long_dir, long_half) = if w >= h {
            ((ux, uy), w / 2.0)
        } else {
            ((-uy, ux), h / 2.0)
        };
        best = Some((
            area,
            (
                center.0 + long_dir.0 * long_half,
                center.1 + long_dir.1 * long_half,
            ),
            (
                center.0 - long_dir.0 * long_half,
                center.1 - long_dir.1 * long_half,
            ),
        ));
    }
    best.map(|(_, a, b)| vec![a, b]).unwrap_or_default()
}

fn closest_outline_point(
    vertices: &[(f64, f64)],
    closed: bool,
    x: f64,
    y: f64,
) -> Option<((f64, f64), f64)> {
    if closed {
        let polygon: Polygon = vertices
            .iter()
            .map(|&(px, py)| Point::new(px, py))
            .collect();
        let (_, point, dist_sq) = get_polygon_closest_point(&polygon, x, y)?;
        return Some(((point.x, point.y), dist_sq));
    }
    let mut best: Option<((f64, f64), f64)> = None;
    for pair in vertices.windows(2) {
        let p1 = Point::new(pair[0].0, pair[0].1);
        let p2 = Point::new(pair[1].0, pair[1].1);
        let (_, point, dist) = get_line_segment_closest_point(p1, p2, x, y);
        if best.is_none_or(|b| dist * dist < b.1) {
            best = Some(((point.x, point.y), dist * dist));
        }
    }
    best
}

/// Builds a multi-ring closed geometry from a flat ring list.
pub fn rings_to_geometry(rings: &[Vec<(f64, f64)>]) -> Geometry {
    let mut geo = Geometry::new();
    for ring in rings {
        if ring.len() < 3 {
            continue;
        }
        geo.move_to(ring[0].0, ring[0].1, 0.0);
        for (x, y) in &ring[1..] {
            geo.line_to(*x, *y, 0.0);
        }
        geo.close_path();
    }
    geo
}

/// Groups a flat ring list into `(outer, holes)` solids.
pub fn group_rings_into_solids(rings: &[Vec<(f64, f64)>]) -> Vec<Solid> {
    let geo = rings_to_geometry(rings);
    if geo.is_empty() {
        return Vec::new();
    }
    let contours = crate::geo::algo::topology::split_into_contours(&geo);
    let refs: Vec<&Geometry> = contours.iter().collect();
    let normalized =
        crate::geo::algo::topology::normalize_winding_orders(&refs);
    let mut merged = Geometry::new();
    for contour in &normalized {
        merged.extend(contour);
    }
    let (outers, holes) = merged.split_inner_and_outer_polygons();
    let mut solids: Vec<(Polygon, Vec<Polygon>)> = outers
        .into_iter()
        .map(|outer| (outer, Vec::new()))
        .collect();
    solids.sort_by(|a, b| {
        get_polygon_area(&a.0)
            .partial_cmp(&get_polygon_area(&b.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for hole in holes {
        if hole.is_empty() {
            continue;
        }
        let point = Point::new(hole[0].x, hole[0].y);
        if let Some((_, assigned)) = solids
            .iter_mut()
            .find(|(outer, _)| is_point_inside_polygon(point, outer))
        {
            assigned.push(hole);
        }
    }
    solids
        .into_iter()
        .map(|(outer, holes)| {
            (
                outer.into_iter().map(|p| (p.x, p.y)).collect(),
                holes
                    .into_iter()
                    .map(|hole| hole.into_iter().map(|p| (p.x, p.y)).collect())
                    .collect(),
            )
        })
        .collect()
}

struct EndpointEdge {
    entity_id: u64,
    p1: u64,
    p2: u64,
}

fn endpoint_edges(
    registry: &EntityRegistry,
    entity_ids: &[u64],
) -> Result<Vec<EndpointEdge>, ()> {
    let mut edges = Vec::new();
    for &entity_id in entity_ids {
        let Some(entity) = registry.get_entity(entity_id) else {
            return Err(());
        };
        let endpoints = entity.get_endpoint_ids();
        if endpoints.len() != 2 {
            return Err(());
        }
        edges.push(EndpointEdge {
            entity_id,
            p1: endpoints[0],
            p2: endpoints[1],
        });
    }
    Ok(edges)
}

/// Offsets a single connected outline. Closed outlines grow/shrink
/// through the geometry offsetter; open outlines become a closed
/// slot-like outline with round caps.
pub fn offset_outline(
    vertices: &[(f64, f64)],
    closed: bool,
    distance: f64,
    tolerance: f64,
) -> Vec<(Vec<(f64, f64)>, bool)> {
    use crate::geo::algo::offset::grow_geometry;
    use crate::geo::shape::polygon::{offset_polyline, EndStyle, JoinStyle};

    if vertices.len() < 2 || distance.abs() < 1e-9 {
        return Vec::new();
    }
    if closed {
        let mut geo = Geometry::new();
        geo.move_to(vertices[0].0, vertices[0].1, 0.0);
        for (x, y) in &vertices[1..] {
            geo.line_to(*x, *y, 0.0);
        }
        geo.close_path();
        let grown = grow_geometry(&geo, distance);
        return grown
            .to_polygons(tolerance)
            .into_iter()
            .map(|p| (p.iter().map(|v| (v.x, v.y)).collect(), true))
            .collect();
    }
    offset_polyline(
        &vertices.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        distance,
        JoinStyle::Miter,
        EndStyle::Round,
    )
    .into_iter()
    .map(|p| (p.iter().map(|v| (v.x, v.y)).collect(), true))
    .collect()
}

/// The `(center, handle)` frame of an outline: the largest inscribed
/// circle's center for closed outlines, and the outline point
/// farthest from it along the flattest hull edges as the handle.
pub fn frame_for_outline(
    vertices: &[(f64, f64)],
    closed: bool,
) -> ((f64, f64), (f64, f64)) {
    use crate::geo::algo::polylabel::find_largest_circle;

    let xs: Vec<f64> = vertices.iter().map(|v| v.0).collect();
    let ys: Vec<f64> = vertices.iter().map(|v| v.1).collect();
    let (min_x, max_x) = (
        xs.iter().cloned().fold(f64::MAX, f64::min),
        xs.iter().cloned().fold(f64::MIN, f64::max),
    );
    let (min_y, max_y) = (
        ys.iter().cloned().fold(f64::MAX, f64::min),
        ys.iter().cloned().fold(f64::MIN, f64::max),
    );
    let mut cx = (min_x + max_x) / 2.0;
    let mut cy = (min_y + max_y) / 2.0;
    if closed {
        let polygon: Polygon =
            vertices.iter().map(|&(x, y)| Point::new(x, y)).collect();
        if let Some((center, _)) = find_largest_circle(&polygon, &[], 0.5) {
            cx = center.x;
            cy = center.y;
        }
    }
    let mut handle: Option<(f64, f64)> = None;
    let mut best_dist_sq = 1e-12;
    for reference in short_edge_midpoints(vertices) {
        let Some((point, _)) =
            closest_outline_point(vertices, closed, reference.0, reference.1)
        else {
            continue;
        };
        let dx = point.0 - cx;
        let dy = point.1 - cy;
        let frame_dist_sq = dx * dx + dy * dy;
        if frame_dist_sq > best_dist_sq {
            best_dist_sq = frame_dist_sq;
            handle = Some(point);
        }
    }
    if let Some(handle) = handle {
        return ((cx, cy), handle);
    }
    let mut scale = (max_x - min_x) / 2.0;
    if scale <= 1e-9 {
        scale = ((max_y - min_y) / 2.0).max(1.0);
    }
    ((cx, cy), (cx + scale, cy))
}

/// Maps world vertices into the outline's local frame: the handle
/// becomes `(1, 0)` relative to the center, distances normalized.
pub fn normalize_vertices(
    center: (f64, f64),
    handle: (f64, f64),
    world_vertices: &[(f64, f64)],
) -> Vec<(f64, f64)> {
    let dx = handle.0 - center.0;
    let dy = handle.1 - center.1;
    let scale = dx.hypot(dy);
    if scale < 1e-9 {
        return Vec::new();
    }
    let (ux, uy) = (dx / scale, dy / scale);
    world_vertices
        .iter()
        .map(|(x, y)| {
            (
                ((x - center.0) * ux + (y - center.1) * uy) / scale,
                (-(x - center.0) * uy + (y - center.1) * ux) / scale,
            )
        })
        .collect()
}
