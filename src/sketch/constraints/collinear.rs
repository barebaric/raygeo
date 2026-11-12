//! Collinear constraint: three points on one line.

use serde_json::{json, Value};

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces three points are collinear (cross product residual).
#[derive(Clone, Debug, PartialEq)]
pub struct CollinearConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
    /// Third point.
    pub p3: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

impl CollinearConstraint {
    /// Creates a collinear constraint.
    pub fn new(p1: u64, p2: u64, p3: u64) -> Self {
        CollinearConstraint {
            user_visible: true,
            p1,
            p2,
            p3,
        }
    }
}

impl SketchConstraint for CollinearConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.p1, self.p2, self.p3]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        let (x3, y3) = reg.xy(self.p3);
        vec![(x2 - x1) * (y3 - y1) - (y2 - y1) * (x3 - x1)]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        let (x3, y3) = reg.xy(self.p3);
        vec![
            (self.p1, 0, y2 - y3, x3 - x2),
            (self.p2, 0, y3 - y1, -(x3 - x1)),
            (self.p3, 0, -(y2 - y1), x2 - x1),
        ]
    }
}

impl CollinearConstraint {
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
            "type": "CollinearConstraint",
            "user_visible": self.user_visible,
        });
        data["p1"] = json!(self.p1);
        data["p2"] = json!(self.p2);
        data["p3"] = json!(self.p3);
        data
    }
}
