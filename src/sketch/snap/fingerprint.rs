//! Stable fingerprints of the geometry the snap caches depend on.
//!
//! Built from typed values (ids, flags, serialized signatures) so a
//! comparison never depends on Python `repr()` strings.

use std::collections::HashSet;

use crate::sketch::registry::EntityRegistry;

use super::types::SnapContext;

/// The `(excluded_entities, excluded_points)` sets a drag context
/// implies: the dragged/preview entities plus every point they
/// reference.  Shared by the geometry fingerprint and the
/// intersection cache key.
pub fn excluded_sets(
    registry: &EntityRegistry,
    context: Option<&SnapContext>,
) -> (HashSet<u64>, HashSet<u64>) {
    let empty = SnapContext::default();
    let context = context.unwrap_or(&empty);
    let mut excluded_entities: HashSet<u64> =
        context.dragged_entity_ids.iter().copied().collect();
    excluded_entities.extend(context.preview_entity_ids.iter().copied());
    let mut excluded_points: HashSet<u64> =
        context.dragged_point_ids.iter().copied().collect();
    for entity_id in &excluded_entities {
        if let Some(ids) = registry.entity_point_ids(*entity_id) {
            excluded_points.extend(ids);
        }
    }
    (excluded_entities, excluded_points)
}

/// Whether the entity participates in snap geometry: not excluded,
/// no dragged points, and not construction when those are excluded.
pub fn entity_participates(
    entity: &dyn crate::sketch::entities::entity::Entity,
    excluded_entities: &HashSet<u64>,
    excluded_points: &HashSet<u64>,
    include_construction: bool,
) -> bool {
    if excluded_entities.contains(&entity.base().id) {
        return false;
    }
    if entity
        .get_point_ids()
        .iter()
        .any(|p| excluded_points.contains(p))
    {
        return false;
    }
    include_construction || !entity.base().construction
}

/// Stable identity of the geometry a snap index depends on.
///
/// Deterministic for a given registry state: entity part in id
/// order, optional point part in id order, coordinates rounded to
/// the model's six-decimal precision.
pub fn geometry_fingerprint(
    registry: &EntityRegistry,
    context: Option<&SnapContext>,
    include_construction: bool,
    include_points: bool,
) -> String {
    let (excluded_entities, excluded_points) = excluded_sets(registry, context);
    let mut parts: Vec<String> = Vec::new();
    for entity_id in registry.entity_ids() {
        let Some(entity) = registry.get_entity(entity_id) else {
            continue;
        };
        if !entity_participates(
            entity,
            &excluded_entities,
            &excluded_points,
            include_construction,
        ) {
            continue;
        }
        parts.push(format!(
            "entity:{}:{}:{}:{}:{}",
            entity_id,
            entity.type_name(),
            entity.base().construction,
            entity.base().invisible,
            entity.geometry_signature(registry)
        ));
    }
    if include_points {
        for id in registry.point_ids() {
            if excluded_points.contains(&id) {
                continue;
            }
            let Some(point) = registry.get_point(id) else {
                continue;
            };
            parts.push(format!(
                "point:{id}:{:.6}:{:.6}",
                (point.x * 1e6).round() / 1e6,
                (point.y * 1e6).round() / 1e6,
            ));
        }
    }
    parts.join("|")
}
