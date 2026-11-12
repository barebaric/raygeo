//! Aspect-ratio constraint between two segment lengths.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_status_color, CairoLike, ConstraintStatus, DrawStyle, ICON_SIZE,
};
use crate::sketch::entities::point::SketchPoint;
use crate::sketch::registry::EntityRegistry;

/// Enforces `dist(p1, p2) == ratio * dist(p3, p4)`.
#[derive(Clone, Debug, PartialEq)]
pub struct AspectRatioConstraint {
    /// First point of the reference segment.
    pub p1: u64,
    /// Second point of the reference segment.
    pub p2: u64,
    /// First point of the scaled segment.
    pub p3: u64,
    /// Second point of the scaled segment.
    pub p4: u64,
    /// Resolved ratio value.
    pub ratio: f64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl AspectRatioConstraint {
    /// Creates an aspect-ratio constraint with a resolved ratio.
    pub fn new(p1: u64, p2: u64, p3: u64, p4: u64, ratio: f64) -> Self {
        AspectRatioConstraint {
            p1,
            p2,
            p3,
            p4,
            ratio,
            user_visible: true,
        }
    }
}

impl SketchConstraint for AspectRatioConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.p1, self.p2, self.p3, self.p4]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        let dist1 = (x2 - x1).hypot(y2 - y1);
        let (x3, y3) = reg.xy(self.p3);
        let (x4, y4) = reg.xy(self.p4);
        let dist2 = (x4 - x3).hypot(y4 - y3);
        if dist2 < 1e-9 {
            return vec![dist1];
        }
        vec![dist1 - dist2 * self.ratio]
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

        let icon_pos = icon_pos(
            (self.p1, self.p2),
            (self.p3, self.p4),
            (&p1, &p2, &p3, &p4),
            to_screen,
        );
        let Some((cx, cy)) = icon_pos else {
            return false;
        };
        (mx - cx).hypot(my - cy) < threshold
    }

    fn draw(
        &self,
        registry: &EntityRegistry,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        let (Some(p1), Some(p2), Some(p3), Some(p4)) = (
            registry.get_point(self.p1),
            registry.get_point(self.p2),
            registry.get_point(self.p3),
            registry.get_point(self.p4),
        ) else {
            return;
        };
        let Some((cx, cy)) = icon_pos(
            (self.p1, self.p2),
            (self.p3, self.p4),
            (&p1, &p2, &p3, &p4),
            to_screen,
        ) else {
            return;
        };

        let hs = ICON_SIZE / 2.0;
        let glow_radius = hs + 4.0;

        ctx.save();
        if style.is_selected {
            ctx.set_source_rgba(0.2, 0.6, 1.0, 0.4);
            ctx.arc(cx, cy, glow_radius, 0.0, 2.0 * std::f64::consts::PI);
            ctx.fill();
        }
        if style.status == ConstraintStatus::Conflicting {
            ctx.set_source_rgba(1.0, 0.2, 0.2, 0.5);
            ctx.arc(cx, cy, glow_radius, 0.0, 2.0 * std::f64::consts::PI);
            ctx.fill();
        }

        // Corner brackets, translated to the icon anchor; the
        // reference's round cap/join apply at render time.
        ctx.set_line_width(2.0);
        ctx.set_line_cap_round();
        ctx.set_line_join_round();

        // Top-right corner bracket (L-shape pointing into the corner)
        let (x1, y1) = (cx + hs * 0.4, cy + hs);
        let (x2, y2) = (cx + hs, cy + hs);
        let (x3, y3) = (cx + hs, cy + hs * 0.4);
        ctx.move_to(x1, y1);
        ctx.line_to(x2, y2);
        ctx.line_to(x3, y3);

        // Bottom-left corner bracket
        let (x1, y1) = (cx - hs * 0.4, cy - hs);
        let (x2, y2) = (cx - hs, cy - hs);
        let (x3, y3) = (cx - hs, cy - hs * 0.4);
        ctx.move_to(x1, y1);
        ctx.line_to(x2, y2);
        ctx.line_to(x3, y3);

        draw_status_color(ctx, style);
        ctx.stroke();
        ctx.restore();
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        let dx1 = x2 - x1;
        let dy1 = y2 - y1;
        let dist1 = dx1.hypot(dy1);
        let (x3, y3) = reg.xy(self.p3);
        let (x4, y4) = reg.xy(self.p4);
        let dx2 = x4 - x3;
        let dy2 = y4 - y3;
        let dist2 = dx2.hypot(dy2);

        let mut entries = Vec::new();
        if dist1 > 1e-9 {
            let (ux, uy) = (dx1 / dist1, dy1 / dist1);
            entries.push((self.p1, 0, -ux, -uy));
            entries.push((self.p2, 0, ux, uy));
        }
        if dist2 > 1e-9 {
            let (ux, uy) = (dx2 / dist2, dy2 / dist2);
            entries.push((self.p3, 0, self.ratio * ux, self.ratio * uy));
            entries.push((self.p4, 0, -self.ratio * ux, -self.ratio * uy));
        }
        entries
    }
}

/// Screen position of the constraint icon: near the shared junction
/// point along the external angle bisector when the segments adjoin,
/// otherwise at the center of the two segment midpoints.
fn icon_pos(
    ids1: (u64, u64),
    ids2: (u64, u64),
    points: (&SketchPoint, &SketchPoint, &SketchPoint, &SketchPoint),
    to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
) -> Option<(f64, f64)> {
    let (p1, p2, p3, p4) = points;
    let mut junction_ids: Vec<u64> = Vec::new();
    for id in [ids1.0, ids1.1] {
        if (id == ids2.0 || id == ids2.1) && !junction_ids.contains(&id) {
            junction_ids.push(id);
        }
    }

    if junction_ids.len() == 1 {
        let junction_id = junction_ids[0];
        let p_junc = if ids1.0 == junction_id { p1 } else { p2 };
        let p_other1 = if ids1.1 == junction_id { p1 } else { p2 };
        let p_other2 = if ids2.1 == junction_id { p3 } else { p4 };

        let s_junc = to_screen((p_junc.x, p_junc.y));
        let s_other1 = to_screen((p_other1.x, p_other1.y));
        let s_other2 = to_screen((p_other2.x, p_other2.y));

        let sv1 = (s_other1.0 - s_junc.0, s_other1.1 - s_junc.1);
        let sv2 = (s_other2.0 - s_junc.0, s_other2.1 - s_junc.1);
        let slen1 = sv1.0.hypot(sv1.1);
        let slen2 = sv2.0.hypot(sv2.1);

        if slen1 < 1e-9 || slen2 < 1e-9 {
            return Some(s_junc);
        }

        let su1 = (sv1.0 / slen1, sv1.1 / slen1);
        let su2 = (sv2.0 / slen2, sv2.1 / slen2);

        let mut s_bisector = (-(su1.0 + su2.0), -(su1.1 + su2.1));
        let mut len_sb = s_bisector.0.hypot(s_bisector.1);

        if len_sb < 1e-9 {
            s_bisector = (-su1.1, su1.0);
            len_sb = 1.0;
        }

        let su_bisector = (s_bisector.0 / len_sb, s_bisector.1 / len_sb);

        let offset = 18.0;
        return Some((
            s_junc.0 + offset * su_bisector.0,
            s_junc.1 + offset * su_bisector.1,
        ));
    }

    let m1_x = (p1.x + p2.x) / 2.0;
    let m1_y = (p1.y + p2.y) / 2.0;
    let m2_x = (p3.x + p4.x) / 2.0;
    let m2_y = (p3.y + p4.y) / 2.0;
    Some(to_screen(((m1_x + m2_x) / 2.0, (m1_y + m2_y) / 2.0)))
}

impl AspectRatioConstraint {
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
            "type": "AspectRatioConstraint",
            "user_visible": self.user_visible,
        });
        data["p1"] = json!(self.p1);
        data["p2"] = json!(self.p2);
        data["p3"] = json!(self.p3);
        data["p4"] = json!(self.p4);
        data["ratio"] = json!(self.ratio);
        data
    }
}
