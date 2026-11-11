//! Distance constraint: fixed distance between two points.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces a fixed distance between two points.
#[derive(Clone, Debug, PartialEq)]
pub struct DistanceConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
    /// Resolved distance value.
    pub value: f64,
}

impl DistanceConstraint {
    /// Creates a distance constraint with a resolved value.
    pub fn new(p1: u64, p2: u64, value: f64) -> Self {
        DistanceConstraint { p1, p2, value }
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
}
