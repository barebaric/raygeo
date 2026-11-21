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

/// Enforces a line is tangent to a circle/arc: the absolute signed
/// distance from the center to the line equals the radius.
///
/// References the line and the shape by id and resolves their
/// defining points through the point context at residual time,
/// mirroring the reference model's entity-reference design.
#[derive(Clone, Debug, PartialEq)]
pub struct TangentConstraint {
    /// The tangent line.
    pub line_id: u64,
    /// The tangent circle or arc.
    pub shape_id: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

/// Resolves a line/shape id pair to
/// `(line_p1, line_p2, center, radius_point)`.
fn resolve_tangent_points(
    ctx: &dyn PointContext,
    line_id: u64,
    shape_id: u64,
) -> Option<(u64, u64, u64, u64)> {
    let line = ctx.get_entity(line_id)?;
    if line.type_name() != "line" {
        return None;
    }
    let line_ids = line.get_point_ids();
    let shape = ctx.get_entity(shape_id)?;
    let shape_ids = shape.get_point_ids();
    match shape.type_name() {
        "circle" => {
            Some((line_ids[0], line_ids[1], shape_ids[0], shape_ids[1]))
        }
        "arc" => Some((line_ids[0], line_ids[1], shape_ids[2], shape_ids[0])),
        _ => None,
    }
}

impl TangentConstraint {
    /// Creates a tangent constraint between a line and a circle/arc.
    pub fn new(line_id: u64, shape_id: u64) -> Self {
        TangentConstraint {
            line_id,
            shape_id,
            user_visible: true,
        }
    }
}

impl SketchConstraint for TangentConstraint {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_json(&self) -> Value {
        TangentConstraint::to_json(self)
    }

    fn referenced_points(&self, ctx: &dyn PointContext) -> Vec<u64> {
        match resolve_tangent_points(ctx, self.line_id, self.shape_id) {
            Some((l1, l2, center, radius_pt)) => {
                vec![l1, l2, center, radius_pt]
            }
            None => vec![],
        }
    }

    fn residual_count(&self, _ctx: &dyn PointContext) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let Some((l1, l2, center, radius_pt)) =
            resolve_tangent_points(reg, self.line_id, self.shape_id)
        else {
            return vec![0.0];
        };
        let (cx, cy) = reg.xy(center);
        let (px, py) = reg.xy(radius_pt);
        let signed = get_point_line_signed_distance(
            crate::geo::types::Point::new(cx, cy),
            crate::geo::types::Point::new(reg.xy(l1).0, reg.xy(l1).1),
            crate::geo::types::Point::new(reg.xy(l2).0, reg.xy(l2).1),
        );
        vec![signed.abs() - (px - cx).hypot(py - cy)]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let Some((l1, l2, center, radius_pt)) =
            resolve_tangent_points(reg, self.line_id, self.shape_id)
        else {
            return vec![];
        };
        let (cx, cy) = reg.xy(center);
        let (px, py) = reg.xy(radius_pt);
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
        entries.push((radius_pt, 0, -rad_ux, -rad_uy));

        let (lx1, ly1) = reg.xy(l1);
        let (lx2, ly2) = reg.xy(l2);
        let line_dx = lx2 - lx1;
        let line_dy = ly2 - ly1;
        let len_sq = line_dx * line_dx + line_dy * line_dy;

        if len_sq < 1e-18 {
            // Degenerate line: gradient of the plain distance.
            let dist_c_l1 = (cx - lx1).hypot(cy - ly1);
            if dist_c_l1 < 1e-9 {
                entries.push((center, 0, rad_ux, rad_uy));
                return entries;
            }
            let ux = (cx - lx1) / dist_c_l1;
            let uy = (cy - ly1) / dist_c_l1;
            entries.push((center, 0, rad_ux + ux, rad_uy + uy));
            entries.push((l1, 0, -ux, -uy));
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
            center,
            0,
            rad_ux + sign * grad_pol_c_x,
            rad_uy + sign * grad_pol_c_y,
        ));
        entries.push((l1, 0, sign * grad_pol_l1_x, sign * grad_pol_l1_y));
        entries.push((l2, 0, sign * grad_pol_l2_x, sign * grad_pol_l2_y));
        entries
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        model: &dyn PointContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        let Some((l1, l2, center, _radius_pt)) =
            resolve_tangent_points(model, self.line_id, self.shape_id)
        else {
            return false;
        };
        let (Some(p1), Some(p2), Some(center)) = (
            model.get_point(l1),
            model.get_point(l2),
            model.get_point(center),
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
        let Some(line_ids) = context.entity_point_ids(self.line_id) else {
            return;
        };
        if context
            .entity_type(self.line_id)
            .map(|name| name.as_ref() != "line")
            .unwrap_or(true)
        {
            return;
        }
        let Some(shape_ids) = context.entity_point_ids(self.shape_id) else {
            return;
        };
        let (l1, l2) = (line_ids[0], line_ids[1]);
        let (center, _radius_pt) = match context
            .entity_type(self.shape_id)
            .map(|name| name.as_ref().to_owned())
        {
            Some(name) if name == "circle" => (shape_ids[0], shape_ids[1]),
            Some(name) if name == "arc" => (shape_ids[2], shape_ids[0]),
            _ => return,
        };
        let (Some(p1), Some(p2), Some(center)) = (
            context.get_point(l1),
            context.get_point(l2),
            context.get_point(center),
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
    /// Serializes to the reference constraint dictionary shape.
    pub fn to_json(&self) -> Value {
        json!({
            "type": "TangentConstraint",
            "line_id": self.line_id,
            "shape_id": self.shape_id,
            "user_visible": self.user_visible,
        })
    }
}
