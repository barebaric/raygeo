//! Circle entity, mirroring the reference `Circle` class.

use serde_json::{json, Value};

use crate::geo::geometry::Geometry;
use crate::geo::shape::circle::{
    does_circle_intersect_rect, get_circle_signed_distance,
    is_circle_inside_rect,
};
use crate::geo::types::{Point, Rect};

use super::entity::{Entity, EntityBase};
use crate::sketch::registry::EntityRegistry;

/// A full circle defined by center and radius point.
#[derive(Clone, Debug)]
pub struct Circle {
    pub base: EntityBase,
    pub center: u64,
    pub radius_point: u64,
}

impl Circle {
    pub fn new(
        id: u64,
        center: u64,
        radius_point: u64,
        construction: bool,
    ) -> Self {
        Circle {
            base: EntityBase {
                id,
                construction,
                invisible: false,
                array_copy: false,
                constrained: false,
                fill_color: None,
            },
            center,
            radius_point,
        }
    }

    pub fn from_json(data: &Value) -> Option<Circle> {
        Some(Circle {
            base: super::entity::base_from_json(data),
            center: data.get("center_idx")?.as_u64()?,
            radius_point: data.get("radius_pt_idx")?.as_u64()?,
        })
    }

    pub fn to_json(&self) -> Value {
        let base = self.base.to_base_json();
        let extra = json!({
            "type": self.type_name(),
            "center_idx": self.center,
            "radius_pt_idx": self.radius_point,
        });
        EntityBase::combined_json(&base, extra)
    }

    pub fn radius(&self, model: &EntityRegistry) -> f64 {
        let (Some((cx, cy)), Some((px, py))) = (
            model.point_position(self.center),
            model.point_position(self.radius_point),
        ) else {
            return 0.0;
        };
        (px - cx).hypot(py - cy)
    }
}

impl Entity for Circle {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut EntityBase {
        &mut self.base
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn type_name(&self) -> &'static str {
        "circle"
    }

    fn to_json(&self) -> Value {
        Circle::to_json(self)
    }

    fn get_point_ids(&self) -> Vec<u64> {
        vec![self.center, self.radius_point]
    }

    fn characteristic_length_pairs(&self) -> Vec<[u64; 2]> {
        vec![[self.center, self.radius_point]]
    }

    fn is_radius_entity(&self) -> bool {
        true
    }

    fn is_closed_loop(&self) -> bool {
        true
    }

    fn enclosed_signed_area(&self, model: &EntityRegistry) -> Option<f64> {
        // By convention, a single circle loop is CCW -> positive area.
        let radius = self.radius(model);
        Some(std::f64::consts::PI * radius * radius)
    }

    fn contains_point(&self, model: &EntityRegistry, x: f64, y: f64) -> bool {
        let (Some((cx, cy)), Some((px, py))) = (
            model.point_position(self.center),
            model.point_position(self.radius_point),
        ) else {
            return false;
        };
        let radius = (px - cx).hypot(py - cy);
        let dist_sq = (x - cx) * (x - cx) + (y - cy) * (y - cy);
        dist_sq <= radius * radius
    }

    fn hit_test(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
    ) -> bool {
        let Some((cx, cy)) = model.point_position(self.center) else {
            return false;
        };
        let radius = self.radius(model);
        if radius == 0.0 {
            return false;
        }
        let signed = get_circle_signed_distance(
            Point::new(mx, my),
            Point::new(cx, cy),
            radius,
        );
        signed.abs() < threshold
    }

    fn get_junction_point_ids(&self) -> Vec<u64> {
        vec![self.center, self.radius_point]
    }

    fn get_drag_anchor_points(&self, point_id: u64) -> Vec<u64> {
        if point_id == self.radius_point {
            return vec![self.center];
        }
        Vec::new()
    }

    fn get_ignorable_unconstrained_points(&self) -> Vec<u64> {
        // If the circle is geometrically constrained, the radius point
        // only acts as a handle for the radius value.
        if self.base.constrained {
            return vec![self.radius_point];
        }
        Vec::new()
    }

    fn compute_constrained(
        &self,
        model: &EntityRegistry,
        constrains_radius: &dyn Fn(u64) -> bool,
    ) -> bool {
        let center_constrained = model
            .get_point(self.center)
            .map(|p| p.constrained)
            .unwrap_or(false);
        let mut radii_defined = model
            .get_point(self.radius_point)
            .map(|p| p.constrained)
            .unwrap_or(false);
        if !radii_defined && constrains_radius(self.base.id) {
            radii_defined = true;
        }
        center_constrained && radii_defined
    }

    fn is_contained_by(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let Some((cx, cy)) = model.point_position(self.center) else {
            return false;
        };
        let radius = self.radius(model);
        is_circle_inside_rect(Point::new(cx, cy), radius, rect)
    }

    fn intersects_rect(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let Some((cx, cy)) = model.point_position(self.center) else {
            return false;
        };
        let radius = self.radius(model);
        // A circle that is contained by a rect also intersects it.
        // The primitive for intersection seems to miss this case, so
        // containment is checked first.
        if is_circle_inside_rect(Point::new(cx, cy), radius, rect) {
            return true;
        }
        does_circle_intersect_rect(Point::new(cx, cy), radius, rect)
    }

    fn to_geometry(&self, model: &EntityRegistry) -> Geometry {
        let mut geo = Geometry::new();
        let (Some((cx, cy)), Some((px, py))) = (
            model.point_position(self.center),
            model.point_position(self.radius_point),
        ) else {
            return geo;
        };
        let (dx, dy) = (px - cx, py - cy);
        geo.move_to(px, py, 0.0);
        geo.arc_to(cx - dx, cy - dy, -dx, -dy, false, 0.0);
        geo.arc_to(px, py, dx, dy, false, 0.0);
        geo
    }

    fn signed_distance_to(
        &self,
        px: f64,
        py: f64,
        model: &EntityRegistry,
    ) -> Option<f64> {
        let (cx, cy) = model.point_position(self.center)?;
        let radius = self.radius(model);
        Some(get_circle_signed_distance(
            Point::new(px, py),
            Point::new(cx, cy),
            radius,
        ))
    }
}
