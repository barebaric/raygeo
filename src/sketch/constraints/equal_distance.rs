//! Equal-distance constraint between two point pairs.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

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
}

impl EqualDistanceConstraint {
    /// Creates an equal-distance constraint.
    pub fn new(p1: u64, p2: u64, p3: u64, p4: u64) -> Self {
        EqualDistanceConstraint { p1, p2, p3, p4 }
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
}
