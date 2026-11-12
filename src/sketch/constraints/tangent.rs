//! Tangent constraint: a line tangent to a circle or arc.

use crate::geo::shape::line::{
    get_line_closest_point, get_point_line_signed_distance,
};
use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_conflict_underlay, draw_selection_underlay, draw_status_color,
    CairoLike, DrawContext, DrawStyle, TANGENT_OFFSET, TANGENT_RADIUS,
};
use crate::sketch::registry::EntityRegistry;

/// Enforces a line is tangent to a circle/arc: the absolute signed
/// distance from the center to the line equals the radius.
#[derive(Clone, Debug, PartialEq)]
pub struct TangentConstraint {
    /// First point of the line.
    pub l1: u64,
    /// Second point of the line.
    pub l2: u64,
    /// Center point of the circle/arc.
    pub center: u64,
    /// Radius-bearing point (circle radius point or arc start).
    pub radius_point: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl TangentConstraint {
    /// Creates a tangent constraint between a line and a circle/arc.
    pub fn new(l1: u64, l2: u64, center: u64, radius_point: u64) -> Self {
        TangentConstraint {
            l1,
            l2,
            center,
            radius_point,
            user_visible: true,
        }
    }
}

impl SketchConstraint for TangentConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.l1, self.l2, self.center, self.radius_point]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (cx, cy) = reg.xy(self.center);
        let (px, py) = reg.xy(self.radius_point);
        let signed = get_point_line_signed_distance(
            crate::geo::types::Point::new(cx, cy),
            crate::geo::types::Point::new(reg.xy(self.l1).0, reg.xy(self.l1).1),
            crate::geo::types::Point::new(reg.xy(self.l2).0, reg.xy(self.l2).1),
        );
        vec![signed.abs() - (px - cx).hypot(py - cy)]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let (cx, cy) = reg.xy(self.center);
        let (px, py) = reg.xy(self.radius_point);
        let mut entries = Vec::new();

        // Part 1: gradient of the radius term.
        let rad_dx = px - cx;
        let rad_dy = py - cy;
        let radius = rad_dx.hypot(rad_dy);
        let (rad_ux, rad_uy) = if radius > 1e-9 {
            (rad_dx / radius, rad_dy / radius)
        } else {
            (0.0, 0.0)
        };
        entries.push((self.radius_point, 0, -rad_ux, -rad_uy));

        let (lx1, ly1) = reg.xy(self.l1);
        let (lx2, ly2) = reg.xy(self.l2);
        let line_dx = lx2 - lx1;
        let line_dy = ly2 - ly1;
        let len_sq = line_dx * line_dx + line_dy * line_dy;

        if len_sq < 1e-18 {
            // Degenerate line: gradient of the plain distance.
            let dist_c_l1 = (cx - lx1).hypot(cy - ly1);
            if dist_c_l1 < 1e-9 {
                entries.push((self.center, 0, rad_ux, rad_uy));
                return entries;
            }
            let ux = (cx - lx1) / dist_c_l1;
            let uy = (cy - ly1) / dist_c_l1;
            entries.push((self.center, 0, rad_ux + ux, rad_uy + uy));
            entries.push((self.l1, 0, -ux, -uy));
            return entries;
        }

        let length = len_sq.sqrt();
        let inv_len = 1.0 / length;
        let cross = line_dx * (cy - ly1) - line_dy * (cx - lx1);
        let sign = 1.0_f64.copysign(cross);

        // Part 2: gradient of the signed distance, combined with the
        // sign of the abs() term.
        let nx = -line_dy * inv_len;
        let ny = line_dx * inv_len;
        let pol_err = cross * inv_len;
        let inv_len_sq = inv_len * inv_len;

        let grad_pol_c_x = nx;
        let grad_pol_c_y = ny;
        let grad_pol_l2_x =
            (cy - ly1) * inv_len - pol_err * line_dx * inv_len_sq;
        let grad_pol_l2_y =
            -(cx - lx1) * inv_len - pol_err * line_dy * inv_len_sq;
        let grad_pol_l1_x = -grad_pol_c_x - grad_pol_l2_x;
        let grad_pol_l1_y = -grad_pol_c_y - grad_pol_l2_y;

        entries.push((
            self.center,
            0,
            rad_ux + sign * grad_pol_c_x,
            rad_uy + sign * grad_pol_c_y,
        ));
        entries.push((self.l1, 0, sign * grad_pol_l1_x, sign * grad_pol_l1_y));
        entries.push((self.l2, 0, sign * grad_pol_l2_x, sign * grad_pol_l2_y));
        entries
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        let (Some(p1), Some(p2), Some(center)) = (
            model.get_point(self.l1),
            model.get_point(self.l2),
            model.get_point(self.center),
        ) else {
            return false;
        };

        // Closest point on the infinite line from the center, in
        // model space.
        let tangent = get_line_closest_point(
            crate::geo::types::Point::new(p1.x, p1.y),
            crate::geo::types::Point::new(p2.x, p2.y),
            center.x,
            center.y,
        );

        // Convert to screen space first, then offset in screen pixels.
        let (sx_tangent, sy_tangent) = to_screen((tangent.x, tangent.y));
        let (sx_center, sy_center) = to_screen((center.x, center.y));

        let angle = (sy_tangent - sy_center).atan2(sx_tangent - sx_center);

        let offset = 15.0;
        let symbol_sx = sx_tangent + offset * angle.cos();
        let symbol_sy = sy_tangent + offset * angle.sin();

        (mx - symbol_sx).hypot(my - symbol_sy) < threshold
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        let (Some(p1), Some(p2), Some(center)) = (
            context.get_point(self.l1),
            context.get_point(self.l2),
            context.get_point(self.center),
        ) else {
            return;
        };

        // Closest point on the infinite line from the center, in
        // model space.
        let tangent = get_line_closest_point(
            crate::geo::types::Point::new(p1.x, p1.y),
            crate::geo::types::Point::new(p2.x, p2.y),
            center.x,
            center.y,
        );

        let (sx_tangent, sy_tangent) = to_screen((tangent.x, tangent.y));
        let (sx_center, sy_center) = to_screen((center.x, center.y));
        let angle = (sy_tangent - sy_center).atan2(sx_tangent - sx_center);
        let sx = sx_tangent + TANGENT_OFFSET * angle.cos();
        let sy = sy_tangent + TANGENT_OFFSET * angle.sin();

        let rotation = angle + std::f64::consts::FRAC_PI_2;
        let radius = TANGENT_RADIUS;

        ctx.save();
        ctx.set_line_width(1.5);
        ctx.translate(sx, sy);
        ctx.rotate(rotation);
        ctx.arc(
            0.0,
            -radius,
            radius,
            std::f64::consts::FRAC_PI_2 - std::f64::consts::FRAC_PI_3,
            std::f64::consts::FRAC_PI_2 + std::f64::consts::FRAC_PI_3,
        );
        ctx.move_to(-radius * 1.2, 0.0);
        ctx.line_to(radius * 1.2, 0.0);
        if style.is_selected {
            draw_selection_underlay(ctx);
        }
        draw_conflict_underlay(ctx, style);
        draw_status_color(ctx, style);
        ctx.stroke();
        ctx.restore();
    }
}

impl TangentConstraint {
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
            "type": "TangentConstraint",
            "user_visible": self.user_visible,
        });
        data["l1_idx"] = json!(self.l1);
        data["l2_idx"] = json!(self.l2);
        data["center_idx"] = json!(self.center);
        data["radius_pt_idx"] = json!(self.radius_point);
        data
    }
}
