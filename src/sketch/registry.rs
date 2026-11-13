//! The sketch model registry: points, entities, id allocation and
//! usage tracking — the Rust counterpart of the sketcher's
//! `EntityRegistry`.

use std::collections::HashMap;

use serde_json::Value;

use crate::sketch::constraints::json::{constraint_from_dict, ConstraintEntry};
use crate::sketch::entities::entity::Entity;
pub use crate::sketch::entities::entity::FontConfig;
use crate::sketch::entities::entity_from_json;
use crate::sketch::entities::point::SketchPoint;

use crate::sketch::constraint::{PointContext, SketchConstraint};

/// Stores all points and entities of a sketch document.
#[derive(Default)]
pub struct EntityRegistry {
    points: Vec<SketchPoint>,
    point_index: HashMap<u64, usize>,
    entities: Vec<Box<dyn Entity>>,
    entity_index: HashMap<u64, usize>,
    constraints: Vec<ConstraintEntry>,
    id_counter: u64,
    entity_version: u64,
    point_usage: HashMap<u64, u64>,
}

impl EntityRegistry {
    /// Creates an empty model.
    pub fn new() -> Self {
        EntityRegistry::default()
    }

    // ── lookups ─────────────────────────────────────────────────

    /// Point count.
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Entity count.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// All point ids in insertion order.
    pub fn point_ids(&self) -> Vec<u64> {
        self.points.iter().map(|p| p.id).collect()
    }

    /// All entity ids in insertion order.
    pub fn entity_ids(&self) -> Vec<u64> {
        self.entities.iter().map(|e| e.base().id).collect()
    }

    /// Returns a copy of the point with the given id.
    pub fn get_point(&self, id: u64) -> Option<SketchPoint> {
        self.point_index.get(&id).map(|&i| self.points[i].clone())
    }

    /// Coordinates of a point, for geometry helpers.
    pub fn point_position(&self, id: u64) -> Option<(f64, f64)> {
        self.point_index
            .get(&id)
            .map(|&i| (self.points[i].x, self.points[i].y))
    }

    /// Sets a point's coordinates.
    pub fn set_point_position(&mut self, id: u64, x: f64, y: f64) -> bool {
        match self.point_index.get(&id) {
            Some(&i) => {
                self.points[i].x = x;
                self.points[i].y = y;
                true
            }
            None => false,
        }
    }

    /// Sets a point's solver-constrained flag.
    pub fn set_point_constrained(
        &mut self,
        id: u64,
        constrained: bool,
    ) -> bool {
        match self.point_index.get(&id) {
            Some(&i) => {
                self.points[i].constrained = constrained;
                true
            }
            None => false,
        }
    }

    /// Returns a shared reference to the entity with the given id.
    pub fn get_entity(&self, id: u64) -> Option<&dyn Entity> {
        self.entity_index
            .get(&id)
            .map(|&i| self.entities[i].as_ref())
    }

    /// Returns a mutable reference to the entity with the given id.
    pub fn get_entity_mut(&mut self, id: u64) -> Option<&mut Box<dyn Entity>> {
        self.entity_index.get(&id).map(|&i| &mut self.entities[i])
    }

    /// The reference serialization type string of an entity.
    pub fn entity_type(&self, id: u64) -> Option<&'static str> {
        self.get_entity(id).map(|e| e.type_name())
    }

    /// IDs of all control points of an entity.
    pub fn entity_point_ids(&self, id: u64) -> Option<Vec<u64>> {
        self.get_entity(id).map(|e| e.get_point_ids())
    }

    /// The two endpoint ids of an edge entity.
    pub fn entity_endpoint_ids(&self, id: u64) -> Option<Vec<u64>> {
        self.get_entity(id).map(|e| e.get_endpoint_ids())
    }

    /// Whether the point is one of a text box frame's corner points.
    pub fn point_is_in_text_box_frame(&self, point_id: u64) -> bool {
        for entity in self.entities.iter() {
            if entity.get_all_frame_point_ids(self).contains(&point_id) {
                return true;
            }
        }
        false
    }

    /// Point ids that move rigidly with `point_id` during dragging.
    pub fn get_rigidly_connected_points(&self, point_id: u64) -> Vec<u64> {
        let mut result = Vec::new();
        for entity in self.entities.iter() {
            result.extend(entity.get_rigidly_connected_points(point_id));
        }
        result.sort_unstable();
        result.dedup();
        result
    }

    /// Point ids pinned while `point_id` is dragged.
    pub fn get_drag_anchor_points(&self, point_id: u64) -> Vec<u64> {
        let mut result = Vec::new();
        for entity in self.entities.iter() {
            result.extend(entity.get_drag_anchor_points(point_id));
        }
        result.sort_unstable();
        result.dedup();
        result
    }

    /// Whether the point is referenced by any entity.
    pub fn is_point_used(&self, id: u64) -> bool {
        self.point_usage.get(&id).copied().unwrap_or(0) > 0
    }

    /// Monotonic version bumped on every entity-list mutation.
    /// All sketch points, in storage order.
    pub fn all_points(&self) -> &[SketchPoint] {
        &self.points
    }

    /// All entities, in storage order.
    pub fn all_entities(&self) -> &[Box<dyn Entity>] {
        &self.entities
    }

    /// Appends a pre-built entity object, assigning the next free
    /// id, and returns it.
    pub fn add_entity_object(&mut self, mut entity: Box<dyn Entity>) -> u64 {
        let id = self.next_id();
        entity.base_mut().id = id;
        self.push_entity(entity)
    }

    pub fn entity_version(&self) -> u64 {
        self.entity_version
    }

    /// Adds a constraint from a reference constraint dictionary
    /// (`.ryp` shape), resolving entity references through this
    /// registry.  Returns the constraint handle (list index).
    pub fn add_constraint_json(&mut self, dict: &Value) -> Result<u64, String> {
        let constraint = constraint_from_dict(dict, self)
            .ok_or_else(|| "unsupported constraint dict".to_string())?;
        Ok(self.push_constraint(constraint, dict.clone()))
    }

    /// Appends a pre-built constraint object, storing its reference
    /// dictionary, and returns the constraint handle (list index).
    pub fn add_constraint_object(
        &mut self,
        constraint: Box<dyn SketchConstraint>,
    ) -> u64 {
        let dict = constraint.to_json();
        self.push_constraint(constraint, dict)
    }

    fn push_constraint(
        &mut self,
        constraint: Box<dyn SketchConstraint>,
        dict: Value,
    ) -> u64 {
        let referenced = constraint.referenced_points();
        let index = self.constraints.len();
        self.constraints
            .push(ConstraintEntry::new(constraint, dict, referenced));
        index as u64
    }

    /// All stored constraints, in insertion order.
    pub fn all_constraints(&self) -> &[ConstraintEntry] {
        &self.constraints
    }

    /// The stored constraint at the given handle.
    pub fn get_constraint(&self, index: u64) -> Option<&dyn SketchConstraint> {
        self.constraints
            .get(index as usize)
            .map(|e| e.constraint.as_ref())
    }

    /// Number of stored constraints.
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// The stored reference dictionary of a constraint.
    pub fn constraint_dict(&self, index: u64) -> Option<Value> {
        self.constraints.get(index as usize).map(|e| e.dict.clone())
    }

    /// Removes a constraint by handle.
    pub fn remove_constraint(&mut self, index: u64) {
        if (index as usize) < self.constraints.len() {
            self.constraints.remove(index as usize);
        }
    }

    /// Residual values of all stored constraints at the current
    /// point positions, in constraint order.
    pub fn constraint_residuals(&self) -> Vec<f64> {
        let mut residuals = Vec::new();
        for entry in self.constraints.iter() {
            if entry.constraint.residual_count() == 0 {
                continue;
            }
            residuals.extend(entry.constraint.error(self));
        }
        residuals
    }

    // ── mutation ────────────────────────────────────────────────

    fn next_id(&mut self) -> u64 {
        let id = self.id_counter;
        self.id_counter += 1;
        self.entity_version += 1;
        id
    }

    fn push_entity(&mut self, entity: Box<dyn Entity>) -> u64 {
        let id = entity.base().id;
        let point_ids = entity.get_point_ids();
        for pid in point_ids {
            *self.point_usage.entry(pid).or_insert(0) += 1;
        }
        self.entity_index.insert(id, self.entities.len());
        self.entities.push(entity);
        id
    }

    /// Adds a point and returns its id.
    pub fn add_point(&mut self, x: f64, y: f64, fixed: bool) -> u64 {
        let id = self.next_id();
        let point = SketchPoint {
            id,
            x,
            y,
            fixed,
            waypoint_type: crate::sketch::entities::point::WaypointType::Sharp,
            constrained: false,
        };
        self.point_index.insert(id, self.points.len());
        self.points.push(point);
        id
    }

    /// Appends pre-built points whose ids are final.
    pub fn add_points(&mut self, points: Vec<SketchPoint>) {
        for point in points {
            self.point_index.insert(point.id, self.points.len());
            self.points.push(point);
        }
    }

    /// Removes the given points.
    pub fn remove_points(&mut self, point_ids: &[u64]) {
        let ids: std::collections::HashSet<u64> =
            point_ids.iter().copied().collect();
        if ids.is_empty() {
            return;
        }
        self.points.retain(|p| !ids.contains(&p.id));
        self.point_index.clear();
        for (i, p) in self.points.iter().enumerate() {
            self.point_index.insert(p.id, i);
        }
    }

    /// Replaces the point list.
    pub fn set_points(&mut self, points: Vec<SketchPoint>) {
        self.points.clear();
        self.point_index.clear();
        self.add_points(points);
    }

    /// Adds a line and returns its id.
    pub fn add_line(&mut self, p1: u64, p2: u64, construction: bool) -> u64 {
        let id = self.next_id();
        self.push_entity(Box::new(crate::sketch::entities::line::Line::new(
            id,
            p1,
            p2,
            construction,
        )))
    }

    /// Adds an arc and returns its id.
    pub fn add_arc(
        &mut self,
        start: u64,
        end: u64,
        center: u64,
        clockwise: bool,
        construction: bool,
    ) -> u64 {
        let id = self.next_id();
        self.push_entity(Box::new(crate::sketch::entities::arc::Arc::new(
            id,
            start,
            end,
            center,
            clockwise,
            construction,
        )))
    }

    /// Adds a circle and returns its id.
    pub fn add_circle(
        &mut self,
        center: u64,
        radius_point: u64,
        construction: bool,
    ) -> u64 {
        let id = self.next_id();
        self.push_entity(Box::new(
            crate::sketch::entities::circle::Circle::new(
                id,
                center,
                radius_point,
                construction,
            ),
        ))
    }

    /// Adds an ellipse and returns its id.
    pub fn add_ellipse(
        &mut self,
        center: u64,
        radius_x_point: u64,
        radius_y_point: u64,
        construction: bool,
    ) -> u64 {
        let id = self.next_id();
        self.push_entity(Box::new(
            crate::sketch::entities::ellipse::Ellipse::new(
                id,
                center,
                radius_x_point,
                radius_y_point,
                construction,
            ),
        ))
    }

    /// Adds a bezier and returns its id.
    pub fn add_bezier(
        &mut self,
        start: u64,
        end: u64,
        construction: bool,
        cp1: Option<[f64; 2]>,
        cp2: Option<[f64; 2]>,
    ) -> u64 {
        let id = self.next_id();
        self.push_entity(Box::new(
            crate::sketch::entities::bezier::Bezier::new(
                id,
                start,
                end,
                construction,
                cp1,
                cp2,
            ),
        ))
    }

    /// Adds a polygon and returns its id.  `vertices` is the outer
    /// ring in frame-local coordinates; `holes` are further rings.
    pub fn add_polygon(
        &mut self,
        center: u64,
        handle: u64,
        vertices: Vec<(f64, f64)>,
        closed: bool,
        construction: bool,
        holes: Vec<Vec<(f64, f64)>>,
    ) -> u64 {
        let id = self.next_id();
        self.push_entity(Box::new(
            crate::sketch::entities::polygon::PolygonEntity::new(
                id,
                center,
                handle,
                vertices,
                closed,
                construction,
                holes,
            ),
        ))
    }

    /// Adds a text box and returns its id.
    #[allow(clippy::too_many_arguments)]
    pub fn add_text_box(
        &mut self,
        origin: u64,
        width: u64,
        height: u64,
        content: String,
        font: crate::sketch::entities::entity::FontConfig,
        construction_line_ids: Vec<u64>,
    ) -> u64 {
        let id = self.next_id();
        let mut entity = crate::sketch::entities::text_box::TextBoxEntity::new(
            id, origin, width, height, content, false,
        );
        entity.font_config =
            crate::sketch::entities::text_box::core_font_from_model(&font);
        entity.construction_line_ids = construction_line_ids;
        self.push_entity(Box::new(entity))
    }

    /// Removes entities by id, updating the point usage counts.
    pub fn remove_entities_by_id(&mut self, entity_ids: &[u64]) {
        let ids: std::collections::HashSet<u64> =
            entity_ids.iter().copied().collect();
        if ids.is_empty() {
            return;
        }
        for entity in self.entities.iter() {
            if ids.contains(&entity.base().id) {
                for pid in entity.get_point_ids() {
                    let count = self.point_usage.entry(pid).or_insert(0);
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        self.point_usage.remove(&pid);
                    }
                }
            }
        }
        self.entities.retain(|e| !ids.contains(&e.base().id));
        self.entity_index.clear();
        for (i, e) in self.entities.iter().enumerate() {
            self.entity_index.insert(e.base().id, i);
        }
        self.entity_version += 1;
    }

    // ── serialization ───────────────────────────────────────────

    /// Serializes to the reference registry dictionary shape
    /// (`.ryp` compatible).
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "points": self.points.iter()
                .map(crate::sketch::entities::point::SketchPoint::to_json)
                .collect::<Vec<_>>(),
            "entities": self.entities.iter()
                .map(|e| e.to_json())
                .collect::<Vec<_>>(),
            "constraints": self.constraints.iter()
                .map(|e| e.dict.clone())
                .collect::<Vec<_>>(),
            "id_counter": self.id_counter,
        })
    }

    /// Deserializes from the reference registry dictionary shape.
    pub fn from_json(data: &Value) -> Option<EntityRegistry> {
        let mut model = EntityRegistry::new();
        let points = data.get("points")?.as_array()?;
        for point_data in points {
            let point = SketchPoint::from_json(point_data)?;
            model.point_index.insert(point.id, model.points.len());
            model.points.push(point);
        }
        let empty_entities: Vec<Value> = Vec::new();
        let entities_data = data
            .get("entities")
            .and_then(Value::as_array)
            .unwrap_or(&empty_entities);
        for entity_data in entities_data {
            if let Some(entity) = entity_from_json(entity_data) {
                let eid = entity.base().id;
                model.entity_index.insert(eid, model.entities.len());
                model.entities.push(entity);
            }
        }
        let empty_constraints: Vec<Value> = Vec::new();
        let constraints_data = data
            .get("constraints")
            .and_then(Value::as_array)
            .unwrap_or(&empty_constraints);
        for constraint_data in constraints_data {
            let _ = model.add_constraint_json(constraint_data);
        }
        model.id_counter =
            data.get("id_counter").and_then(Value::as_u64).unwrap_or(0);
        model.entity_version += 1;
        model.rebuild_usage_counts();
        Some(model)
    }

    /// Rebuilds the point usage counts by scanning all entities.
    pub fn rebuild_usage_counts(&mut self) {
        self.point_usage.clear();
        for entity in self.entities.iter() {
            for pid in entity.get_point_ids() {
                *self.point_usage.entry(pid).or_insert(0) += 1;
            }
        }
    }

    /// Re-computes every entity's constrained flag from the defining
    /// points' flags.  `constrains_radius(entity_id)` reports whether
    /// a constraint explicitly dimensions an entity's radius.
    pub fn update_constrained_status(
        &mut self,
        constrains_radius: &dyn Fn(u64) -> bool,
    ) {
        let flags: Vec<bool> = self
            .entities
            .iter()
            .map(|e| e.compute_constrained(self, constrains_radius))
            .collect();
        for (entity, flag) in self.entities.iter_mut().zip(flags) {
            entity.base_mut().constrained = flag;
        }
    }
}

impl PointContext for EntityRegistry {
    fn xy(&self, id: u64) -> (f64, f64) {
        self.point_position(id).unwrap_or((0.0, 0.0))
    }

    fn set_xy(&mut self, id: u64, x: f64, y: f64) {
        let _ = self.set_point_position(id, x, y);
    }
}

impl crate::sketch::draw::DrawContext for EntityRegistry {
    fn get_point(&self, id: u64) -> Option<crate::sketch::draw::DrawPoint> {
        self.get_point(id)
            .map(|point| crate::sketch::draw::DrawPoint {
                x: point.x,
                y: point.y,
                fixed: point.fixed,
            })
    }

    fn entity_ids(&self) -> Vec<u64> {
        EntityRegistry::entity_ids(self)
    }

    fn entity_type(&self, id: u64) -> Option<std::borrow::Cow<'static, str>> {
        EntityRegistry::entity_type(self, id).map(std::borrow::Cow::Borrowed)
    }

    fn entity_point_ids(&self, id: u64) -> Option<Vec<u64>> {
        EntityRegistry::entity_point_ids(self, id)
    }

    fn point_in_text_box_frame(&self, point_id: u64) -> bool {
        EntityRegistry::point_is_in_text_box_frame(self, point_id)
    }
}
