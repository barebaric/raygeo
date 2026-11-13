//! Equal-distance constraint between two point pairs.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_equal_symbol, screen_scale, CairoLike, DrawContext, DrawStyle,
    EQUAL_OFFSET_SCREEN,
};
use crate::sketch::registry::EntityRegistry;

use std::f64::consts::FRAC_PI_2;

/// Enforces `dist(p1, p2) == dist(p3, p4)`.
#[derive(Clone, Debug, PartialEq)]
pub struct EqualDistanceConstraint {
    /// First point of the first pair.
    pub p1: u64,
    /// Second point of the first pair.
    pub p2: u64,
    /// First point of the second pair.
    pub p3: u64,
    /// Second point of the second pair.
    pub p4: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl EqualDistanceConstraint {
    /// Creates an equal-distance constraint.
    pub fn new(p1: u64, p2: u64, p3: u64, p4: u64) -> Self {
        EqualDistanceConstraint {
            user_visible: true,
            p1,
            p2,
            p3,
            p4,
        }
    }
}

impl SketchConstraint for EqualDistanceConstraint {
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
        vec![dist1 - dist2]
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
            entries.push((self.p3, 0, ux, uy));
            entries.push((self.p4, 0, -ux, -uy));
        }
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
        let (Some(pa), Some(pb), Some(pc), Some(pd)) = (
            model.get_point(self.p1),
            model.get_point(self.p2),
            model.get_point(self.p3),
            model.get_point(self.p4),
        ) else {
            return false;
        };

        let sym_x = (pa.x + pb.x + pc.x + pd.x) / 4.0;
        let sym_y = (pa.y + pb.y + pc.y + pd.y) / 4.0;

        let tangent_angle = (pb.y - pa.y).atan2(pb.x - pa.x);
        let normal_angle = tangent_angle - FRAC_PI_2;

        let p0 = to_screen((0.0, 0.0));
        let p1 = to_screen((1.0, 0.0));
        let mut scale = (p1.0 - p0.0).hypot(p1.1 - p0.1);
        if scale < 1e-9 {
            scale = 1.0;
        }

        let offset_dist_model = 15.0 / scale;
        let final_x = sym_x + offset_dist_model * normal_angle.cos();
        let final_y = sym_y + offset_dist_model * normal_angle.sin();
        let (sym_sx, sym_sy) = to_screen((final_x, final_y));
        (mx - sym_sx).hypot(my - sym_sy) < threshold
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        let (Some(pa), Some(pb), Some(pc), Some(pd)) = (
            context.get_point(self.p1),
            context.get_point(self.p2),
            context.get_point(self.p3),
            context.get_point(self.p4),
        ) else {
            return;
        };

        let sym_x = (pa.x + pb.x + pc.x + pd.x) / 4.0;
        let sym_y = (pa.y + pb.y + pc.y + pd.y) / 4.0;

        let tangent_angle = (pb.y - pa.y).atan2(pb.x - pa.x);
        let normal_angle = tangent_angle - FRAC_PI_2;

        let scale = screen_scale(to_screen);
        let offset_dist_model = EQUAL_OFFSET_SCREEN / scale;
        let final_x = sym_x + offset_dist_model * normal_angle.cos();
        let final_y = sym_y + offset_dist_model * normal_angle.sin();
        let (sym_sx, sym_sy) = to_screen((final_x, final_y));
        draw_equal_symbol(ctx, sym_sx, sym_sy, style, screen_scale(to_screen));
    }
}

impl EqualDistanceConstraint {
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
            "type": "EqualDistanceConstraint",
            "user_visible": self.user_visible,
        });
        data["p1"] = json!(self.p1);
        data["p2"] = json!(self.p2);
        data["p3"] = json!(self.p3);
        data["p4"] = json!(self.p4);
        data
    }
}
