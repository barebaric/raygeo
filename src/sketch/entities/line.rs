//! Line entity, mirroring the reference `Line` class.

use serde_json::{json, Value};

use crate::geo::geometry::Geometry;
use crate::geo::shape::line::{
    does_line_segment_intersect_rect, get_line_segment_closest_point,
};
use crate::geo::types::{Point, Rect};

use super::entity::{base_from_json, Entity, EntityBase};
use crate::sketch::registry::EntityRegistry;

/// A straight segment between two points.
#[derive(Clone, Debug)]
pub struct Line {
    pub base: EntityBase,
    pub p1: u64,
    pub p2: u64,
}

impl Line {
    pub fn new(id: u64, p1: u64, p2: u64, construction: bool) -> Self {
        Line {
            base: EntityBase {
                id,
                construction,
                invisible: false,
                array_copy: false,
                constrained: false,
                fill_color: None,
            },
            p1,
            p2,
        }
    }

    pub fn from_json(data: &Value) -> Option<Line> {
        Some(Line {
            base: base_from_json(data),
            p1: data.get("p1_idx")?.as_u64()?,
            p2: data.get("p2_idx")?.as_u64()?,
        })
    }

    pub fn to_json(&self) -> Value {
        let base = self.base.to_base_json();
        let extra = json!({
            "type": self.type_name(),
            "p1_idx": self.p1,
            "p2_idx": self.p2,
        });
        EntityBase::combined_json(&base, extra)
    }
}

impl Entity for Line {
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
        "line"
    }

    fn to_json(&self) -> Value {
        Line::to_json(self)
    }

    fn get_point_ids(&self) -> Vec<u64> {
        vec![self.p1, self.p2]
    }

    fn get_endpoint_ids(&self) -> Vec<u64> {
        vec![self.p1, self.p2]
    }

    fn get_junction_point_ids(&self) -> Vec<u64> {
        vec![self.p1, self.p2]
    }

    fn is_edge_entity(&self) -> bool {
        true
    }

    fn characteristic_length_pairs(&self) -> Vec<[u64; 2]> {
        vec![[self.p1, self.p2]]
    }

    fn tangent_at(
        &self,
        model: &EntityRegistry,
        point_id: u64,
    ) -> Option<(f64, f64)> {
        let p1 = model.point_position(self.p1)?;
        let p2 = model.point_position(self.p2)?;
        let (dx, dy) = (p2.0 - p1.0, p2.1 - p1.1);
        Some(if point_id == self.p1 {
            (dx, dy)
        } else {
            (-dx, -dy)
        })
    }

    fn signed_distance_to(
        &self,
        px: f64,
        py: f64,
        model: &EntityRegistry,
    ) -> Option<f64> {
        let p1 = model.point_position(self.p1)?;
        let p2 = model.point_position(self.p2)?;
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let length = dx.hypot(dy);
        if length < 1e-9 {
            return Some((px - p1.0).hypot(py - p1.1));
        }
        let cross = dx * (py - p1.1) - (px - p1.0) * dy;
        Some(cross / length)
    }

    fn hit_test(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
    ) -> bool {
        let (Some(p1), Some(p2)) =
            (model.point_position(self.p1), model.point_position(self.p2))
        else {
            return false;
        };
        let (_, _, dist_sq) = get_line_segment_closest_point(
            Point::new(p1.0, p1.1),
            Point::new(p2.0, p2.1),
            mx,
            my,
        );
        dist_sq < threshold * threshold
    }

    fn compute_constrained(
        &self,
        model: &EntityRegistry,
        _constrains_radius: &dyn Fn(u64) -> bool,
    ) -> bool {
        let c1 = model.get_point(self.p1).map(|p| p.constrained);
        let c2 = model.get_point(self.p2).map(|p| p.constrained);
        c1.unwrap_or(false) && c2.unwrap_or(false)
    }

    fn is_contained_by(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let (Some(p1), Some(p2)) =
            (model.point_position(self.p1), model.point_position(self.p2))
        else {
            return false;
        };
        let geo_rect = rect;
        crate::geo::shape::line::is_point_inside_rect(
            Point::new(p1.0, p1.1),
            geo_rect,
        ) && crate::geo::shape::line::is_point_inside_rect(
            Point::new(p2.0, p2.1),
            geo_rect,
        )
    }

    fn intersects_rect(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let (Some(p1), Some(p2)) =
            (model.point_position(self.p1), model.point_position(self.p2))
        else {
            return false;
        };
        does_line_segment_intersect_rect(
            Point::new(p1.0, p1.1),
            Point::new(p2.0, p2.1),
            rect,
        )
    }

    fn to_geometry(&self, model: &EntityRegistry) -> Geometry {
        let mut geo = Geometry::new();
        let (Some(p1), Some(p2)) =
            (model.point_position(self.p1), model.point_position(self.p2))
        else {
            return geo;
        };
        geo.move_to(p1.0, p1.1, 0.0);
        geo.line_to(p2.0, p2.1, 0.0);
        geo
    }

    fn append_to_geometry(
        &self,
        geo: &mut Geometry,
        model: &EntityRegistry,
        forward: bool,
    ) {
        let end = if forward { self.p2 } else { self.p1 };
        if let Some((x, y)) = model.point_position(end) {
            geo.line_to(x, y, 0.0);
        }
    }

    fn to_polygon_vertices(
        &self,
        model: &EntityRegistry,
        forward: bool,
    ) -> Vec<(f64, f64)> {
        let start = if forward { self.p1 } else { self.p2 };
        model.point_position(start).into_iter().collect()
    }

    fn to_polyline(
        &self,
        model: &EntityRegistry,
        _tolerance: f64,
    ) -> Vec<(f64, f64)> {
        // Overridden because raygeo's polygon conversion skips lone
        // segments.
        let (Some(p1), Some(p2)) =
            (model.point_position(self.p1), model.point_position(self.p2))
        else {
            return Vec::new();
        };
        vec![p1, p2]
    }
}
