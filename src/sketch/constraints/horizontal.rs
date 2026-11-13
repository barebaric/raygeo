//! Horizontal constraint: two points share their Y coordinate.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_conflict_underlay, draw_selection_underlay, draw_status_color,
    CairoLike, DrawContext, DrawStyle, MARKER_OFFSET, MARKER_SIZE, MARKER_T,
};
use crate::sketch::registry::EntityRegistry;

/// Enforces two points have the same Y coordinate.
#[derive(Clone, Debug, PartialEq)]
pub struct HorizontalConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl HorizontalConstraint {
    /// Creates a horizontal constraint between two points.
    pub fn new(p1: u64, p2: u64) -> Self {
        HorizontalConstraint {
            user_visible: true,
            p1,
            p2,
        }
    }
}

impl SketchConstraint for HorizontalConstraint {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_json(&self) -> Value {
        HorizontalConstraint::to_json(self)
    }

    fn referenced_points(&self) -> Vec<u64> {
        vec![self.p1, self.p2]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        vec![reg.xy(self.p1).1 - reg.xy(self.p2).1]
    }

    fn gradient(&self, _reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        vec![(self.p1, 0, 0.0, 1.0), (self.p2, 0, 0.0, -1.0)]
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
        let cx = sym_x;
        let cy = sym_y - 10.0;
        (mx - cx).hypot(my - cy) < threshold
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        let (Some(p1), Some(p2)) =
            (context.get_point(self.p1), context.get_point(self.p2))
        else {
            return;
        };
        let s1 = to_screen((p1.x, p1.y));
        let s2 = to_screen((p2.x, p2.y));

        let mx = s1.0 + (s2.0 - s1.0) * MARKER_T;
        let my = s1.1 + (s2.1 - s1.1) * MARKER_T;

        ctx.save();
        ctx.set_line_width(2.0);
        ctx.move_to(mx - MARKER_SIZE, my - MARKER_OFFSET);
        ctx.line_to(mx + MARKER_SIZE, my - MARKER_OFFSET);
        if style.is_selected {
            draw_selection_underlay(ctx);
        }
        draw_conflict_underlay(ctx, style);
        draw_status_color(ctx, style);
        ctx.stroke();
        ctx.restore();
    }
}

impl HorizontalConstraint {
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
            "type": "HorizontalConstraint",
            "user_visible": self.user_visible,
        });
        data["p1"] = json!(self.p1);
        data["p2"] = json!(self.p2);
        data
    }
}
