//! Bezier entity, mirroring the reference `Bezier` class.

use serde_json::{json, Value};

use crate::geo::geometry::Geometry;
use crate::geo::shape::bezier::get_bezier_samples;
use crate::geo::shape::line::{
    does_line_segment_intersect_rect, get_line_segment_closest_point,
};
use crate::geo::shape::rect::does_rect_contain_rect;
use crate::geo::types::{Point, Rect};

use super::entity::{quantize, Entity, EntityBase};
use super::mirror::MirrorAxis;
use crate::sketch::registry::EntityRegistry;

/// A cubic bezier between two points, with the control points stored
/// as offsets relative to their owning endpoints.
#[derive(Clone, Debug)]
pub struct Bezier {
    pub base: EntityBase,
    pub start: u64,
    pub end: u64,
    pub cp1: Option<[f64; 2]>,
    pub cp2: Option<[f64; 2]>,
}

impl Bezier {
    pub fn new(
        id: u64,
        start: u64,
        end: u64,
        construction: bool,
        cp1: Option<[f64; 2]>,
        cp2: Option<[f64; 2]>,
    ) -> Self {
        Bezier {
            base: EntityBase {
                id,
                construction,
                invisible: false,
                array_copy: false,
                constrained: false,
            },
            start,
            end,
            cp1,
            cp2,
        }
    }

    pub fn from_json(data: &Value) -> Option<Bezier> {
        let cp = |dx: &str, dy: &str| -> Option<[f64; 2]> {
            Some([data.get(dx)?.as_f64()?, data.get(dy)?.as_f64()?])
        };
        Some(Bezier {
            base: super::entity::base_from_json(data),
            start: data.get("start_idx")?.as_u64()?,
            end: data.get("end_idx")?.as_u64()?,
            cp1: cp("cp1_dx", "cp1_dy"),
            cp2: cp("cp2_dx", "cp2_dy"),
        })
    }

    pub fn to_json(&self) -> Value {
        let base = self.base.to_base_json();
        let mut extra = json!({
            "type": self.type_name(),
            "start_idx": self.start,
            "end_idx": self.end,
        });
        if let Some([dx, dy]) = self.cp1 {
            extra["cp1_dx"] = json!(dx);
            extra["cp1_dy"] = json!(dy);
        }
        if let Some([dx, dy]) = self.cp2 {
            extra["cp2_dx"] = json!(dx);
            extra["cp2_dy"] = json!(dy);
        }
        EntityBase::combined_json(&base, extra)
    }

    /// Control point offsets; `None` per control point when absent
    /// (the reference stores `Option` per control point).
    pub fn get_control_points(&self) -> (Option<[f64; 2]>, Option<[f64; 2]>) {
        (self.cp1, self.cp2)
    }

    /// Absolute control point coordinates, falling back to the
    /// owning endpoint for absent control points.
    pub fn get_control_points_or_endpoints(
        &self,
        model: &EntityRegistry,
    ) -> (f64, f64, f64, f64) {
        let Some((sx, sy)) = model.point_position(self.start) else {
            return (0.0, 0.0, 0.0, 0.0);
        };
        let Some(end) = model.point_position(self.end) else {
            return (sx, sy, sx, sy);
        };
        let (ex, ey) = end;
        let cp1 = match self.cp1 {
            Some([dx, dy]) => (sx + dx, sy + dy),
            None => (sx, sy),
        };
        let cp2 = match self.cp2 {
            Some([dx, dy]) => (ex + dx, ey + dy),
            None => (ex, ey),
        };
        (cp1.0, cp1.1, cp2.0, cp2.1)
    }

    /// True when no control offsets exist: the curve degenerates to
    /// a straight segment.
    pub fn is_line(&self) -> bool {
        self.cp1.is_none() && self.cp2.is_none()
    }

    fn bbox(&self, model: &EntityRegistry) -> Rect {
        let Some((sx, sy)) = model.point_position(self.start) else {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        };
        let Some((ex, ey)) = model.point_position(self.end) else {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        };
        if self.is_line() {
            return Rect::new(sx.min(ex), sy.min(ey), sx.max(ex), sy.max(ey));
        }
        let (cp1x, cp1y, cp2x, cp2y) =
            self.get_control_points_or_endpoints(model);
        let xs = [sx, cp1x, cp2x, ex];
        let ys = [sy, cp1y, cp2y, ey];
        let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_y = ys.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_y = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        Rect::new(min_x, min_y, max_x, max_y)
    }
}

impl Entity for Bezier {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut EntityBase {
        &mut self.base
    }

    fn type_name(&self) -> &'static str {
        "bezier"
    }

    fn to_json(&self) -> Value {
        Bezier::to_json(self)
    }

    fn get_state(&self) -> Option<Value> {
        let cp1 = self.cp1.map(|p| json!([p[0], p[1]]));
        let cp2 = self.cp2.map(|p| json!([p[0], p[1]]));
        Some(json!({
            "construction": self.base.construction,
            "cp1": cp1,
            "cp2": cp2,
        }))
    }

    fn set_state(&mut self, state: &Value) {
        if let Some(v) = state.get("construction").and_then(Value::as_bool) {
            self.base.construction = v;
        }
        let read_cp = |key: &str| -> Option<[f64; 2]> {
            let array = state.get(key)?.as_array()?;
            Some([array.first()?.as_f64()?, array.get(1)?.as_f64()?])
        };
        if state.get("cp1").is_some() {
            self.cp1 = read_cp("cp1");
        }
        if state.get("cp2").is_some() {
            self.cp2 = read_cp("cp2");
        }
    }

    fn mirror(&mut self, axis: MirrorAxis) {
        // Control points are stored as (dx, dy) offsets relative to
        // the start/end points. The endpoints are mirrored by the
        // command, so the deltas must be mirrored too.
        if let Some(cp) = self.cp1 {
            self.cp1 = Some(axis.flip_offset(cp));
        }
        if let Some(cp) = self.cp2 {
            self.cp2 = Some(axis.flip_offset(cp));
        }
    }

    fn geometry_signature(&self, model: &EntityRegistry) -> Value {
        let mut signature =
            match super::entity::points_signature(model, &self.get_point_ids())
            {
                Value::Array(array) => array,
                _ => Vec::new(),
            };
        signature.push(match self.cp1 {
            Some([dx, dy]) => json!([quantize(dx), quantize(dy)]),
            None => Value::Null,
        });
        signature.push(match self.cp2 {
            Some([dx, dy]) => json!([quantize(dx), quantize(dy)]),
            None => Value::Null,
        });
        Value::Array(signature)
    }

    fn get_point_ids(&self) -> Vec<u64> {
        vec![self.start, self.end]
    }

    fn get_endpoint_ids(&self) -> Vec<u64> {
        vec![self.start, self.end]
    }

    fn get_junction_point_ids(&self) -> Vec<u64> {
        vec![self.start, self.end]
    }

    fn is_edge_entity(&self) -> bool {
        true
    }

    fn supports_point_on_curve(&self) -> bool {
        true
    }

    fn characteristic_length_pairs(&self) -> Vec<[u64; 2]> {
        vec![[self.start, self.end]]
    }

    fn tangent_at(
        &self,
        model: &EntityRegistry,
        point_id: u64,
    ) -> Option<(f64, f64)> {
        let (sx, sy) = model.point_position(self.start)?;
        let (ex, ey) = model.point_position(self.end)?;
        let (cp1x, cp1y, cp2x, cp2y) =
            self.get_control_points_or_endpoints(model);
        if point_id == self.start {
            Some((cp1x - sx, cp1y - sy))
        } else {
            Some((ex - cp2x, ey - cp2y))
        }
    }

    fn signed_distance_to(
        &self,
        px: f64,
        py: f64,
        model: &EntityRegistry,
    ) -> Option<f64> {
        let (sx, sy) = model.point_position(self.start)?;
        let (ex, ey) = model.point_position(self.end)?;
        if self.is_line() {
            let (_, _, dist_sq) = get_line_segment_closest_point(
                Point::new(sx, sy),
                Point::new(ex, ey),
                px,
                py,
            );
            return Some(dist_sq.sqrt());
        }
        let (cp1x, cp1y, cp2x, cp2y) =
            self.get_control_points_or_endpoints(model);
        match crate::geo::shape::bezier::get_bezier_closest_point(
            crate::geo::types::Point3D::new(ex, ey, 0.0),
            crate::geo::types::Point3D::new(cp1x, cp1y, 0.0),
            crate::geo::types::Point3D::new(cp2x, cp2y, 0.0),
            crate::geo::types::Point3D::new(sx, sy, 0.0),
            px,
            py,
        ) {
            Some((_, _, dist_sq)) => Some(dist_sq.sqrt()),
            None => Some(0.0),
        }
    }

    fn hit_test(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
    ) -> bool {
        let (Some((sx, sy)), Some((ex, ey))) = (
            model.point_position(self.start),
            model.point_position(self.end),
        ) else {
            return false;
        };
        if self.is_line() {
            let (_, _, dist_sq) = get_line_segment_closest_point(
                Point::new(sx, sy),
                Point::new(ex, ey),
                mx,
                my,
            );
            return dist_sq < threshold * threshold;
        }
        let (cp1x, cp1y, cp2x, cp2y) =
            self.get_control_points_or_endpoints(model);
        let points = get_bezier_samples(
            Point::new(sx, sy),
            Point::new(cp1x, cp1y),
            Point::new(cp2x, cp2y),
            Point::new(ex, ey),
            20,
        );
        points.windows(2).any(|pair| {
            let (_, _, dist_sq) =
                get_line_segment_closest_point(pair[0], pair[1], mx, my);
            dist_sq < threshold * threshold
        })
    }

    fn compute_constrained(
        &self,
        model: &EntityRegistry,
        _constrains_radius: &dyn Fn(u64) -> bool,
    ) -> bool {
        let s = model.get_point(self.start).map(|p| p.constrained);
        let e = model.get_point(self.end).map(|p| p.constrained);
        s.unwrap_or(false) && e.unwrap_or(false)
    }

    fn is_contained_by(&self, rect: Rect, model: &EntityRegistry) -> bool {
        does_rect_contain_rect(rect, self.bbox(model))
    }

    fn intersects_rect(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let (Some((sx, sy)), Some((ex, ey))) = (
            model.point_position(self.start),
            model.point_position(self.end),
        ) else {
            return false;
        };
        if self.is_line() {
            return does_line_segment_intersect_rect(
                Point::new(sx, sy),
                Point::new(ex, ey),
                rect,
            );
        }
        let (cp1x, cp1y, cp2x, cp2y) =
            self.get_control_points_or_endpoints(model);
        let points = get_bezier_samples(
            Point::new(sx, sy),
            Point::new(cp1x, cp1y),
            Point::new(cp2x, cp2y),
            Point::new(ex, ey),
            20,
        );
        for pair in points.windows(2) {
            if does_line_segment_intersect_rect(pair[0], pair[1], rect) {
                return true;
            }
        }
        points.iter().any(|p| {
            rect.min.x <= p.x
                && p.x <= rect.max.x
                && rect.min.y <= p.y
                && p.y <= rect.max.y
        })
    }

    fn to_geometry(&self, model: &EntityRegistry) -> Geometry {
        let mut geo = Geometry::new();
        let (Some((sx, sy)), Some((ex, ey))) = (
            model.point_position(self.start),
            model.point_position(self.end),
        ) else {
            return geo;
        };
        geo.move_to(sx, sy, 0.0);
        if self.is_line() {
            geo.line_to(ex, ey, 0.0);
        } else {
            let (cp1x, cp1y, cp2x, cp2y) =
                self.get_control_points_or_endpoints(model);
            use crate::geo::types::Point3D;
            geo.bezier_to(
                Point3D::new(cp1x, cp1y, 0.0),
                Point3D::new(cp2x, cp2y, 0.0),
                Point3D::new(ex, ey, 0.0),
            );
        }
        geo
    }

    fn append_to_geometry(
        &self,
        geo: &mut Geometry,
        model: &EntityRegistry,
        forward: bool,
    ) {
        let (Some((sx, sy)), Some((ex, ey))) = (
            model.point_position(self.start),
            model.point_position(self.end),
        ) else {
            return;
        };
        if self.is_line() {
            if forward {
                geo.line_to(ex, ey, 0.0);
            } else {
                geo.line_to(sx, sy, 0.0);
            }
            return;
        }
        let (cp1x, cp1y, cp2x, cp2y) =
            self.get_control_points_or_endpoints(model);
        use crate::geo::types::Point3D;
        if forward {
            geo.bezier_to(
                Point3D::new(cp1x, cp1y, 0.0),
                Point3D::new(cp2x, cp2y, 0.0),
                Point3D::new(ex, ey, 0.0),
            );
        } else {
            geo.bezier_to(
                Point3D::new(cp2x, cp2y, 0.0),
                Point3D::new(cp1x, cp1y, 0.0),
                Point3D::new(sx, sy, 0.0),
            );
        }
    }

    fn to_polygon_vertices(
        &self,
        model: &EntityRegistry,
        forward: bool,
    ) -> Vec<(f64, f64)> {
        let (Some((sx, sy)), Some((ex, ey))) = (
            model.point_position(self.start),
            model.point_position(self.end),
        ) else {
            return Vec::new();
        };
        if self.is_line() {
            return if forward {
                vec![(sx, sy)]
            } else {
                vec![(ex, ey)]
            };
        }
        let (cp1x, cp1y, cp2x, cp2y) =
            self.get_control_points_or_endpoints(model);
        let mut points: Vec<(f64, f64)> = get_bezier_samples(
            Point::new(sx, sy),
            Point::new(cp1x, cp1y),
            Point::new(cp2x, cp2y),
            Point::new(ex, ey),
            20,
        )
        .into_iter()
        .map(|p| (p.x, p.y))
        .collect();
        if !forward {
            points.reverse();
        }
        points
    }
}
