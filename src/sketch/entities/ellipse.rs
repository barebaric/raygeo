//! Ellipse entity, mirroring the reference `Ellipse` class.

use serde_json::{json, Value};

use crate::geo::geometry::Geometry;
use crate::geo::shape::ellipse::{
    is_point_inside_ellipse, is_point_on_ellipse,
};
use crate::geo::types::{Point, Rect};

use super::entity::{Entity, EntityBase};
use crate::sketch::registry::EntityRegistry;

/// An ellipse defined by center and the two radius points.
#[derive(Clone, Debug)]
pub struct Ellipse {
    pub base: EntityBase,
    pub center: u64,
    pub radius_x_point: u64,
    pub radius_y_point: u64,
    pub helper_line_ids: Vec<u64>,
}

impl Ellipse {
    pub fn new(
        id: u64,
        center: u64,
        radius_x_point: u64,
        radius_y_point: u64,
        construction: bool,
    ) -> Self {
        Ellipse {
            base: EntityBase {
                id,
                construction,
                invisible: false,
                array_copy: false,
                constrained: false,
                fill_color: None,
            },
            center,
            radius_x_point,
            radius_y_point,
            helper_line_ids: Vec::new(),
        }
    }

    pub fn from_json(data: &Value) -> Option<Ellipse> {
        Some(Ellipse {
            base: super::entity::base_from_json(data),
            center: data.get("center_idx")?.as_u64()?,
            radius_x_point: data.get("radius_x_pt_idx")?.as_u64()?,
            radius_y_point: data.get("radius_y_pt_idx")?.as_u64()?,
            helper_line_ids: data
                .get("helper_line_ids")
                .and_then(Value::as_array)
                .map(|array| array.iter().filter_map(Value::as_u64).collect())
                .unwrap_or_default(),
        })
    }

    pub fn to_json(&self) -> Value {
        let base = self.base.to_base_json();
        let extra = json!({
            "type": self.type_name(),
            "center_idx": self.center,
            "radius_x_pt_idx": self.radius_x_point,
            "radius_y_pt_idx": self.radius_y_point,
            "helper_line_ids": self.helper_line_ids,
        });
        EntityBase::combined_json(&base, extra)
    }

    pub fn radii(&self, model: &EntityRegistry) -> (f64, f64) {
        let (Some((cx, cy)), Some((x1, y1)), Some((x2, y2))) = (
            model.point_position(self.center),
            model.point_position(self.radius_x_point),
            model.point_position(self.radius_y_point),
        ) else {
            return (0.0, 0.0);
        };
        ((x1 - cx).hypot(y1 - cy), (x2 - cx).hypot(y2 - cy))
    }

    pub fn rotation(&self, model: &EntityRegistry) -> f64 {
        let Some((cx, cy)) = model.point_position(self.center) else {
            return 0.0;
        };
        let Some((px, py)) = model.point_position(self.radius_x_point) else {
            return 0.0;
        };
        (py - cy).atan2(px - cx)
    }

    fn frame_circle_geometry(
        cx: f64,
        cy: f64,
        r: f64,
        cos_a: f64,
        sin_a: f64,
    ) -> Geometry {
        let mut geo = Geometry::new();
        let start = (cx + r * cos_a, cy + r * sin_a);
        let mid = (cx - r * cos_a, cy - r * sin_a);
        geo.move_to(start.0, start.1, 0.0);
        geo.arc_to(mid.0, mid.1, -r * cos_a, -r * sin_a, false, 0.0);
        geo.arc_to(start.0, start.1, r * cos_a, r * sin_a, false, 0.0);
        geo
    }
}

impl Entity for Ellipse {
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
        "ellipse"
    }

    fn to_json(&self) -> Value {
        Ellipse::to_json(self)
    }

    fn get_point_ids(&self) -> Vec<u64> {
        vec![self.center, self.radius_x_point, self.radius_y_point]
    }

    fn get_helper_ids(&self) -> Vec<u64> {
        self.helper_line_ids.clone()
    }

    fn clear_helper_references(&mut self) {
        self.helper_line_ids = Vec::new();
    }

    fn is_closed_loop(&self) -> bool {
        true
    }

    fn characteristic_length_pairs(&self) -> Vec<[u64; 2]> {
        vec![
            [self.center, self.radius_x_point],
            [self.center, self.radius_y_point],
        ]
    }

    fn characteristic_length(&self, model: &EntityRegistry) -> f64 {
        let (rx, ry) = self.radii(model);
        (rx + ry) / 2.0
    }

    fn tangent_at(
        &self,
        _model: &EntityRegistry,
        _point_id: u64,
    ) -> Option<(f64, f64)> {
        // No well-defined endpoints for edge traversal.
        Some((1.0, 0.0))
    }

    fn enclosed_signed_area(&self, model: &EntityRegistry) -> Option<f64> {
        let (rx, ry) = self.radii(model);
        Some(std::f64::consts::PI * rx * ry)
    }

    fn contains_point(&self, model: &EntityRegistry, x: f64, y: f64) -> bool {
        let Some((cx, cy)) = model.point_position(self.center) else {
            return false;
        };
        let (rx, ry) = self.radii(model);
        if rx <= 1e-9 || ry <= 1e-9 {
            return false;
        }
        let rotation = self.rotation(model);
        is_point_inside_ellipse(
            Point::new(x, y),
            Point::new(cx, cy),
            rx,
            ry,
            rotation,
        )
    }

    fn get_junction_point_ids(&self) -> Vec<u64> {
        vec![self.center, self.radius_x_point, self.radius_y_point]
    }

    fn get_rigidly_connected_points(&self, point_id: u64) -> Vec<u64> {
        if point_id == self.center {
            return vec![self.center, self.radius_x_point, self.radius_y_point];
        }
        Vec::new()
    }

    fn get_drag_anchor_points(&self, point_id: u64) -> Vec<u64> {
        if point_id == self.radius_x_point || point_id == self.radius_y_point {
            return vec![self.center];
        }
        Vec::new()
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
        let (rx, ry) = self.radii(model);
        if rx < 1e-9 || ry < 1e-9 {
            return false;
        }
        let rotation = self.rotation(model);
        is_point_on_ellipse(
            Point::new(mx, my),
            Point::new(cx, cy),
            rx,
            ry,
            rotation,
            threshold,
        )
    }

    fn get_ignorable_unconstrained_points(&self) -> Vec<u64> {
        if self.base.constrained {
            return vec![self.radius_x_point, self.radius_y_point];
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
            .get_point(self.radius_x_point)
            .map(|p| p.constrained)
            .unwrap_or(false)
            && model
                .get_point(self.radius_y_point)
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
        let (rx, ry) = self.radii(model);
        (cx - rx) >= rect.min.x
            && (cy - ry) >= rect.min.y
            && (cx + rx) <= rect.max.x
            && (cy + ry) <= rect.max.y
    }

    fn intersects_rect(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let Some((cx, cy)) = model.point_position(self.center) else {
            return false;
        };
        let (rx, ry) = self.radii(model);

        if self.is_contained_by(rect, model) {
            return true;
        }

        let closest_x = rect.min.x.max(cx.min(rect.max.x));
        let closest_y = rect.min.y.max(cy.min(rect.max.y));
        let dx = closest_x - cx;
        let dy = closest_y - cy;
        let dist_sq = if rx > 0.0 && ry > 0.0 {
            (dx / rx).powi(2) + (dy / ry).powi(2)
        } else {
            f64::INFINITY
        };
        if dist_sq > 1.0 {
            return false;
        }

        let dx_far = (rect.min.x - cx).abs().max((rect.max.x - cx).abs());
        let dy_far = (rect.min.y - cy).abs().max((rect.max.y - cy).abs());
        let dist_sq_far = if rx > 0.0 && ry > 0.0 {
            (dx_far / rx).powi(2) + (dy_far / ry).powi(2)
        } else {
            f64::INFINITY
        };
        dist_sq_far >= 1.0
    }

    fn to_geometry(&self, model: &EntityRegistry) -> Geometry {
        let mut geo = Geometry::new();
        let Some((cx, cy)) = model.point_position(self.center) else {
            return geo;
        };
        let (rx, ry) = self.radii(model);
        if rx < 1e-9 || ry < 1e-9 {
            return geo;
        }
        let rotation = self.rotation(model);
        let (cos_a, sin_a) = (rotation.cos(), rotation.sin());

        // Near-circular: emit two exact half arcs like the reference.
        if (rx - ry).abs() < 1e-9 {
            return Self::frame_circle_geometry(cx, cy, rx, cos_a, sin_a);
        }

        // Eccentricity-driven sampling, capped to keep tessellation
        // bounded even for extreme aspect ratios.
        let ratio = rx.max(ry) / rx.min(ry);
        let num_segments = ((64.0 * ratio) as usize).clamp(32, 256);
        for i in 0..num_segments {
            let angle =
                2.0 * std::f64::consts::PI * i as f64 / num_segments as f64;
            let local_x = rx * angle.cos();
            let local_y = ry * angle.sin();
            let x = cx + local_x * cos_a - local_y * sin_a;
            let y = cy + local_x * sin_a + local_y * cos_a;
            if i == 0 {
                geo.move_to(x, y, 0.0);
            } else {
                geo.line_to(x, y, 0.0);
            }
        }
        geo.close_path();
        let fitted = crate::geo::algo::fitting::fit_arcs(geo.data(), 0.1);
        geo.data = fitted;
        geo
    }

    fn create_fill_geometry(&self, model: &EntityRegistry) -> Option<Geometry> {
        Some(self.to_geometry(model))
    }
}
