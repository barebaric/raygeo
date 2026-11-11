//! Horizontal constraint: two points share their Y coordinate.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces two points have the same Y coordinate.
#[derive(Clone, Debug, PartialEq)]
pub struct HorizontalConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
}

impl HorizontalConstraint {
    /// Creates a horizontal constraint between two points.
    pub fn new(p1: u64, p2: u64) -> Self {
        HorizontalConstraint { p1, p2 }
    }
}

impl SketchConstraint for HorizontalConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.p1, self.p2]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        vec![reg.xy(self.p1).1 - reg.xy(self.p2).1]
    }

    fn gradient(&self, _reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        vec![(self.p1, 0, 0.0, 1.0), (self.p2, 0, 0.0, -1.0)]
    }
}
