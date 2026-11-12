//! Point-on-line constraint: a point on a line or circle circumference.

use crate::geo::shape::line::get_point_line_signed_distance;
use crate::geo::types::Point;
use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_point_marker, CairoLike, DrawContext, DrawStyle,
};
use crate::sketch::registry::EntityRegistry;

/// The resolved target geometry, mirroring the reference constraint's
/// entity cases: an infinite line, or a circle/arc circumference.
#[derive(Clone, Debug, PartialEq)]
pub enum PointOnTarget {
    /// The infinite line through two points.
    Line { l1: u64, l2: u64 },
    /// The circle/arc through center and radius-bearing point.
    RadiusCurve { center: u64, radius_point: u64 },
}

/// Enforces a point lies on a line or circle/arc circumference.
#[derive(Clone, Debug, PartialEq)]
pub struct PointOnLineConstraint {
    /// The constrained point.
    pub point: u64,
    /// Resolved target geometry.
    pub target: PointOnTarget,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl PointOnLineConstraint {
    /// Creates a point-on-line/curve constraint.
    pub fn new(point: u64, target: PointOnTarget) -> Self {
        PointOnLineConstraint {
            user_visible: true,
            point,
            target,
        }
    }
}

impl SketchConstraint for PointOnLineConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        let mut ids = vec![self.point];
        match &self.target {
            PointOnTarget::Line { l1, l2 } => {
                ids.push(*l1);
                ids.push(*l2);
            }
            PointOnTarget::RadiusCurve {
                center,
                radius_point,
            } => {
                ids.push(*center);
                ids.push(*radius_point);
            }
        }
        ids
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        match &self.target {
            PointOnTarget::Line { l1, l2 } => {
                let signed = get_point_line_signed_distance(
                    Point::new(reg.xy(self.point).0, reg.xy(self.point).1),
                    Point::new(reg.xy(*l1).0, reg.xy(*l1).1),
                    Point::new(reg.xy(*l2).0, reg.xy(*l2).1),
                );
                vec![signed]
            }
            PointOnTarget::RadiusCurve {
                center,
                radius_point,
            } => {
                let (px, py) = reg.xy(self.point);
                let (cx, cy) = reg.xy(*center);
                let (rx, ry) = reg.xy(*radius_point);
                let radius = (rx - cx).hypot(ry - cy);
                vec![(px - cx).hypot(py - cy) - radius]
            }
        }
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let (px, py) = reg.xy(self.point);
        let mut entries = Vec::new();
        match &self.target {
            PointOnTarget::Line { l1, l2 } => {
                let (lx1, ly1) = reg.xy(*l1);
                let (lx2, ly2) = reg.xy(*l2);
                let dx = lx2 - lx1;
                let dy = ly2 - ly1;
                let len_sq = dx * dx + dy * dy;

                if len_sq < 1e-18 {
                    let dpx = px - lx1;
                    let dpy = py - ly1;
                    let dist = dpx.hypot(dpy);
                    if dist < 1e-9 {
                        return entries;
                    }
                    let ux = dpx / dist;
                    let uy = dpy / dist;
                    entries.push((self.point, 0, ux, uy));
                    entries.push((*l1, 0, -ux, -uy));
                    return entries;
                }

                let length = len_sq.sqrt();
                let inv_len = 1.0 / length;
                let inv_len_sq = inv_len * inv_len;
                let nx = -dy * inv_len;
                let ny = dx * inv_len;
                entries.push((self.point, 0, nx, ny));

                let err = get_point_line_signed_distance(
                    Point::new(px, py),
                    Point::new(lx1, ly1),
                    Point::new(lx2, ly2),
                );
                let grad_l2_x = (py - ly1) * inv_len - err * dx * inv_len_sq;
                let grad_l2_y = -(px - lx1) * inv_len - err * dy * inv_len_sq;
                entries.push((*l2, 0, grad_l2_x, grad_l2_y));
                entries.push((*l1, 0, -nx - grad_l2_x, -ny - grad_l2_y));
            }
            PointOnTarget::RadiusCurve {
                center,
                radius_point,
            } => {
                let (cx, cy) = reg.xy(*center);
                let dist_pc = (px - cx).hypot(py - cy);
                let (ux, uy) = if dist_pc > 1e-9 {
                    ((px - cx) / dist_pc, (py - cy) / dist_pc)
                } else {
                    (0.0, 0.0)
                };
                entries.push((self.point, 0, ux, uy));

                let (rx, ry) = reg.xy(*radius_point);
                let radius = (rx - cx).hypot(ry - cy);
                let (rux, ruy) = if radius > 1e-9 {
                    ((rx - cx) / radius, (ry - cy) / radius)
                } else {
                    (0.0, 0.0)
                };
                entries.push((*center, 0, rux - ux, ruy - uy));
                entries.push((*radius_point, 0, -rux, -ruy));
            }
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
        let Some(pt) = model.get_point(self.point) else {
            return false;
        };
        let s_pt = to_screen((pt.x, pt.y));
        (mx - s_pt.0).hypot(my - s_pt.1) < threshold
    }

    fn draw(
        &self,
        context: &dyn DrawContext,
        to_screen: &dyn Fn((f64, f64)) -> (f64, f64),
        ctx: &dyn CairoLike,
        style: &DrawStyle,
    ) {
        // Hide the constraint if its point is part of a text box.
        if context.point_in_text_box_frame(self.point) {
            return;
        }
        let Some(pt) = context.get_point(self.point) else {
            return;
        };
        let (sx, sy) = to_screen((pt.x, pt.y));
        draw_point_marker(ctx, sx, sy, style);
    }
}

impl PointOnLineConstraint {
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
        let data = json!({
            "type": "PointOnLineConstraint",
            "user_visible": self.user_visible,
        });
        let _ = (self.point, &self.target);
        data
    }
}
