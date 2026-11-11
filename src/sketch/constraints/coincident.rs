//! Coincident constraint: two points at the same location.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces two points are at the same location.
#[derive(Clone, Debug, PartialEq)]
pub struct CoincidentConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
}

impl CoincidentConstraint {
    /// Creates a coincident constraint between two points.
    pub fn new(p1: u64, p2: u64) -> Self {
        CoincidentConstraint { p1, p2 }
    }
}

impl SketchConstraint for CoincidentConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.p1, self.p2]
    }

    fn residual_count(&self) -> usize {
        2
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        vec![x1 - x2, y1 - y2]
    }

    fn gradient(&self, _reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        vec![
            (self.p1, 0, 1.0, 0.0),
            (self.p1, 1, 0.0, 1.0),
            (self.p2, 0, -1.0, 0.0),
            (self.p2, 1, 0.0, -1.0),
        ]
    }
}
