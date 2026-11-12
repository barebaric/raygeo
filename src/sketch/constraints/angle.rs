//! Angle constraint between two lines.

use crate::geo::shape::arc::normalize_angle;
use crate::geo::shape::line::get_line_line_intersection;
use crate::geo::types::Point;
use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_conflict_underlay, draw_label, draw_selection_underlay,
    draw_status_color, format_value, screen_scale, CairoLike, DrawContext,
    DrawStyle,
};
use crate::sketch::registry::EntityRegistry;

/// Weight of the angular residual.
pub const ANGLE_WEIGHT: f64 = 10.0;

/// Screen-space radius of the angle marker arc, matching the
/// reference renderer.
pub const ARC_RADIUS: f64 = 35.0;

/// Screen-space distance of the angle label from the arc center.
pub const LABEL_RADIUS: f64 = ARC_RADIUS - 10.0;

/// Central-difference step used for the Jacobian.
const GRAD_FD_EPS: f64 = 1e-6;

/// Enforces a specific angle in degrees between two lines.
///
/// The angle is measured clockwise from the anchor direction towards
/// the other direction, using the rays pointing away from the lines'
/// intersection towards the far points.  The Jacobian uses central
/// finite differences, mirroring the reference implementation.
#[derive(Clone, Debug, PartialEq)]
pub struct AngleConstraint {
    /// First point of the anchor line.
    pub p1: u64,
    /// Second point of the anchor line.
    pub p2: u64,
    /// First point of the other line.
    pub p3: u64,
    /// Second point of the other line.
    pub p4: u64,
    /// Far point of the anchor line; `None` picks the endpoint
    /// farther from the intersection.
    pub e1_far: Option<u64>,
    /// Far point of the other line.
    pub e2_far: Option<u64>,
    /// Resolved angle value in degrees.
    pub value_deg: f64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Stored expression string (evaluated caller-side).
    pub expression: Option<String>,
}

impl AngleConstraint {
    /// Creates an angle constraint with a resolved value.
    pub fn new(
        p1: u64,
        p2: u64,
        p3: u64,
        p4: u64,
        e1_far: Option<u64>,
        e2_far: Option<u64>,
        value_deg: f64,
    ) -> Self {
        AngleConstraint {
            p1,
            p2,
            p3,
            p4,
            e1_far,
            e2_far,
            value_deg,
            user_visible: true,
            expression: None,
        }
    }

    fn far_point(
        &self,
        first: u64,
        second: u64,
        far: Option<u64>,
        ix: f64,
        iy: f64,
        reg: &dyn PointContext,
    ) -> (f64, f64) {
        if let Some(id) = far {
            if id == first {
                return reg.xy(first);
            }
            if id == second {
                return reg.xy(second);
            }
        }
        let a = reg.xy(first);
        let b = reg.xy(second);
        let d1 = (a.0 - ix).powi(2) + (a.1 - iy).powi(2);
        let d2 = (b.0 - ix).powi(2) + (b.1 - iy).powi(2);
        if d1 > d2 {
            a
        } else {
            b
        }
    }
}

impl SketchConstraint for AngleConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        let mut ids = vec![self.p1, self.p2, self.p3, self.p4];
        for far in [self.e1_far, self.e2_far].into_iter().flatten() {
            if !ids.contains(&far) {
                ids.push(far);
            }
        }
        ids
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let intersection = get_line_line_intersection(
            Point::new(reg.xy(self.p1).0, reg.xy(self.p1).1),
            Point::new(reg.xy(self.p2).0, reg.xy(self.p2).1),
            Point::new(reg.xy(self.p3).0, reg.xy(self.p3).1),
            Point::new(reg.xy(self.p4).0, reg.xy(self.p4).1),
        );
        let (ix, iy) = match intersection {
            Some(point) => (point.x, point.y),
            None => return vec![0.0],
        };

        let far1 = self.far_point(self.p1, self.p2, self.e1_far, ix, iy, reg);
        let far2 = self.far_point(self.p3, self.p4, self.e2_far, ix, iy, reg);

        let dx1 = far1.0 - ix;
        let dy1 = far1.1 - iy;
        let dx2 = far2.0 - ix;
        let dy2 = far2.1 - iy;
        let len1_sq = dx1 * dx1 + dy1 * dy1;
        let len2_sq = dx2 * dx2 + dy2 * dy2;
        if len1_sq < 1e-12 || len2_sq < 1e-12 {
            return vec![0.0];
        }

        let anchor_dir = dy1.atan2(dx1);
        let other_dir = dy2.atan2(dx2);
        let current = normalize_angle(anchor_dir - other_dir);
        let target = self.value_deg.to_radians();
        let mut diff = current - target;
        while diff > std::f64::consts::PI {
            diff -= 2.0 * std::f64::consts::PI;
        }
        while diff <= -std::f64::consts::PI {
            diff += 2.0 * std::f64::consts::PI;
        }
        vec![diff * ANGLE_WEIGHT]
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        let (Some(p1), Some(p2), Some(p3), Some(p4)) = (
            model.get_point(self.p1),
            model.get_point(self.p2),
            model.get_point(self.p3),
            model.get_point(self.p4),
        ) else {
            return false;
        };

        let intersection = get_line_line_intersection(
            Point::new(p1.x, p1.y),
            Point::new(p2.x, p2.y),
            Point::new(p3.x, p3.y),
            Point::new(p4.x, p4.y),
        );
        let (ix, iy) = match intersection {
            Some(point) => (point.x, point.y),
            None => (
                (p1.x + p2.x + p3.x + p4.x) / 4.0,
                (p1.y + p2.y + p3.y + p4.y) / 4.0,
            ),
        };
        let (sx, sy) = to_screen((ix, iy));

        let far1 = self.far_point(self.p1, self.p2, self.e1_far, ix, iy, model);
        let far2 = self.far_point(self.p3, self.p4, self.e2_far, ix, iy, model);
        let far1_screen = to_screen(far1);
        let far2_screen = to_screen(far2);

        let anchor_ang = (far1_screen.1 - sy).atan2(far1_screen.0 - sx);
        let other_ang = (far2_screen.1 - sy).atan2(far2_screen.0 - sx);

        let dist = (mx - sx).hypot(my - sy);
        if (dist - ARC_RADIUS).abs() > threshold {
            return false;
        }

        let click_ang = (my - sy).atan2(mx - sx);

        let ccw_diff = normalize_angle(other_ang - anchor_ang);

        let click_from_anchor = normalize_angle(click_ang - anchor_ang);

        click_from_anchor <= ccw_diff
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        let (Some(p1), Some(p2), Some(p3), Some(p4)) = (
            context.get_point(self.p1),
            context.get_point(self.p2),
            context.get_point(self.p3),
            context.get_point(self.p4),
        ) else {
            return;
        };
        let intersection = get_line_line_intersection(
            Point::new(p1.x, p1.y),
            Point::new(p2.x, p2.y),
            Point::new(p3.x, p3.y),
            Point::new(p4.x, p4.y),
        );
        let (ix, iy) = match intersection {
            Some(point) => (point.x, point.y),
            None => (
                (p1.x + p2.x + p3.x + p4.x) / 4.0,
                (p1.y + p2.y + p3.y + p4.y) / 4.0,
            ),
        };
        let (sx, sy) = to_screen((ix, iy));

        let far1 =
            draw_far_point(context, self.p1, self.p2, self.e1_far, ix, iy);
        let far2 =
            draw_far_point(context, self.p3, self.p4, self.e2_far, ix, iy);
        let far1_screen = to_screen(far1);
        let far2_screen = to_screen(far2);
        let anchor_ang = (far1_screen.1 - sy).atan2(far1_screen.0 - sx);
        let other_ang = (far2_screen.1 - sy).atan2(far2_screen.0 - sx);

        ctx.save();
        ctx.set_line_width(1.5);
        ctx.arc(sx, sy, ARC_RADIUS, anchor_ang, other_ang);
        if style.is_selected {
            draw_selection_underlay(ctx);
        }
        draw_conflict_underlay(ctx, style);
        draw_status_color(ctx, style);
        ctx.stroke();

        let label = format!("{}°", format_value(self.value_deg));
        let ccw_diff = normalize_angle(other_ang - anchor_ang);
        let mid = anchor_ang + ccw_diff / 2.0;
        let label_x = sx + mid.cos() * LABEL_RADIUS;
        let label_y = sy + mid.sin() * LABEL_RADIUS;
        draw_label(
            ctx,
            &label,
            label_x,
            label_y,
            style,
            screen_scale(to_screen),
        );
        ctx.restore();
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let mut entries = Vec::new();
        let mut seen: Vec<u64> = Vec::new();
        for pid in [self.p1, self.p2, self.p3, self.p4] {
            if seen.contains(&pid) {
                continue;
            }
            seen.push(pid);
            let (orig_x, orig_y) = reg.xy(pid);

            reg.set_xy(pid, orig_x + GRAD_FD_EPS, orig_y);
            let err_x_plus = self.error(reg)[0];
            reg.set_xy(pid, orig_x - GRAD_FD_EPS, orig_y);
            let err_x_minus = self.error(reg)[0];
            let gx = (err_x_plus - err_x_minus) / (2.0 * GRAD_FD_EPS);

            reg.set_xy(pid, orig_x, orig_y + GRAD_FD_EPS);
            let err_y_plus = self.error(reg)[0];
            reg.set_xy(pid, orig_x, orig_y - GRAD_FD_EPS);
            let err_y_minus = self.error(reg)[0];
            let gy = (err_y_plus - err_y_minus) / (2.0 * GRAD_FD_EPS);

            reg.set_xy(pid, orig_x, orig_y);
            entries.push((pid, 0, gx, gy));
        }
        entries
    }
}

impl AngleConstraint {
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
            "type": "AngleConstraint",
            "user_visible": self.user_visible,
        });
        data["p1_idx"] = json!(self.p1);
        data["p2_idx"] = json!(self.p2);
        data["p3_idx"] = json!(self.p3);
        data["p4_idx"] = json!(self.p4);
        data["value"] = json!(self.value_deg);
        data["e1_far_idx"] = json!(self.e1_far);
        data["e2_far_idx"] = json!(self.e2_far);
        if let Some(expression) = &self.expression {
            data["expression"] = json!(expression);
        }
        data
    }
}

/// Resolves the far endpoint for the drawn arc through the draw
/// context, mirroring the reference far-point selection.
fn draw_far_point(
    context: &dyn DrawContext,
    first: u64,
    second: u64,
    far: Option<u64>,
    ix: f64,
    iy: f64,
) -> (f64, f64) {
    if let Some(id) = far {
        if id == first || id == second {
            if let Some(point) = context.get_point(id) {
                return (point.x, point.y);
            }
        }
    }
    let (Some(a), Some(b)) =
        (context.get_point(first), context.get_point(second))
    else {
        return (0.0, 0.0);
    };
    let d1 = (a.x - ix).powi(2) + (a.y - iy).powi(2);
    let d2 = (b.x - ix).powi(2) + (b.y - iy).powi(2);
    if d1 > d2 {
        (a.x, a.y)
    } else {
        (b.x, b.y)
    }
}
