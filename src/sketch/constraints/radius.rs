//! Radius constraint: radius of a circle or arc.
//!
//! References the constrained entity by id and resolves it through
//! the point context at residual time, mirroring the reference
//! model's entity-reference design.

use serde_json::{json, Value};

use crate::geo::math::unit_or_zero;
use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_conflict_underlay, draw_label, draw_status_color, format_value,
    screen_scale, CairoLike, DrawContext, DrawStyle, LABEL_DIST_SCREEN,
    LABEL_HIT_SIZE, LABEL_PAD,
};
use crate::sketch::entities::arc::Arc;
use crate::sketch::entities::entity::EntityKind;

/// Enforces the radius of a circle or arc.
#[derive(Clone, Debug, PartialEq)]
pub struct RadiusConstraint {
    /// The constrained entity (circle or arc).
    pub entity_id: u64,
    /// Target radius value.
    pub value: f64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Stored expression string (evaluated caller-side).
    pub expression: Option<String>,
}

/// Resolves a radius-bearing entity to its `(center, radius_point)`
/// pair.  Arcs use the start point as the radius-bearing point.
fn resolve_radius_points(
    ctx: &dyn PointContext,
    entity_id: u64,
) -> Option<(u64, u64)> {
    let entity = ctx.get_entity(entity_id)?;
    if !entity.is_radius_entity() {
        return None;
    }
    match entity.kind() {
        EntityKind::Circle | EntityKind::Arc => {
            entity.center_radius_point_ids()
        }
        _ => None,
    }
}

impl RadiusConstraint {
    /// Creates a radius constraint for the given entity.
    pub fn new(entity_id: u64, value: f64) -> Self {
        RadiusConstraint {
            entity_id,
            value,
            user_visible: true,
            expression: None,
        }
    }
}

impl SketchConstraint for RadiusConstraint {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_json(&self) -> Value {
        RadiusConstraint::to_json(self)
    }

    fn referenced_points(&self, ctx: &dyn PointContext) -> Vec<u64> {
        match resolve_radius_points(ctx, self.entity_id) {
            Some((center, radius_pt)) => vec![center, radius_pt],
            None => vec![],
        }
    }

    fn residual_count(&self, _ctx: &dyn PointContext) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        match resolve_radius_points(reg, self.entity_id) {
            Some((center, radius_pt)) => {
                let (cx, cy) = reg.xy(center);
                let (px, py) = reg.xy(radius_pt);
                vec![(px - cx).hypot(py - cy) - self.value]
            }
            None => vec![0.0],
        }
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        match resolve_radius_points(reg, self.entity_id) {
            Some((center, radius_pt)) => {
                let (cx, cy) = reg.xy(center);
                let (px, py) = reg.xy(radius_pt);
                let dx = px - cx;
                let dy = py - cy;
                let (ux, uy) = unit_or_zero(dx, dy);
                vec![(radius_pt, 0, ux, uy), (center, 0, -ux, -uy)]
            }
            None => vec![],
        }
    }

    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        _threshold: f64,
        model: &dyn PointContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
    ) -> bool {
        let Some((center_id, radius_pt_id)) =
            resolve_radius_points(model, self.entity_id)
        else {
            return false;
        };
        let (Some(center), Some(radius_pt)) =
            (model.get_point(center_id), model.get_point(radius_pt_id))
        else {
            return false;
        };
        let radius = (radius_pt.x - center.x).hypot(radius_pt.y - center.y);
        if radius == 0.0 {
            return false;
        }

        // The label angles along the arc midpoint for arcs and along
        // the radius point direction for circles, mirroring the
        // reference implementation.
        let arc_mid_angle = model.get_entity(self.entity_id).and_then(|e| {
            let arc = e.as_any().downcast_ref::<Arc>()?;
            let midpoint = arc.get_midpoint(model)?;
            Some((midpoint.1 - center.y).atan2(midpoint.0 - center.x))
        });
        let mid_angle = arc_mid_angle.unwrap_or_else(|| {
            (radius_pt.y - center.y).atan2(radius_pt.x - center.x)
        });

        let p0 = to_screen((0.0, 0.0));
        let p1 = to_screen((1.0, 0.0));
        let mut scale = (p1.0 - p0.0).hypot(p1.1 - p0.1);
        if scale < 1e-9 {
            scale = 1.0;
        }

        let label_dist = radius + LABEL_DIST_SCREEN / scale;
        let (label_sx, label_sy) = to_screen((
            center.x + label_dist * mid_angle.cos(),
            center.y + label_dist * mid_angle.sin(),
        ));

        let half_w = LABEL_HIT_SIZE / 2.0;
        let half_h = half_w;

        let x_min = label_sx - half_w - LABEL_PAD;
        let x_max = label_sx + half_w + LABEL_PAD;
        let y_min = label_sy - half_h - LABEL_PAD;
        let y_max = label_sy + half_h + LABEL_PAD;

        x_min <= mx && mx <= x_max && y_min <= my && my <= y_max
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        let Some(point_ids) = context.entity_point_ids(self.entity_id) else {
            return;
        };
        if point_ids.len() < 2 {
            return;
        }
        let (Some(center), Some(radius_pt)) = (
            context.get_point(point_ids[0]),
            context.get_point(point_ids[1]),
        ) else {
            return;
        };
        let radius = (radius_pt.x - center.x).hypot(radius_pt.y - center.y);
        if radius == 0.0 {
            return;
        }
        let scale = screen_scale(to_screen);
        let mid_angle = match context.entity_arc_midpoint(self.entity_id) {
            Some((mx, my)) => (my - center.y).atan2(mx - center.x),
            None => (radius_pt.y - center.y).atan2(radius_pt.x - center.x),
        };
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
    /// Serializes to the reference constraint dictionary shape.
    pub fn to_json(&self) -> Value {
        let mut data = json!({
            "type": "RadiusConstraint",
            "entity_id": self.entity_id,
            "value": self.value,
            "user_visible": self.user_visible,
        });
        if let Some(expression) = &self.expression {
            data["expression"] = json!(expression);
        }
        data
    }
}
