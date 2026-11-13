//! Entities of the sketch model.
//!
//! [`Entity`] is the trait mirroring the reference Python base
//! class: every polymorphic method of the old entities exists here
//! with the same semantics, one implementation per entity file
//! (`line.rs`, `arc.rs`, ...).  Methods that the Python signatures
//! supplied through the registry (point lookups) take the model as
//! a parameter; methods of the offset/placement command layer are
//! not part of the model trait.

use serde_json::{json, Value};

use crate::sketch::registry::EntityRegistry;

/// Rounds to 6 decimals, matching the reference `quantize` used in
/// geometry signatures.
pub fn quantize(value: f64) -> f64 {
    (value * 1.0e6).round() / 1.0e6
}

/// RGBA color as `[r, g, b, a]`.
pub type ColorRgba = [f64; 4];

/// Font configuration of a text box, mirroring the reference
/// serialization keys (`font_family`, `font_size`, `bold`,
/// `italic`).  Unknown keys of stored documents are preserved
/// verbatim in `extra`.
#[derive(Clone, Debug)]
pub struct FontConfig {
    pub family: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub extra: serde_json::Map<String, Value>,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig {
            family: "sans-serif".to_string(),
            size: 10.0,
            bold: false,
            italic: false,
            extra: serde_json::Map::new(),
        }
    }
}

/// Common fields of every entity, mirroring the reference base
/// class attributes.
#[derive(Clone, Debug)]
pub struct EntityBase {
    pub id: u64,
    pub construction: bool,
    pub invisible: bool,
    pub array_copy: bool,
    pub constrained: bool,
    pub fill_color: Option<ColorRgba>,
}

impl EntityBase {
    /// Base serialization fields (`.ryp` compatible): `invisible`
    /// and `array_copy` only emitted when true.
    pub fn to_base_json(&self) -> Value {
        let mut data = json!({
            "id": self.id,
            "construction": self.construction,
        });
        if self.invisible {
            data["invisible"] = json!(true);
        }
        if self.array_copy {
            data["array_copy"] = json!(true);
        }
        if let Some(color) = self.fill_color {
            data["fill_color"] = json!(color);
        }
        data
    }

    /// Restores the base flags that are not constructor arguments.
    pub fn restore_base_flags(&mut self, data: &Value) {
        self.invisible = data
            .get("invisible")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.array_copy = data
            .get("array_copy")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    }

    fn merge_extra(data: &mut Value, extra: Value) {
        if let (Some(map), Some(extra_map)) =
            (data.as_object_mut(), extra.as_object())
        {
            for (key, value) in extra_map {
                map.insert(key.clone(), value.clone());
            }
        }
    }

    /// Helper for kind payloads: merges the kind keys into the base
    /// dict (used by the per-entity `to_json` implementations).
    pub(crate) fn combined_json(base: &Value, extra: Value) -> Value {
        let mut data = base.clone();
        Self::merge_extra(&mut data, extra);
        data
    }
}

/// The polymorphic entity interface.  Default bodies mirror the
/// reference base class defaults; per-entity files override exactly
/// where the Python classes override.
pub trait Entity: Send + Sync {
    fn base(&self) -> &EntityBase;
    fn base_mut(&mut self) -> &mut EntityBase;

    /// Downcast hook for the binding layer.
    fn as_any(&self) -> &dyn std::any::Any;

    /// The reference serialization type string.
    fn type_name(&self) -> &'static str;

    /// Serialization to the reference dictionary shape.
    fn to_json(&self) -> Value;

    // ── state snapshots (undo) ─────────────────────────────────

    /// Solver-relevant discrete state; `None` when the entity has
    /// none.
    fn get_state(&self) -> Option<Value> {
        Some(json!({
            "construction": self.base().construction,
        }))
    }

    fn set_state(&mut self, state: &Value) {
        if let Some(v) = state.get("construction").and_then(Value::as_bool) {
            self.base_mut().construction = v;
        }
    }

    // ── structural queries ─────────────────────────────────────

    fn get_point_ids(&self) -> Vec<u64> {
        Vec::new()
    }

    /// Hashable signature of all geometry affecting the entity's
    /// shape: quantized defining-point positions, extended by
    /// entity-internal state.
    fn geometry_signature(&self, model: &EntityRegistry) -> Value {
        points_signature(model, &self.get_point_ids())
    }

    fn get_endpoint_ids(&self) -> Vec<u64> {
        Vec::new()
    }

    /// Returns all frame point ids of a text box frame; other entity
    /// kinds return none.
    fn get_all_frame_point_ids(&self, _model: &EntityRegistry) -> Vec<u64> {
        Vec::new()
    }

    fn get_ignorable_unconstrained_points(&self) -> Vec<u64> {
        Vec::new()
    }

    fn get_junction_point_ids(&self) -> Vec<u64> {
        Vec::new()
    }

    fn get_helper_ids(&self) -> Vec<u64> {
        Vec::new()
    }

    fn clear_helper_references(&mut self) {}

    /// Points that move together with `point_id` as a rigid body
    /// during dragging.
    fn get_rigidly_connected_points(&self, _point_id: u64) -> Vec<u64> {
        Vec::new()
    }

    /// Points pinned while `point_id` is dragged.
    fn get_drag_anchor_points(&self, _point_id: u64) -> Vec<u64> {
        Vec::new()
    }

    // ── classification ─────────────────────────────────────────

    fn is_edge_entity(&self) -> bool {
        false
    }

    fn is_radius_entity(&self) -> bool {
        false
    }

    fn is_closed_loop(&self) -> bool {
        false
    }

    /// Signed area enclosed by this entity when it alone forms a
    /// closed loop; `None` when unsupported (the reference raises
    /// `NotImplementedError`).
    fn enclosed_signed_area(&self, _model: &EntityRegistry) -> Option<f64> {
        None
    }

    fn supports_fill(&self) -> bool {
        false
    }

    fn supports_point_on_curve(&self) -> bool {
        false
    }

    // ── characteristic lengths (equal-length constraints) ──────

    fn characteristic_length_pairs(&self) -> Vec<[u64; 2]> {
        Vec::new()
    }

    fn characteristic_length(&self, model: &EntityRegistry) -> f64 {
        let pairs = self.characteristic_length_pairs();
        let Some(pair) = pairs.first() else {
            return 0.0;
        };
        match (model.point_position(pair[0]), model.point_position(pair[1])) {
            (Some((x1, y1)), Some((x2, y2))) => (x2 - x1).hypot(y2 - y1),
            _ => 0.0,
        }
    }

    // ── geometry queries ───────────────────────────────────────

    fn hit_test(
        &self,
        _mx: f64,
        _my: f64,
        _threshold: f64,
        _model: &EntityRegistry,
    ) -> bool {
        false
    }

    fn is_contained_by(
        &self,
        _rect: crate::geo::types::Rect,
        _model: &EntityRegistry,
    ) -> bool {
        false
    }

    fn intersects_rect(
        &self,
        _rect: crate::geo::types::Rect,
        _model: &EntityRegistry,
    ) -> bool {
        false
    }

    /// Signed distance from a point to the entity's locus, or
    /// `None` when the entity cannot compute one (the reference
    /// raises `NotImplementedError`).
    fn signed_distance_to(
        &self,
        _px: f64,
        _py: f64,
        _model: &EntityRegistry,
    ) -> Option<f64> {
        None
    }

    /// Tangent vector at a defining point, pointing away from it
    /// along the entity's curve; `None` when the entity has no
    /// directional tangent (the reference raises
    /// `NotImplementedError`).
    fn tangent_at(
        &self,
        _model: &EntityRegistry,
        _point_id: u64,
    ) -> Option<(f64, f64)> {
        None
    }

    fn contains_point(
        &self,
        _model: &EntityRegistry,
        _x: f64,
        _y: f64,
    ) -> bool {
        false
    }

    // ── geometry emission ──────────────────────────────────────

    fn to_geometry(
        &self,
        _model: &EntityRegistry,
    ) -> crate::geo::geometry::Geometry {
        crate::geo::geometry::Geometry::new()
    }

    fn create_fill_geometry(
        &self,
        _model: &EntityRegistry,
    ) -> Option<crate::geo::geometry::Geometry> {
        None
    }

    fn append_to_geometry(
        &self,
        _geo: &mut crate::geo::geometry::Geometry,
        _model: &EntityRegistry,
        _forward: bool,
    ) {
    }

    fn to_polygon_vertices(
        &self,
        _model: &EntityRegistry,
        _forward: bool,
    ) -> Vec<(f64, f64)> {
        Vec::new()
    }

    fn to_polyline(
        &self,
        model: &EntityRegistry,
        tolerance: f64,
    ) -> Vec<(f64, f64)> {
        match self.to_geometry(model).to_polygons(tolerance).first() {
            Some(polygon) => polygon.iter().map(|p| (p.x, p.y)).collect(),
            None => Vec::new(),
        }
    }

    fn to_polylines(
        &self,
        model: &EntityRegistry,
        tolerance: f64,
    ) -> Vec<Vec<(f64, f64)>> {
        vec![self.to_polyline(model, tolerance)]
    }

    // ── fill color ─────────────────────────────────────────────

    /// Sets the fill color.
    fn set_fill_color(&mut self, color: Option<ColorRgba>) -> bool {
        self.base_mut().fill_color = color;
        true
    }

    // ── solver status ──────────────────────────────────────────

    /// Re-computes the fully-constrained flag from the defining
    /// points' constrained flags.  `constrains_radius(entity_id)`
    /// reports whether a constraint explicitly defines the radius of
    /// an entity (radius/diameter constraints).
    fn compute_constrained(
        &self,
        _model: &EntityRegistry,
        _constrains_radius: &dyn Fn(u64) -> bool,
    ) -> bool {
        false
    }

    // ── transforms ─────────────────────────────────────────────

    /// Updates entity-internal state for a mirror transform (arc
    /// chirality, bezier control offsets, polygon ring winding).
    /// Point positions are mirrored centrally by the caller.
    fn mirror(&mut self, _axis: MirrorAxis) {}
}

use super::mirror::MirrorAxis;

/// Deserializes an entity from a reference dictionary, dispatching
/// on the `type` key.
pub fn entity_from_json(data: &Value) -> Option<Box<dyn Entity>> {
    let type_name = data.get("type").and_then(Value::as_str)?;
    match type_name {
        "line" => Some(Box::new(super::line::Line::from_json(data)?)),
        "arc" => Some(Box::new(super::arc::Arc::from_json(data)?)),
        "circle" => Some(Box::new(super::circle::Circle::from_json(data)?)),
        "ellipse" => Some(Box::new(super::ellipse::Ellipse::from_json(data)?)),
        "bezier" => Some(Box::new(super::bezier::Bezier::from_json(data)?)),
        "polygon" => {
            Some(Box::new(super::polygon::PolygonEntity::from_json(data)?))
        }
        "text_box" => {
            Some(Box::new(super::text_box::TextBoxEntity::from_json(data)?))
        }
        _ => None,
    }
}

/// Reads the common base fields from a reference dictionary.
pub(crate) fn base_from_json(data: &Value) -> EntityBase {
    EntityBase {
        id: data.get("id").and_then(Value::as_u64).unwrap_or(0),
        construction: data
            .get("construction")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        invisible: data
            .get("invisible")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        array_copy: data
            .get("array_copy")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        constrained: false,
        fill_color: data.get("fill_color").and_then(|value| {
            let array = value.as_array()?;
            Some([
                array.first()?.as_f64()?,
                array.get(1)?.as_f64()?,
                array.get(2)?.as_f64()?,
                array.get(3)?.as_f64()?,
            ])
        }),
    }
}

/// Quantized defining-point positions as a JSON array — the shared
/// prefix of every entity's geometry signature.
pub(crate) fn points_signature(
    model: &EntityRegistry,
    point_ids: &[u64],
) -> Value {
    Value::Array(
        point_ids
            .iter()
            .filter_map(|id| model.point_position(*id))
            .map(|(x, y)| json!([quantize(x), quantize(y)]))
            .collect(),
    )
}
