//! Rotational constraint: a point rotated about a center.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces `p2` is `p1` rotated by `angle_rad` about `center`.
#[derive(Clone, Debug, PartialEq)]
pub struct RotationalConstraint {
    /// Rotation center.
    pub center: u64,
    /// Source point.
    pub p1: u64,
    /// Rotated point.
    pub p2: u64,
    /// Resolved rotation angle in radians.
    pub angle_rad: f64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Stored expression string (evaluated caller-side).
    pub expression: Option<String>,
}

impl RotationalConstraint {
    /// Creates a rotational constraint with a resolved angle.
    pub fn new(center: u64, p1: u64, p2: u64, angle_rad: f64) -> Self {
        RotationalConstraint {
            center,
            p1,
            p2,
            angle_rad,
            user_visible: true,
            expression: None,
        }
    }
}

impl SketchConstraint for RotationalConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.center, self.p1, self.p2]
    }

    fn residual_count(&self) -> usize {
        2
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (cx, cy) = reg.xy(self.center);
        let (sx, sy) = reg.xy(self.p1);
        let (tx, ty) = reg.xy(self.p2);
        let ca = self.angle_rad.cos();
        let sa = self.angle_rad.sin();
        let dx = sx - cx;
        let dy = sy - cy;
        vec![tx - (cx + ca * dx - sa * dy), ty - (cy + sa * dx + ca * dy)]
    }

    fn gradient(&self, _reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let ca = self.angle_rad.cos();
        let sa = self.angle_rad.sin();
        vec![
            (self.p2, 0, 1.0, 0.0),
            (self.p2, 1, 0.0, 1.0),
            (self.p1, 0, -ca, sa),
            (self.p1, 1, -sa, -ca),
            (self.center, 0, -1.0 + ca, -sa),
            (self.center, 1, sa, -1.0 + ca),
        ]
    }
}

impl RotationalConstraint {
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
            "type": "RotationalConstraint",
            "user_visible": self.user_visible,
        });
        data["center"] = json!(self.center);
        data["p1"] = json!(self.p1);
        data["p2"] = json!(self.p2);
        data["value"] = json!(self.angle_rad);
        if let Some(expression) = &self.expression {
            data["expression"] = json!(expression);
        }
        data
    }
}
