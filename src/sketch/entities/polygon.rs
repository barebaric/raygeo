//! Polygon entity, mirroring the reference `PolygonEntity` class.

use serde_json::{json, Value};

use crate::geo::geometry::Geometry;
use crate::geo::shape::line::{
    does_line_segment_intersect_rect, get_line_segment_closest_point,
};
use crate::geo::shape::polygon::{
    get_polygon_signed_area, is_point_inside_polygon,
};
use crate::geo::types::{Point, Rect};

use super::entity::{quantize, Entity, EntityBase};
use super::mirror::MirrorAxis;
use crate::sketch::registry::EntityRegistry;

/// A polygon (optionally multi-ring) defined in a local frame: the
/// rings are (u, v) coordinates relative to the frame spanned by
/// center and handle points.
#[derive(Clone, Debug)]
pub struct PolygonEntity {
    pub base: EntityBase,
    pub center: u64,
    pub handle: u64,
    /// Ring 0 is the outer contour; further rings are holes.
    pub rings: Vec<Vec<(f64, f64)>>,
    pub closed: bool,
}

impl PolygonEntity {
    pub fn new(
        id: u64,
        center: u64,
        handle: u64,
        vertices: Vec<(f64, f64)>,
        closed: bool,
        construction: bool,
        holes: Vec<Vec<(f64, f64)>>,
    ) -> Self {
        let mut rings = vec![vertices];
        rings.extend(holes);
        let mut entity = PolygonEntity {
            base: EntityBase {
                id,
                construction,
                invisible: false,
                array_copy: false,
                constrained: false,
                fill_color: None,
            },
            center,
            handle,
            rings,
            closed,
        };
        entity.normalize_ring_winding();
        entity
    }

    pub fn from_json(data: &Value) -> Option<PolygonEntity> {
        let read_ring = |value: &Value| -> Vec<(f64, f64)> {
            value
                .as_array()
                .map(|array| {
                    array
                        .iter()
                        .filter_map(|vertex| {
                            let pair = vertex.as_array()?;
                            Some((
                                pair.first()?.as_f64()?,
                                pair.get(1)?.as_f64()?,
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let outer = read_ring(data.get("vertices")?);
        let holes: Vec<Vec<(f64, f64)>> = data
            .get("rings")
            .and_then(Value::as_array)
            .map(|arrays| arrays.iter().map(read_ring).collect())
            .unwrap_or_default();
        let mut rings = vec![outer];
        rings.extend(holes);
        Some(PolygonEntity {
            base: super::entity::base_from_json(data),
            center: data.get("center_idx")?.as_u64()?,
            handle: data.get("handle_idx")?.as_u64()?,
            rings,
            closed: data
                .get("closed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }

    pub fn to_json(&self) -> Value {
        let ring_json = |ring: &[(f64, f64)]| -> Value {
            json!(ring.iter().map(|(u, v)| [u, v]).collect::<Vec<_>>())
        };
        let base = self.base.to_base_json();
        let mut extra = json!({
            "type": self.type_name(),
            "center_idx": self.center,
            "handle_idx": self.handle,
            "vertices": ring_json(&self.rings[0]),
            "closed": self.closed,
        });
        if self.rings.len() > 1 {
            extra["rings"] = json!(self.rings[1..]
                .iter()
                .map(|r| ring_json(r))
                .collect::<Vec<_>>());
        }
        EntityBase::combined_json(&base, extra)
    }

    /// Enforces the winding convention on closed shapes: ring 0 CCW,
    /// hole rings CW. The local frame is right-handed, so local
    /// winding matches world winding.
    pub fn normalize_ring_winding(&mut self) {
        if !self.closed {
            return;
        }
        if let Some(outer) = self.rings.first_mut() {
            if get_polygon_signed_area(&to_points(outer)) < 0.0 {
                outer.reverse();
            }
        }
        for ring in self.rings.iter_mut().skip(1) {
            if get_polygon_signed_area(&to_points(ring)) > 0.0 {
                ring.reverse();
            }
        }
    }

    /// Frame basis: (cx, cy, x_hat_x, x_hat_y, scale).
    fn frame(&self, model: &EntityRegistry) -> (f64, f64, f64, f64, f64) {
        let Some((cx, cy)) = model.point_position(self.center) else {
            return (0.0, 0.0, 1.0, 0.0, 0.0);
        };
        let Some((hx, hy)) = model.point_position(self.handle) else {
            return (cx, cy, 1.0, 0.0, 0.0);
        };
        let (dx, dy) = (hx - cx, hy - cy);
        let scale = dx.hypot(dy);
        if scale < 1e-9 {
            (cx, cy, 1.0, 0.0, 0.0)
        } else {
            (cx, cy, dx / scale, dy / scale, scale)
        }
    }

    /// The outer ring in world coordinates.
    pub fn get_world_vertices(
        &self,
        model: &EntityRegistry,
    ) -> Vec<(f64, f64)> {
        self.get_world_rings(model)
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    /// Every ring in world coordinates (outer first).
    pub fn get_world_rings(
        &self,
        model: &EntityRegistry,
    ) -> Vec<Vec<(f64, f64)>> {
        let (cx, cy, ux, uy, scale) = self.frame(model);
        self.rings
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|(u, v)| {
                        (
                            cx + scale * (u * ux - v * uy),
                            cy + scale * (u * uy + v * ux),
                        )
                    })
                    .collect()
            })
            .collect()
    }

    /// (world_vertices, closed) for offset processing.
    pub fn to_outline(
        &self,
        model: &EntityRegistry,
    ) -> (Vec<(f64, f64)>, bool) {
        (self.get_world_vertices(model), self.closed)
    }
}

fn to_points(ring: &[(f64, f64)]) -> Vec<Point> {
    ring.iter().map(|(x, y)| Point::new(*x, *y)).collect()
}

impl Entity for PolygonEntity {
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
        "polygon"
    }

    fn to_json(&self) -> Value {
        PolygonEntity::to_json(self)
    }

    fn get_point_ids(&self) -> Vec<u64> {
        vec![self.center, self.handle]
    }

    fn get_junction_point_ids(&self) -> Vec<u64> {
        vec![self.center, self.handle]
    }

    fn get_state(&self) -> Option<Value> {
        let mut state = json!({
            "construction": self.base.construction,
            "vertices": self.rings[0],
            "closed": self.closed,
        });
        if self.rings.len() > 1 {
            state["rings"] = json!(self.rings[1..]);
        }
        Some(state)
    }

    fn set_state(&mut self, state: &Value) {
        if let Some(v) = state.get("construction").and_then(Value::as_bool) {
            self.base.construction = v;
        }
        let read_ring = |value: &Value| -> Vec<(f64, f64)> {
            value
                .as_array()
                .map(|array| {
                    array
                        .iter()
                        .filter_map(|vertex| {
                            let pair = vertex.as_array()?;
                            Some((
                                pair.first()?.as_f64()?,
                                pair.get(1)?.as_f64()?,
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
        if let Some(outer) = state.get("vertices") {
            rings.push(read_ring(outer));
        }
        if let Some(holes) = state.get("rings").and_then(Value::as_array) {
            for hole in holes {
                rings.push(read_ring(hole));
            }
        }
        if !rings.is_empty() {
            self.rings = rings;
        }
        if let Some(v) = state.get("closed").and_then(Value::as_bool) {
            self.closed = v;
        }
        self.normalize_ring_winding();
    }

    fn geometry_signature(&self, model: &EntityRegistry) -> Value {
        let mut signature =
            match super::entity::points_signature(model, &self.get_point_ids())
            {
                Value::Array(array) => array,
                _ => Vec::new(),
            };
        signature.push(json!(self
            .rings
            .iter()
            .map(|ring| json!(ring
                .iter()
                .map(|(u, v)| [quantize(*u), quantize(*v)])
                .collect::<Vec<_>>()))
            .collect::<Vec<_>>()));
        signature.push(json!(self.closed));
        Value::Array(signature)
    }

    fn compute_constrained(
        &self,
        model: &EntityRegistry,
        _constrains_radius: &dyn Fn(u64) -> bool,
    ) -> bool {
        let c = model.get_point(self.center).map(|p| p.constrained);
        let h = model.get_point(self.handle).map(|p| p.constrained);
        c.unwrap_or(false) && h.unwrap_or(false)
    }

    fn get_rigidly_connected_points(&self, point_id: u64) -> Vec<u64> {
        if point_id == self.center {
            return vec![self.center, self.handle];
        }
        Vec::new()
    }

    fn get_drag_anchor_points(&self, point_id: u64) -> Vec<u64> {
        if point_id == self.handle {
            return vec![self.center];
        }
        Vec::new()
    }

    fn is_closed_loop(&self) -> bool {
        self.closed
    }

    /// The frame points are mirrored centrally by the command, which
    /// flips the frame's chirality; negating the local v component
    /// mirrors the outline with it (true for both mirror axes).
    fn mirror(&mut self, _axis: MirrorAxis) {
        for ring in self.rings.iter_mut() {
            for vertex in ring.iter_mut() {
                vertex.1 = -vertex.1;
            }
        }
        self.normalize_ring_winding();
    }

    fn enclosed_signed_area(&self, model: &EntityRegistry) -> Option<f64> {
        // By convention, a single closed loop is CCW -> positive area.
        let rings = self.get_world_rings(model);
        let mut area = get_polygon_signed_area(&to_points(&rings[0])).abs();
        for hole in rings.iter().skip(1) {
            area -= get_polygon_signed_area(&to_points(hole)).abs();
        }
        Some(area)
    }

    fn contains_point(&self, model: &EntityRegistry, x: f64, y: f64) -> bool {
        // Inside the outer ring and not inside any hole.
        if !self.closed {
            return false;
        }
        let rings = self.get_world_rings(model);
        if rings[0].len() < 3
            || !is_point_inside_polygon(Point::new(x, y), &to_points(&rings[0]))
        {
            return false;
        }
        rings[1..]
            .iter()
            .filter(|hole| hole.len() >= 3)
            .all(|hole| {
                !is_point_inside_polygon(Point::new(x, y), &to_points(hole))
            })
    }

    fn hit_test(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
    ) -> bool {
        for vertices in self.get_world_rings(model) {
            if vertices.len() < 2 {
                continue;
            }
            let count = vertices.len();
            let last = if self.closed { count } else { count - 1 };
            for i in 0..last {
                let a = vertices[i];
                let b = vertices[(i + 1) % count];
                let (_, _, dist_sq) = get_line_segment_closest_point(
                    Point::new(a.0, a.1),
                    Point::new(b.0, b.1),
                    mx,
                    my,
                );
                if dist_sq < threshold * threshold {
                    return true;
                }
            }
        }
        false
    }

    fn is_contained_by(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let rings = self.get_world_rings(model);
        if rings.is_empty() || rings[0].is_empty() {
            return false;
        }
        rings.iter().all(|ring| {
            ring.iter().all(|(x, y)| {
                rect.min.x <= *x
                    && *x <= rect.max.x
                    && rect.min.y <= *y
                    && *y <= rect.max.y
            })
        })
    }

    fn intersects_rect(&self, rect: Rect, model: &EntityRegistry) -> bool {
        let rings = self.get_world_rings(model);
        if rings.is_empty() || rings[0].len() < 2 {
            return false;
        }
        for vertices in &rings {
            let count = vertices.len();
            let last = if self.closed { count } else { count - 1 };
            for i in 0..last {
                let a = vertices[i];
                let b = vertices[(i + 1) % count];
                if does_line_segment_intersect_rect(
                    Point::new(a.0, a.1),
                    Point::new(b.0, b.1),
                    rect,
                ) {
                    return true;
                }
            }
        }
        let center = (
            (rect.min.x + rect.max.x) / 2.0,
            (rect.min.y + rect.max.y) / 2.0,
        );
        if self.contains_point(model, center.0, center.1) {
            return true;
        }
        self.is_contained_by(rect, model)
    }

    fn to_geometry(&self, model: &EntityRegistry) -> Geometry {
        let mut geo = Geometry::new();
        let rings = self.get_world_rings(model);
        for ring in rings {
            let Some(&first) = ring.first() else {
                continue;
            };
            geo.move_to(first.0, first.1, 0.0);
            for (x, y) in ring.iter().skip(1) {
                geo.line_to(*x, *y, 0.0);
            }
            if self.closed {
                geo.close_path();
            }
        }
        geo
    }

    fn to_polygon_vertices(
        &self,
        model: &EntityRegistry,
        forward: bool,
    ) -> Vec<(f64, f64)> {
        let mut vertices = self.get_world_vertices(model);
        if !forward {
            vertices.reverse();
        }
        vertices
    }
}
