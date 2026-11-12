//! Vertical constraint: two points share their X coordinate.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_conflict_underlay, draw_selection_underlay, draw_status_color,
    CairoLike, DrawStyle, MARKER_OFFSET, MARKER_SIZE, MARKER_T,
};
use crate::sketch::registry::EntityRegistry;

/// Enforces two points have the same X coordinate.
#[derive(Clone, Debug, PartialEq)]
pub struct VerticalConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl VerticalConstraint {
    /// Creates a vertical constraint between two points.
    pub fn new(p1: u64, p2: u64) -> Self {
        VerticalConstraint {
            user_visible: true,
            p1,
            p2,
        }
    }
}

impl SketchConstraint for VerticalConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.p1, self.p2]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        vec![reg.xy(self.p1).0 - reg.xy(self.p2).0]
    }

    fn gradient(&self, _reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        vec![(self.p1, 0, 1.0, 0.0), (self.p2, 0, -1.0, 0.0)]
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        let (Some(p1), Some(p2)) =
            (model.get_point(self.p1), model.get_point(self.p2))
        else {
            return false;
        };
        let s1 = to_screen((p1.x, p1.y));
        let s2 = to_screen((p2.x, p2.y));

        let t = 0.2;
        let sym_x = s1.0 + (s2.0 - s1.0) * t;
        let sym_y = s1.1 + (s2.1 - s1.1) * t;
        let cx = sym_x + 10.0;
        let cy = sym_y;
        (mx - cx).hypot(my - cy) < threshold
    }

    fn draw(
        &self,
        registry: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        let (Some(p1), Some(p2)) =
            (registry.get_point(self.p1), registry.get_point(self.p2))
        else {
            return;
        };
        let s1 = to_screen((p1.x, p1.y));
        let s2 = to_screen((p2.x, p2.y));

        let mx = s1.0 + (s2.0 - s1.0) * MARKER_T;
        let my = s1.1 + (s2.1 - s1.1) * MARKER_T;

        ctx.save();
        ctx.set_line_width(2.0);
        ctx.move_to(mx + MARKER_OFFSET, my - MARKER_SIZE);
        ctx.line_to(mx + MARKER_OFFSET, my + MARKER_SIZE);
        if style.is_selected {
            draw_selection_underlay(ctx);
        }
        draw_conflict_underlay(ctx, style);
        draw_status_color(ctx, style);
        ctx.stroke();
        ctx.restore();
    }
}

impl VerticalConstraint {
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
            "type": "VerticalConstraint",
            "user_visible": self.user_visible,
        });
        data["p1"] = json!(self.p1);
        data["p2"] = json!(self.p2);
        data
    }
}
