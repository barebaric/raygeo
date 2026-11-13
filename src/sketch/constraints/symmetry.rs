//! Symmetry constraint: two points mirrored about a point or a line.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_conflict_underlay, draw_selection_underlay, draw_status_color,
    draw_symmetry_arrows, CairoLike, DrawContext, DrawStyle,
};
use crate::sketch::registry::EntityRegistry;

/// The symmetry reference, mirroring the reference constraint's two
/// cases: point symmetry about a center, or line symmetry about an
/// axis line.
#[derive(Clone, Debug, PartialEq)]
pub enum SymmetryTarget {
    /// The center is the midpoint of `p1` and `p2`.
    Point { center: u64 },
    /// `p1` and `p2` are mirrored across the line `(l1, l2)`.
    Line { l1: u64, l2: u64 },
}

/// Enforces two points are symmetric about a point or a line.
#[derive(Clone, Debug, PartialEq)]
pub struct SymmetryConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
    /// Symmetry reference.
    pub target: SymmetryTarget,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl SymmetryConstraint {
    /// Creates a symmetry constraint.
    pub fn new(p1: u64, p2: u64, target: SymmetryTarget) -> Self {
        SymmetryConstraint {
            user_visible: true,
            p1,
            p2,
            target,
        }
    }
}

impl SketchConstraint for SymmetryConstraint {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn to_json(&self) -> Value {
        SymmetryConstraint::to_json(self)
    }

    fn referenced_points(&self) -> Vec<u64> {
        let mut ids = vec![self.p1, self.p2];
        match &self.target {
            SymmetryTarget::Point { center } => ids.push(*center),
            SymmetryTarget::Line { l1, l2 } => {
                ids.push(*l1);
                ids.push(*l2);
            }
        }
        ids
    }

    fn residual_count(&self) -> usize {
        2
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        match &self.target {
            SymmetryTarget::Point { center } => {
                let (cx, cy) = reg.xy(*center);
                vec![(x1 + x2) - 2.0 * cx, (y1 + y2) - 2.0 * cy]
            }
            SymmetryTarget::Line { l1, l2 } => {
                let (lx1, ly1) = reg.xy(*l1);
                let (lx2, ly2) = reg.xy(*l2);
                let dx_l = lx2 - lx1;
                let dy_l = ly2 - ly1;
                let dx_p = x2 - x1;
                let dy_p = y2 - y1;
                let err_perp = dx_p * dx_l + dy_p * dy_l;
                let mx = (x1 + x2) * 0.5;
                let my = (y1 + y2) * 0.5;
                let err_collinear = (mx - lx1) * dy_l - (my - ly1) * dx_l;
                vec![err_perp, err_collinear]
            }
        }
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        match &self.target {
            SymmetryTarget::Point { center } => vec![
                (self.p1, 0, 1.0, 0.0),
                (self.p1, 1, 0.0, 1.0),
                (self.p2, 0, 1.0, 0.0),
                (self.p2, 1, 0.0, 1.0),
                (*center, 0, -2.0, 0.0),
                (*center, 1, 0.0, -2.0),
            ],
            SymmetryTarget::Line { l1, l2 } => {
                let (x1, y1) = reg.xy(self.p1);
                let (x2, y2) = reg.xy(self.p2);
                let (lx1, ly1) = reg.xy(*l1);
                let (lx2, ly2) = reg.xy(*l2);
                let dxl = lx2 - lx1;
                let dyl = ly2 - ly1;
                let mx = (x1 + x2) * 0.5;
                let my = (y1 + y2) * 0.5;
                vec![
                    (self.p1, 0, -dxl, -dyl),
                    (self.p2, 0, dxl, dyl),
                    (*l1, 0, -(x2 - x1), -(y2 - y1)),
                    (*l2, 0, x2 - x1, y2 - y1),
                    (self.p1, 1, 0.5 * dyl, -0.5 * dxl),
                    (self.p2, 1, 0.5 * dyl, -0.5 * dxl),
                    (*l1, 1, my - ly1 - dyl, dxl - (mx - lx1)),
                    (*l2, 1, -(my - ly1), mx - lx1),
                ]
            }
        }
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
        let mid_x = (s1.0 + s2.0) / 2.0;
        let mid_y = (s1.1 + s2.1) / 2.0;
        let angle = (s2.1 - s1.1).atan2(s2.0 - s1.0);
        let offset = 12.0;
        let lx = mid_x - offset * angle.cos();
        let ly = mid_y - offset * angle.sin();
        let rx = mid_x + offset * angle.cos();
        let ry = mid_y + offset * angle.sin();
        if (mx - lx).hypot(my - ly) < threshold {
            return true;
        }
        (mx - rx).hypot(my - ry) < threshold
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

        ctx.save();
        ctx.set_line_width(1.5);
        draw_symmetry_arrows(ctx, s1, s2);
        if style.is_selected {
            draw_selection_underlay(ctx);
        }
        draw_conflict_underlay(ctx, style);
        draw_status_color(ctx, style);
        ctx.stroke();
        ctx.restore();
    }
}

impl SymmetryConstraint {
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
            "type": "SymmetryConstraint",
            "user_visible": self.user_visible,
        });
        data["p1"] = json!(self.p1);
        data["p2"] = json!(self.p2);
        data
    }
}
