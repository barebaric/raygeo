//! Drag constraint: soft pull towards a target coordinate.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// A transient constraint that pulls a point towards a target
/// (mouse) coordinate, scaled by `weight`.  Used for interactive
/// dragging and for the weak stabilizer terms of a solve.
#[derive(Clone, Debug, PartialEq)]
pub struct DragConstraint {
    /// The dragged point.
    pub point: u64,
    /// Target X coordinate.
    pub target_x: f64,
    /// Target Y coordinate.
    pub target_y: f64,
    /// Pull strength.
    pub weight: f64,
}

impl DragConstraint {
    /// Creates a drag constraint.
    pub fn new(point: u64, target_x: f64, target_y: f64, weight: f64) -> Self {
        DragConstraint {
            point,
            target_x,
            target_y,
            weight,
        }
    }
}

impl SketchConstraint for DragConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.point]
    }

    fn residual_count(&self) -> usize {
        2
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (x, y) = reg.xy(self.point);
        vec![
            (x - self.target_x) * self.weight,
            (y - self.target_y) * self.weight,
        ]
    }

    fn gradient(&self, _reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        vec![
            (self.point, 0, self.weight, 0.0),
            (self.point, 1, 0.0, self.weight),
        ]
    }
}
