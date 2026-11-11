//! Collinear constraint: three points on one line.

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
}

impl CollinearConstraint {
    /// Creates a collinear constraint.
    pub fn new(p1: u64, p2: u64, p3: u64) -> Self {
        CollinearConstraint { p1, p2, p3 }
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
