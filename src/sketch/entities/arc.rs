//! Arc entity, mirroring the reference `Arc` class.

use serde_json::{json, Value};

use crate::geo::geometry::Geometry;
use crate::geo::shape::arc::{
    does_arc_intersect_rect, get_arc_bounds, get_arc_midpoint, is_angle_between,
};
use crate::geo::shape::circle::get_circle_signed_distance;
use crate::geo::shape::rect::does_rect_contain_rect;
use crate::geo::types::{Point, Rect};

use super::entity::{base_from_json, Entity, EntityBase};
use crate::sketch::registry::EntityRegistry;

/// A circular arc from start to end around a center.
#[derive(Clone, Debug)]
pub struct Arc {
    pub base: EntityBase,
    pub start: u64,
    pub end: u64,
    pub center: u64,
    pub clockwise: bool,
}

/// The three defining point positions of an arc: start, end, center.
type ArcPositions = ((f64, f64), (f64, f64), (f64, f64));

impl Arc {
    pub fn new(
        id: u64,
        start: u64,
        end: u64,
        center: u64,
        clockwise: bool,
        construction: bool,
    ) -> Self {
        Arc {
            base: EntityBase {
                id,
                construction,
                invisible: false,
                array_copy: false,
                constrained: false,
            },
            start,
            end,
            center,
            clockwise,
        }
    }

    pub fn from_json(data: &Value) -> Option<Arc> {
        Some(Arc {
            base: base_from_json(data),
            start: data.get("start_idx")?.as_u64()?,
            end: data.get("end_idx")?.as_u64()?,
            center: data.get("center_idx")?.as_u64()?,
            clockwise: data
                .get("clockwise")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }

    pub fn to_json(&self) -> Value {
        let base = self.base.to_base_json();
        let extra = json!({
            "type": self.type_name(),
            "start_idx": self.start,
            "end_idx": self.end,
            "center_idx": self.center,
            "clockwise": self.clockwise,
        });
        EntityBase::combined_json(&base, extra)
    }

    /// Radius from the defining points.
    pub fn radius(&self, model: &EntityRegistry) -> f64 {
        let (Some((cx, cy)), Some((sx, sy))) = (
            model.point_position(self.center),
            model.point_position(self.start),
        ) else {
            return 0.0;
        };
        (sx - cx).hypot(sy - cy)
    }

    fn positions(&self, model: &EntityRegistry) -> Option<ArcPositions> {
        Some((
            model.point_position(self.start)?,
            model.point_position(self.end)?,
            model.point_position(self.center)?,
        ))
    }

    /// Whether `angle` lies within the arc's sweep.
    pub fn is_angle_within_sweep(
        &self,
        angle: f64,
        model: &EntityRegistry,
    ) -> bool {
        let Some(((sx, sy), (ex, ey), (cx, cy))) = self.positions(model) else {
            return false;
        };
        let start_angle = (sy - cy).atan2(sx - cx);
        let end_angle = (ey - cy).atan2(ex - cx);
        is_angle_between(angle, start_angle, end_angle, self.clockwise)
    }

    fn bbox(&self, model: &EntityRegistry) -> Option<Rect> {
        let ((sx, sy), (ex, ey), (cx, cy)) = self.positions(model)?;
        let bounds = get_arc_bounds(
            Point::new(sx, sy),
            Point::new(ex, ey),
            Point::new(cx - sx, cy - sy),
            self.clockwise,
        );
        Some(bounds)
    }
}

impl Entity for Arc {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut EntityBase {
        &mut self.base
    }

    fn type_name(&self) -> &'static str {
        "arc"
    }

    fn to_json(&self) -> Value {
        Arc::to_json(self)
    }

    fn get_state(&self) -> Option<Value> {
        Some(json!({
            "construction": self.base.construction,
            "clockwise": self.clockwise,
        }))
    }

    fn set_state(&mut self, state: &Value) {
        if let Some(v) = state.get("construction").and_then(Value::as_bool) {
            self.base.construction = v;
        }
        if let Some(v) = state.get("clockwise").and_then(Value::as_bool) {
            self.clockwise = v;
        }
    }

    fn mirror(&mut self, _axis: super::mirror::MirrorAxis) {
        // Reflection reverses chirality; point positions are
        // mirrored centrally by the command.
        self.clockwise = !self.clockwise;
    }

    fn geometry_signature(&self, model: &EntityRegistry) -> Value {
        let mut signature =
            match super::entity::points_signature(model, &self.get_point_ids())
            {
                Value::Array(array) => array,
                _ => Vec::new(),
            };
        signature.push(json!(self.clockwise));
        Value::Array(signature)
    }

    fn get_point_ids(&self) -> Vec<u64> {
        vec![self.start, self.end, self.center]
    }

    fn get_endpoint_ids(&self) -> Vec<u64> {
        vec![self.start, self.end]
    }

    fn get_junction_point_ids(&self) -> Vec<u64> {
        vec![self.start, self.end, self.center]
    }

    fn is_edge_entity(&self) -> bool {
        true
    }

    fn is_radius_entity(&self) -> bool {
        true
    }

    fn characteristic_length_pairs(&self) -> Vec<[u64; 2]> {
        vec![[self.center, self.start]]
    }

    fn tangent_at(
        &self,
        model: &EntityRegistry,
        point_id: u64,
    ) -> Option<(f64, f64)> {
        let ((sx, sy), (ex, ey), (cx, cy)) = self.positions(model)?;
        if point_id == self.start {
            let (dx, dy) = (sx - cx, sy - cy);
            return Some(if self.clockwise { (dy, -dx) } else { (-dy, dx) });
        }
        let (dx, dy) = (ex - cx, ey - cy);
        Some(if self.clockwise { (-dy, dx) } else { (dy, -dx) })
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

    fn get_drag_anchor_points(&self, point_id: u64) -> Vec<u64> {
        if point_id == self.start || point_id == self.end {
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
        let radius = self.radius(model);
        if radius == 0.0 {
            return false;
        }
        let dist_mouse = (mx - cx).hypot(my - cy);
        if (dist_mouse - radius).abs() >= threshold {
            return false;
        }
        self.is_angle_within_sweep((my - cy).atan2(mx - cx), model)
    }

    fn compute_constrained(
        &self,
        model: &EntityRegistry,
        _constrains_radius: &dyn Fn(u64) -> bool,
    ) -> bool {
        let s = model.get_point(self.start).map(|p| p.constrained);
        let e = model.get_point(self.end).map(|p| p.constrained);
        let c = model.get_point(self.center).map(|p| p.constrained);
        s.unwrap_or(false) && e.unwrap_or(false) && c.unwrap_or(false)
    }

    fn is_contained_by(&self, rect: Rect, model: &EntityRegistry) -> bool {
        // Strictly inside requires the entire bounding box inside.
        match self.bbox(model) {
            Some(arc_box) => does_rect_contain_rect(rect, arc_box),
            None => false,
        }
    }

    fn intersects_rect(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let Some(((sx, sy), (ex, ey), (cx, cy))) = self.positions(model) else {
            return false;
        };
        does_arc_intersect_rect(
            Point::new(sx, sy),
            Point::new(ex, ey),
            Point::new(cx, cy),
            self.clockwise,
            rect,
        )
    }

    fn to_geometry(&self, model: &EntityRegistry) -> Geometry {
        let mut geo = Geometry::new();
        let Some(((sx, sy), (ex, ey), (cx, cy))) = self.positions(model) else {
            return geo;
        };
        geo.move_to(sx, sy, 0.0);
        geo.arc_to(ex, ey, cx - sx, cy - sy, self.clockwise, 0.0);
        geo
    }

    fn append_to_geometry(
        &self,
        geo: &mut Geometry,
        model: &EntityRegistry,
        forward: bool,
    ) {
        let Some(((sx, sy), (ex, ey), (cx, cy))) = self.positions(model) else {
            return;
        };
        let (target, current) = if forward {
            ((ex, ey), (sx, sy))
        } else {
            ((sx, sy), (ex, ey))
        };
        let is_cw = if forward {
            self.clockwise
        } else {
            !self.clockwise
        };
        geo.arc_to(
            target.0,
            target.1,
            cx - current.0,
            cy - current.1,
            is_cw,
            0.0,
        );
    }

    fn to_polygon_vertices(
        &self,
        model: &EntityRegistry,
        forward: bool,
    ) -> Vec<(f64, f64)> {
        let Some(((sx, sy), (ex, ey), (cx, cy))) = self.positions(model) else {
            return Vec::new();
        };
        let radius = self.radius(model);
        let start_a = (sy - cy).atan2(sx - cx);
        let end_a = (ey - cy).atan2(ex - cx);

        // Scale the sample count by the swept angle so short arcs stay
        // coarse and ~360-degree arcs get enough resolution.
        let num_segments = 16;
        let max_segments = 64;

        let (from_angle, to_angle) = if forward {
            (start_a, end_a)
        } else {
            (end_a, start_a)
        };
        let sweep = crate::geo::shape::arc::get_arc_sweep(
            from_angle,
            to_angle,
            self.clockwise,
        );
        let count = ((num_segments as f64 * sweep.abs() / std::f64::consts::PI)
            as usize)
            .clamp(2, max_segments);
        let samples = crate::geo::shape::arc::get_arc_samples(
            Point::new(cx, cy),
            radius,
            from_angle,
            sweep,
            count,
        );
        samples.into_iter().map(|p| (p.x, p.y)).collect()
    }

    fn enclosed_signed_area(&self, _model: &EntityRegistry) -> Option<f64> {
        None
    }
}

impl Arc {
    /// Midpoint on the arc's circumference.
    pub fn get_midpoint(&self, model: &EntityRegistry) -> Option<(f64, f64)> {
        let ((sx, sy), (ex, ey), (cx, cy)) = self.positions(model)?;
        let mid = get_arc_midpoint(
            Point::new(sx, sy),
            Point::new(ex, ey),
            Point::new(cx, cy),
            self.clockwise,
        );
        Some((mid.x, mid.y))
    }
}
