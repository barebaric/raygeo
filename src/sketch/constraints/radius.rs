//! Radius constraint: radius of a circle or arc.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_conflict_underlay, draw_label, draw_status_color, format_value,
    screen_scale, CairoLike, DrawStyle, LABEL_DIST_SCREEN,
};
use crate::sketch::registry::EntityRegistry;

/// Enforces the radius of a circle or arc: the distance between the
/// center point and the radius-bearing point equals `value`.
#[derive(Clone, Debug, PartialEq)]
pub struct RadiusConstraint {
    /// Center point of the circle/arc.
    pub center: u64,
    /// Radius-bearing point (circle radius point or arc start).
    pub radius_point: u64,
    /// Resolved radius value.
    pub value: f64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Stored expression string (evaluated caller-side).
    pub expression: Option<String>,
}

impl RadiusConstraint {
    /// Creates a radius constraint with a resolved value.
    pub fn new(center: u64, radius_point: u64, value: f64) -> Self {
        RadiusConstraint {
            center,
            radius_point,
            value,
            user_visible: true,
            expression: None,
        }
    }
}

impl SketchConstraint for RadiusConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.center, self.radius_point]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (cx, cy) = reg.xy(self.center);
        let (px, py) = reg.xy(self.radius_point);
        vec![(px - cx).hypot(py - cy) - self.value]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let (cx, cy) = reg.xy(self.center);
        let (px, py) = reg.xy(self.radius_point);
        let dx = px - cx;
        let dy = py - cy;
        let dist = dx.hypot(dy);
        let (ux, uy) = if dist > 1e-9 {
            (dx / dist, dy / dist)
        } else {
            (1.0, 0.0)
        };
        vec![(self.radius_point, 0, ux, uy), (self.center, 0, -ux, -uy)]
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        _threshold: f64,
        model: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        // Label position mirroring the reference circle branch: the
        // label sits `radius + 20 / scale` from the center along the
        // radius point direction.  (The reference arc branch angles
        // the label along the arc midpoint, which is not resolvable
        // from the center/radius point pair alone.)
        let (Some(center), Some(radius_pt)) = (
            model.get_point(self.center),
            model.get_point(self.radius_point),
        ) else {
            return false;
        };
        let radius = (radius_pt.x - center.x).hypot(radius_pt.y - center.y);
        if radius == 0.0 {
            return false;
        }

        let p0 = to_screen((0.0, 0.0));
        let p1 = to_screen((1.0, 0.0));
        let mut scale = (p1.0 - p0.0).hypot(p1.1 - p0.1);
        if scale < 1e-9 {
            scale = 1.0;
        }

        let mid_angle = (radius_pt.y - center.y).atan2(radius_pt.x - center.x);
        let label_dist = radius + 20.0 / scale;
        let (label_sx, label_sy) = to_screen((
            center.x + label_dist * mid_angle.cos(),
            center.y + label_dist * mid_angle.sin(),
        ));

        let label_width = 20.0;
        let label_height = 20.0;
        let half_w = label_width / 2.0;
        let half_h = label_height / 2.0;

        let x_min = label_sx - half_w - 4.0;
        let x_max = label_sx + half_w + 4.0;
        let y_min = label_sy - half_h - 4.0;
        let y_max = label_sy + half_h + 4.0;

        x_min <= mx && mx <= x_max && y_min <= my && my <= y_max
    }

    fn draw(
        &self,
        registry: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        // Label position mirroring the reference circle branch: the
        // label sits `radius + label_dist / scale` from the center
        // along the radius point direction.
        let (Some(center), Some(radius_pt)) = (
            registry.get_point(self.center),
            registry.get_point(self.radius_point),
        ) else {
            return;
        };
        let radius = (radius_pt.x - center.x).hypot(radius_pt.y - center.y);
        if radius == 0.0 {
            return;
        }
        let scale = screen_scale(to_screen);
        let mid_angle = (radius_pt.y - center.y).atan2(radius_pt.x - center.x);
        let label_dist = radius + LABEL_DIST_SCREEN / scale;
        let (sx, sy) = to_screen((
            center.x + label_dist * mid_angle.cos(),
            center.y + label_dist * mid_angle.sin(),
        ));
        let (arc_mid_sx, arc_mid_sy) = to_screen((
            center.x + radius * mid_angle.cos(),
            center.y + radius * mid_angle.sin(),
        ));

        let label = format!("R{}", format_value(self.value));
        ctx.save();
        draw_label(ctx, &label, sx, sy, style, screen_scale(to_screen));
        ctx.set_line_width(1.0);
        ctx.set_dash(4.0, 4.0);
        ctx.move_to(sx, sy);
        ctx.line_to(arc_mid_sx, arc_mid_sy);
        draw_conflict_underlay(ctx, style);
        draw_status_color(ctx, style);
        ctx.stroke();
        ctx.restore();
    }
}

impl RadiusConstraint {
    /// Reference user-visibility flag.
    pub fn user_visible(&self) -> bool {
        self.user_visible
    }

    /// Sets the reference user-visibility flag.
    pub fn set_user_visible(&mut self, value: bool) {
        self.user_visible = value;
    }

    /// The stored expression string, if any (evaluated caller-side).
    pub fn expression(&self) -> Option<&str> {
        self.expression.as_deref()
    }

    /// Sets the expression string (evaluated caller-side).
    pub fn set_expression(&mut self, expression: Option<String>) {
        self.expression = expression;
    }

    /// Serializes to the reference constraint dictionary shape.
    pub fn to_json(&self) -> Value {
        let mut data = json!({
            "type": "RadiusConstraint",
            "user_visible": self.user_visible,
        });
        data["center_idx"] = json!(self.center);
        data["radius_pt_idx"] = json!(self.radius_point);
        data["value"] = json!(self.value);
        if let Some(expression) = &self.expression {
            data["expression"] = json!(expression);
        }
        data
    }
}
