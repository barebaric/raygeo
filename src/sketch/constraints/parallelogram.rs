//! Parallelogram constraint for four points.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces `(p_width - p_origin) == (p4 - p_height)`.
#[derive(Clone, Debug, PartialEq)]
pub struct ParallelogramConstraint {
    /// Origin point.
    pub p_origin: u64,
    /// Width point.
    pub p_width: u64,
    /// Height point.
    pub p_height: u64,
    /// Fourth corner.
    pub p4: u64,
}

impl ParallelogramConstraint {
    /// Creates a parallelogram constraint.
    pub fn new(p_origin: u64, p_width: u64, p_height: u64, p4: u64) -> Self {
        ParallelogramConstraint {
            p_origin,
            p_width,
            p_height,
            p4,
        }
    }
}

impl SketchConstraint for ParallelogramConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.p_origin, self.p_width, self.p_height, self.p4]
    }

    fn residual_count(&self) -> usize {
        2
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (ox, oy) = reg.xy(self.p_origin);
        let (wx, wy) = reg.xy(self.p_width);
        let (hx, hy) = reg.xy(self.p_height);
        let (fx, fy) = reg.xy(self.p4);
        vec![(wx - ox) - (fx - hx), (wy - oy) - (fy - hy)]
    }

    fn gradient(&self, _reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        vec![
            (self.p_origin, 0, -1.0, 0.0),
            (self.p_origin, 1, 0.0, -1.0),
            (self.p_width, 0, 1.0, 0.0),
            (self.p_width, 1, 0.0, 1.0),
            (self.p_height, 0, 1.0, 0.0),
            (self.p_height, 1, 0.0, 1.0),
            (self.p4, 0, -1.0, 0.0),
            (self.p4, 1, 0.0, -1.0),
        ]
    }
}
