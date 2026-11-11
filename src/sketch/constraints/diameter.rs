//! Diameter constraint: diameter of a circle.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces the diameter of a circle: twice the distance between the
/// center point and the radius point equals `value`.
#[derive(Clone, Debug, PartialEq)]
pub struct DiameterConstraint {
    /// Center point of the circle.
    pub center: u64,
    /// Radius-bearing point of the circle.
    pub radius_point: u64,
    /// Resolved diameter value.
    pub value: f64,
}

impl DiameterConstraint {
    /// Creates a diameter constraint with a resolved value.
    pub fn new(center: u64, radius_point: u64, value: f64) -> Self {
        DiameterConstraint {
            center,
            radius_point,
            value,
        }
    }
}

impl SketchConstraint for DiameterConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.center, self.radius_point]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (cx, cy) = reg.xy(self.center);
        let (px, py) = reg.xy(self.radius_point);
        vec![2.0 * ((px - cx).hypot(py - cy)) - self.value]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let (cx, cy) = reg.xy(self.center);
        let (px, py) = reg.xy(self.radius_point);
        let dx = px - cx;
        let dy = py - cy;
        let dist = dx.hypot(dy);
        let (ux, uy) = if dist > 1e-9 {
            (dx / dist, dy / dist)
        } else {
            (1.0, 0.0)
        };
        vec![
            (self.radius_point, 0, 2.0 * ux, 2.0 * uy),
            (self.center, 0, -2.0 * ux, -2.0 * uy),
        ]
    }
}
