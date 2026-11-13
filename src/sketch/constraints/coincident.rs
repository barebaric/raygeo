//! Coincident constraint: two points at the same location.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_point_marker, CairoLike, DrawContext, DrawStyle,
};
use crate::sketch::registry::EntityRegistry;

/// Enforces two points are at the same location.
#[derive(Clone, Debug, PartialEq)]
pub struct CoincidentConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl CoincidentConstraint {
    /// Creates a coincident constraint between two points.
    pub fn new(p1: u64, p2: u64) -> Self {
        CoincidentConstraint {
            user_visible: true,
            p1,
            p2,
        }
    }
}

impl SketchConstraint for CoincidentConstraint {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_json(&self) -> Value {
        CoincidentConstraint::to_json(self)
    }

    fn referenced_points(&self) -> Vec<u64> {
        vec![self.p1, self.p2]
    }

    fn residual_count(&self) -> usize {
        2
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        vec![x1 - x2, y1 - y2]
    }

    fn gradient(&self, _reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        vec![
            (self.p1, 0, 1.0, 0.0),
            (self.p1, 1, 0.0, 1.0),
            (self.p2, 0, -1.0, 0.0),
            (self.p2, 1, 0.0, -1.0),
        ]
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        // The reference picks the non-origin point through the
        // element's sketch; without an element here, prefer the
        // non-fixed point (the origin is the fixed one).
        let p1 = model.get_point(self.p1);
        let p2 = model.get_point(self.p2);
        let (Some(p1), Some(p2)) = (p1, p2) else {
            return false;
        };
        let pid = if p1.fixed && !p2.fixed {
            self.p2
        } else {
            self.p1
        };
        let Some(point) = model.get_point(pid) else {
            return false;
        };
        let s_point = to_screen((point.x, point.y));
        (mx - s_point.0).hypot(my - s_point.1) < threshold
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        // Prefer the non-fixed point if one is the origin, to avoid
        // clutter on the origin.
        let p1 = context.get_point(self.p1);
        let p2 = context.get_point(self.p2);
        let (Some(p1), Some(p2)) = (p1, p2) else {
            return;
        };
        let pid = if p1.fixed && !p2.fixed {
            self.p2
        } else {
            self.p1
        };
        let Some(point) = context.get_point(pid) else {
            return;
        };
        let (sx, sy) = to_screen((point.x, point.y));
        draw_point_marker(ctx, sx, sy, style);
    }
}

impl CoincidentConstraint {
    /// Reference user-visibility flag.
    pub fn user_visible(&self) -> bool {
        self.user_visible
    }

    /// Sets the reference user-visibility flag.
    pub fn set_user_visible(&mut self, value: bool) {
        self.user_visible = value;
    }

    /// Serializes to the reference constraint dictionary shape.
    pub fn to_json(&self) -> Value {
        let mut data = json!({
            "type": "CoincidentConstraint",
            "user_visible": self.user_visible,
        });
        data["p1"] = json!(self.p1);
        data["p2"] = json!(self.p2);
        data
    }
}
