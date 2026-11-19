//! Ephemeral views over groups of entities.
//!
//! An [`EntityGroup`] is cheap to construct and never stored: commands,
//! arrays and tools build one for the duration of a single operation,
//! read the group's geometry and apply transforms through it.

use std::collections::{HashMap, HashSet};

use crate::sketch::entities::point::SketchPoint;
use crate::sketch::entities::Entity;
use crate::sketch::registry::EntityRegistry;
use crate::sketch::transform::PlacementTransform;

/// The bbox center of a set of points.
pub fn points_bbox_center(points: &[SketchPoint]) -> (f64, f64) {
    if points.is_empty() {
        return (0.0, 0.0);
    }
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0)
}

/// A set of entities plus their unique defining points, treated as one
/// piece of geometry.
#[derive(Clone, Debug)]
pub struct EntityGroup {
    pub entity_ids: Vec<u64>,
}

impl EntityGroup {
    /// Creates a group over the given entities.
    pub fn new(entity_ids: Vec<u64>) -> Self {
        EntityGroup { entity_ids }
    }

    /// The group's surviving entities, in group order.
    pub fn entities<'a>(
        &self,
        registry: &'a EntityRegistry,
    ) -> Vec<&'a dyn Entity> {
        self.entity_ids
            .iter()
            .filter_map(|&id| registry.get_entity(id))
            .collect()
    }

    /// The group's unique point ids in stable order.
    pub fn point_ids(&self, registry: &EntityRegistry) -> Vec<u64> {
        let mut seen = HashSet::new();
        let mut point_ids = Vec::new();
        for entity in self.entities(registry) {
            for point_id in entity.get_point_ids() {
                if seen.insert(point_id) {
                    point_ids.push(point_id);
                }
            }
        }
        point_ids
    }

    /// The group's unique defining points.
    pub fn points(&self, registry: &EntityRegistry) -> Vec<SketchPoint> {
        self.point_ids(registry)
            .into_iter()
            .filter_map(|point_id| registry.get_point(point_id))
            .collect()
    }

    /// Logical center of the group's entities.
    pub fn center(&self, registry: &EntityRegistry) -> (f64, f64) {
        let shapes: Vec<&dyn Entity> = self
            .entities(registry)
            .into_iter()
            .filter(|entity| matches!(entity.type_name(), "circle" | "ellipse"))
            .collect();
        if let [shape] = shapes.as_slice() {
            if let Some(point_id) = shape.center_point_id() {
                if let Some(point) = registry.get_point(point_id) {
                    return (point.x, point.y);
                }
            }
        }
        points_bbox_center(&self.points(registry))
    }

    /// Rigidly moves the group's points and internal offsets.
    pub fn apply_placement(
        &self,
        registry: &mut EntityRegistry,
        transform: &dyn PlacementTransform,
    ) {
        let moved: Vec<(u64, f64, f64)> = self
            .points(registry)
            .into_iter()
            .map(|point| {
                let (x, y) = transform.transform_point(point.x, point.y);
                (point.id, x, y)
            })
            .collect();
        for (point_id, x, y) in moved {
            registry.set_point_position(point_id, x, y);
        }
        let entities: Vec<Box<dyn Entity>> = self
            .entities(registry)
            .into_iter()
            .map(|entity| entity.clone_box())
            .collect();
        for mut entity in entities {
            let entity_id = entity.base().id;
            entity.transform_offsets(transform);
            registry.replace_entity(entity_id, entity);
        }
    }

    /// Moves every point of the group by the given delta.
    pub fn translate(&self, registry: &mut EntityRegistry, dx: f64, dy: f64) {
        for point in self.points(registry) {
            registry.set_point_position(point.id, point.x + dx, point.y + dy);
        }
    }

    /// Moves the group radially so its center sits on a circle.
    pub fn radial_project(
        &self,
        registry: &mut EntityRegistry,
        center: (f64, f64),
        radius: f64,
        pin: Option<(f64, f64)>,
    ) {
        if self.points(registry).is_empty() {
            return;
        }
        let (center_x, center_y) = pin.unwrap_or_else(|| self.center(registry));
        let (vx, vy) = (center_x - center.0, center_y - center.1);
        let distance = vx.hypot(vy);
        if distance < 1e-9 {
            return;
        }
        let scale = (radius - distance) / distance;
        self.translate(registry, vx * scale, vy * scale);
    }

    /// Rewrites an existing copy group from this template group.
    pub fn rewrite_copy_from(
        &self,
        registry: &mut EntityRegistry,
        copy: &EntityGroup,
        transform: &dyn PlacementTransform,
        extra_point_pairs: &[(u64, u64)],
    ) -> Vec<(u64, f64, f64)> {
        let template_entities: Vec<Box<dyn Entity>> = self
            .entities(registry)
            .into_iter()
            .map(|entity| entity.clone_box())
            .collect();
        let copy_entities: Vec<Box<dyn Entity>> = copy
            .entities(registry)
            .into_iter()
            .map(|entity| entity.clone_box())
            .collect();

        let mut applied = Vec::new();
        for (template, copy_entity) in
            template_entities.iter().zip(copy_entities.iter())
        {
            let template_point_ids = template.get_point_ids();
            let copy_point_ids = copy_entity.get_point_ids();
            for (&template_id, &copy_id) in
                template_point_ids.iter().zip(copy_point_ids.iter())
            {
                let Some(template_point) = registry.get_point(template_id)
                else {
                    continue;
                };
                let Some(copy_point) = registry.get_point(copy_id) else {
                    continue;
                };
                let (x, y) = transform
                    .transform_point(template_point.x, template_point.y);
                registry.set_point_position(copy_point.id, x, y);
                applied.push((copy_point.id, x, y));
            }
            let mut copy_entity = copy_entity.clone_box();
            let entity_id = copy_entity.base().id;
            copy_entity.rewrite_offsets_from(&**template, transform);
            registry.replace_entity(entity_id, copy_entity);
        }

        for &(template_id, copy_id) in extra_point_pairs {
            let Some(template_point) = registry.get_point(template_id) else {
                continue;
            };
            let Some(copy_point) = registry.get_point(copy_id) else {
                continue;
            };
            let (x, y) =
                transform.transform_point(template_point.x, template_point.y);
            registry.set_point_position(copy_point.id, x, y);
            applied.push((copy_point.id, x, y));
        }
        applied
    }

    /// Captures the current position of every point of the group.
    pub fn snapshot_positions(
        &self,
        registry: &EntityRegistry,
    ) -> Vec<(u64, f64, f64)> {
        self.points(registry)
            .into_iter()
            .map(|point| (point.id, point.x, point.y))
            .collect()
    }

    /// Samples every entity into polylines in model coordinates.
    pub fn polylines(&self, registry: &EntityRegistry) -> Vec<Vec<(f64, f64)>> {
        self.entities(registry)
            .into_iter()
            .flat_map(|entity| entity.to_polylines(registry, 0.1))
            .collect()
    }

    /// Helper geometry belonging to the group's entities.
    pub fn helper_ids(&self, registry: &EntityRegistry) -> Vec<u64> {
        let mut helper_ids = Vec::new();
        for entity in self.entities(registry) {
            helper_ids.extend(entity.get_helper_ids());
        }
        let template_points: HashSet<u64> =
            self.point_ids(registry).into_iter().collect();
        let mut taken: HashSet<u64> = helper_ids.iter().copied().collect();
        taken.extend(self.entity_ids.iter().copied());
        for entity in registry.all_entities() {
            let entity_id = entity.base().id;
            if taken.contains(&entity_id) {
                continue;
            }
            if !(entity.base().construction || entity.base().invisible) {
                continue;
            }
            let point_ids = entity.get_point_ids();
            if !point_ids.is_empty()
                && point_ids.iter().all(|id| template_points.contains(id))
            {
                helper_ids.push(entity_id);
            }
        }
        helper_ids
    }

    /// A view extended with the group's helper entities.
    pub fn with_helpers(&self, registry: &EntityRegistry) -> EntityGroup {
        let mut entity_ids = self.entity_ids.clone();
        entity_ids.extend(self.helper_ids(registry));
        EntityGroup::new(entity_ids)
    }

    /// Rewrites the group's entity point references.
    pub fn remap_point_refs(
        &self,
        registry: &mut EntityRegistry,
        pid_map: &HashMap<u64, u64>,
    ) {
        let map = |id: u64| pid_map.get(&id).copied().unwrap_or(id);
        let entities: Vec<Box<dyn Entity>> = self
            .entities(registry)
            .into_iter()
            .map(|entity| entity.clone_box())
            .collect();
        for mut entity in entities {
            let entity_id = entity.base().id;
            entity.remap_references(&map);
            registry.replace_entity(entity_id, entity);
        }
    }
}
