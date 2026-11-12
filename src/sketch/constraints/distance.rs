//! Distance constraint: fixed distance between two points.

use std::collections::HashSet;

use serde_json::{json, Value};

use crate::geo::shape::line::get_line_segment_closest_point;
use crate::geo::types::Point;
use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};
use crate::sketch::draw::{
    draw_conflict_underlay, draw_label, draw_status_color, format_value,
    screen_scale, CairoLike, DrawContext, DrawStyle,
};
use crate::sketch::registry::EntityRegistry;

/// Enforces a fixed distance between two points.
#[derive(Clone, Debug, PartialEq)]
pub struct DistanceConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
    /// Resolved distance value.
    pub value: f64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Stored expression string (evaluated caller-side).
    pub expression: Option<String>,
}

impl DistanceConstraint {
    /// Creates a distance constraint with a resolved value.
    pub fn new(p1: u64, p2: u64, value: f64) -> Self {
        DistanceConstraint {
            user_visible: true,
            expression: None,
            p1,
            p2,
            value,
        }
    }
}

impl SketchConstraint for DistanceConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.p1, self.p2]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        let dist = (x2 - x1).hypot(y2 - y1);
        vec![dist - self.value]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        let dx = x2 - x1;
        let dy = y2 - y1;
        let dist = dx.hypot(dy);
        let (ux, uy) = if dist > 1e-9 {
            (dx / dist, dy / dist)
        } else {
            (1.0, 0.0)
        };
        vec![(self.p1, 0, -ux, -uy), (self.p2, 0, ux, uy)]
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

        let mut has_geometry = false;
        let endpoints = HashSet::from([self.p1, self.p2]);
        for entity_id in model.entity_ids() {
            if model.entity_type(entity_id) != Some("line") {
                continue;
            }
            if let Some(ids) = model.entity_point_ids(entity_id) {
                if ids.iter().copied().collect::<HashSet<u64>>() == endpoints {
                    has_geometry = true;
                    break;
                }
            }
        }

        if !has_geometry {
            let s1 = to_screen((p1.x, p1.y));
            let s2 = to_screen((p2.x, p2.y));
            let (_, _, dist_sq) = get_line_segment_closest_point(
                Point::new(s1.0, s1.1),
                Point::new(s2.0, s2.1),
                mx,
                my,
            );
            if dist_sq < threshold * threshold {
                return true;
            }
        }

        let s1 = to_screen((p1.x, p1.y));
        let s2 = to_screen((p2.x, p2.y));
        let label_sx = (s1.0 + s2.0) / 2.0;
        let label_sy = (s1.1 + s2.1) / 2.0;

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
        let mx = (s1.0 + s2.0) / 2.0;
        let my = (s1.1 + s2.1) / 2.0;

        ctx.save();
        draw_label(
            ctx,
            &format_value(self.value),
            mx,
            my,
            style,
            screen_scale(to_screen),
        );

        let mut has_geometry = false;
        let endpoints = HashSet::from([self.p1, self.p2]);
        for entity_id in context.entity_ids() {
            if context.entity_type(entity_id).as_deref() != Some("line") {
                continue;
            }
            if let Some(ids) = context.entity_point_ids(entity_id) {
                if ids.iter().copied().collect::<HashSet<u64>>() == endpoints {
                    has_geometry = true;
                    break;
                }
            }
        }

        if !has_geometry {
            ctx.set_line_width(1.0);
            ctx.set_dash(4.0, 4.0);
            ctx.move_to(s1.0, s1.1);
            ctx.line_to(s2.0, s2.1);
            draw_conflict_underlay(ctx, style);
            draw_status_color(ctx, style);
            ctx.stroke();
        }
        ctx.restore();
    }
}

impl DistanceConstraint {
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
            "type": "DistanceConstraint",
            "user_visible": self.user_visible,
        });
        data["p1"] = json!(self.p1);
        data["p2"] = json!(self.p2);
        data["value"] = json!(self.value);
        if let Some(expression) = &self.expression {
            data["expression"] = json!(expression);
        }
        data
    }
}
