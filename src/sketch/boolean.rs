//! Boolean operations over sketch geometry: region collection and
//! Clipper-backed result grouping.

use crate::geo::shape::polygon::{
    get_polygon_signed_area, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_union,
};
use crate::geo::types::Polygon;
use crate::sketch::contour::{
    chain_vertices, group_rings_into_solids, partition_components,
    rings_to_geometry, walk_component,
};
use crate::sketch::entities::polygon::PolygonEntity;
use crate::sketch::registry::EntityRegistry;

const MIN_RING_AREA: f64 = 1e-9;

/// Boolean operation kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Difference,
    Intersection,
    Exclude,
}

/// One closed candidate region.
#[derive(Clone, Debug)]
pub struct BooleanRegion {
    pub rings: Vec<Polygon>,
    pub entity_ids: Vec<u64>,
    pub z: usize,
}

/// One result solid: outer ring plus holes.
#[derive(Clone, Debug)]
pub struct BooleanSolid {
    pub outer: Polygon,
    pub holes: Vec<Polygon>,
}

/// Collects closed candidate regions from the selected entities.
pub fn build_boolean_regions(
    registry: &EntityRegistry,
    entity_ids: &[u64],
    coincident_pairs: &[(u64, u64)],
    tolerance: f64,
) -> Option<Vec<BooleanRegion>> {
    let mut regions = Vec::new();
    let mut chainable = Vec::new();
    for &entity_id in entity_ids {
        let Some(entity) = registry.get_entity(entity_id) else {
            continue;
        };
        if entity.base().construction || entity.base().array_copy {
            continue;
        }
        if entity.type_name() == "text_box" {
            continue;
        }
        if entity.get_endpoint_ids().len() == 2 {
            chainable.push(entity_id);
            continue;
        }
        if let Some(rings) = standalone_rings(registry, entity_id, tolerance) {
            regions.push(BooleanRegion {
                rings,
                entity_ids: vec![entity_id],
                z: registry.entity_index(entity_id).unwrap_or(0),
            });
        }
    }

    if !chainable.is_empty() {
        let components =
            partition_components(registry, &chainable, coincident_pairs)?;
        for component in components {
            let (edges, closed) =
                walk_component(registry, &component, coincident_pairs);
            if !closed {
                return None;
            }
            let vertices = chain_vertices(registry, &edges, true, tolerance);
            if vertices.len() < 3 {
                return None;
            }
            let z = component
                .iter()
                .filter_map(|id| registry.entity_index(*id))
                .max()
                .unwrap_or(0);
            regions.push(BooleanRegion {
                rings: vec![to_polygon(&vertices)],
                entity_ids: component,
                z,
            });
        }
    }

    if regions.len() < 2 {
        return None;
    }
    Some(regions)
}

/// Runs a boolean operation and groups the result into solids.
pub fn apply_boolean(
    op: BooleanOp,
    regions: &[BooleanRegion],
    tolerance: f64,
) -> Option<Vec<BooleanSolid>> {
    let rings = match op {
        BooleanOp::Union => get_polygons_union(
            &regions
                .iter()
                .flat_map(|r| r.rings.clone())
                .collect::<Vec<_>>(),
        ),
        BooleanOp::Difference => {
            let ordered = ordered(regions);
            let tail: Vec<Polygon> =
                ordered[1..].iter().flat_map(|r| r.rings.clone()).collect();
            get_polygons_group_difference(&ordered[0].rings, &tail)
        }
        BooleanOp::Intersection => {
            let ordered = ordered(regions);
            let mut result = ordered[0].rings.clone();
            for region in &ordered[1..] {
                result =
                    get_polygons_group_intersection(&result, &region.rings);
                if result.is_empty() {
                    return None;
                }
            }
            result
        }
        BooleanOp::Exclude => {
            let mut result = regions[0].rings.clone();
            for region in &regions[1..] {
                result = symmetric_difference(&result, &region.rings);
                if result.is_empty() {
                    return None;
                }
            }
            result
        }
    };

    let cleaned: Vec<Polygon> = rings_to_geometry(
        &rings
            .iter()
            .map(|ring| ring.iter().map(|p| (p.x, p.y)).collect())
            .collect::<Vec<_>>(),
    )
    .to_polygons(tolerance)
    .into_iter()
    .filter(|ring| get_polygon_signed_area(ring).abs() > MIN_RING_AREA)
    .collect();
    let grouped = group_rings_into_solids(
        &cleaned
            .iter()
            .map(|ring| ring.iter().map(|p| (p.x, p.y)).collect())
            .collect::<Vec<_>>(),
    );
    if grouped.is_empty() {
        return None;
    }
    Some(
        grouped
            .into_iter()
            .map(|(outer, holes)| BooleanSolid {
                outer: to_polygon(&outer),
                holes: holes.iter().map(|h| to_polygon(h)).collect(),
            })
            .collect(),
    )
}

fn ordered(regions: &[BooleanRegion]) -> Vec<&BooleanRegion> {
    let mut ordered: Vec<&BooleanRegion> = regions.iter().collect();
    ordered.sort_by_key(|r| r.z);
    ordered
}

fn symmetric_difference(a: &[Polygon], b: &[Polygon]) -> Vec<Polygon> {
    let ab = get_polygons_group_difference(a, b);
    let ba = get_polygons_group_difference(b, a);
    get_polygons_union(&[ab, ba].concat())
}

fn standalone_rings(
    registry: &EntityRegistry,
    entity_id: u64,
    tolerance: f64,
) -> Option<Vec<Polygon>> {
    let entity = registry.get_entity(entity_id)?;
    if let Some(polygon) = entity.as_any().downcast_ref::<PolygonEntity>() {
        if !polygon.closed {
            return None;
        }
        return Some(ensure_winding(polygon.get_world_rings(registry)));
    }
    if !entity.is_closed_loop() {
        return None;
    }
    let ring = entity.to_polyline(registry, tolerance);
    if ring.len() < 3 {
        return None;
    }
    Some(ensure_winding(vec![ring]))
}

fn ensure_winding(rings: Vec<Vec<(f64, f64)>>) -> Vec<Polygon> {
    let geo = rings_to_geometry(&rings);
    let contours = crate::geo::algo::topology::split_into_contours(&geo);
    let refs: Vec<&crate::geo::Geometry> = contours.iter().collect();
    let normalized =
        crate::geo::algo::topology::normalize_winding_orders(&refs);
    let mut merged = crate::geo::Geometry::new();
    for contour in &normalized {
        merged.extend(contour);
    }
    merged.to_polygons(0.02)
}

fn to_polygon(points: &[(f64, f64)]) -> Polygon {
    points
        .iter()
        .map(|&(x, y)| crate::geo::types::Point::new(x, y))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_of_two_squares() {
        let a = BooleanRegion {
            rings: vec![to_polygon(&[
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 10.0),
                (0.0, 10.0),
            ])],
            entity_ids: vec![1],
            z: 0,
        };
        let b = BooleanRegion {
            rings: vec![to_polygon(&[
                (10.0, 0.0),
                (20.0, 0.0),
                (20.0, 10.0),
                (10.0, 10.0),
            ])],
            entity_ids: vec![2],
            z: 1,
        };
        let result = apply_boolean(BooleanOp::Union, &[a, b], 0.02).unwrap();
        assert_eq!(result.len(), 1);
    }
}
