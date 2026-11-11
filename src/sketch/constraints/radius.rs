//! Radius constraint: radius of a circle or arc.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces the radius of a circle or arc: the distance between the
/// center point and the radius-bearing point equals `value`.
#[derive(Clone, Debug, PartialEq)]
pub struct RadiusConstraint {
    /// Center point of the circle/arc.
    pub center: u64,
    /// Radius-bearing point (circle radius point or arc start).
    pub radius_point: u64,
    /// Resolved radius value.
    pub value: f64,
}

impl RadiusConstraint {
    /// Creates a radius constraint with a resolved value.
    pub fn new(center: u64, radius_point: u64, value: f64) -> Self {
        RadiusConstraint {
            center,
            radius_point,
            value,
        }
    }
}

impl SketchConstraint for RadiusConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.center, self.radius_point]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (cx, cy) = reg.xy(self.center);
        let (px, py) = reg.xy(self.radius_point);
        vec![(px - cx).hypot(py - cy) - self.value]
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
        vec![(self.radius_point, 0, ux, uy), (self.center, 0, -ux, -uy)]
    }
}
