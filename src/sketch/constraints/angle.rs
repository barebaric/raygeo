//! Angle constraint between two lines.

use crate::geo::shape::arc::normalize_angle;
use crate::geo::shape::line::get_line_line_intersection;
use crate::geo::types::Point;
use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Weight of the angular residual.
pub const ANGLE_WEIGHT: f64 = 10.0;

/// Central-difference step used for the Jacobian.
const GRAD_FD_EPS: f64 = 1e-6;

/// Enforces a specific angle in degrees between two lines.
///
/// The angle is measured clockwise from the anchor direction towards
/// the other direction, using the rays pointing away from the lines'
/// intersection towards the far points.  The Jacobian uses central
/// finite differences, mirroring the reference implementation.
#[derive(Clone, Debug, PartialEq)]
pub struct AngleConstraint {
    /// First point of the anchor line.
    pub p1: u64,
    /// Second point of the anchor line.
    pub p2: u64,
    /// First point of the other line.
    pub p3: u64,
    /// Second point of the other line.
    pub p4: u64,
    /// Far point of the anchor line; `None` picks the endpoint
    /// farther from the intersection.
    pub e1_far: Option<u64>,
    /// Far point of the other line.
    pub e2_far: Option<u64>,
    /// Resolved angle value in degrees.
    pub value_deg: f64,
}

impl AngleConstraint {
    /// Creates an angle constraint with a resolved value.
    pub fn new(
        p1: u64,
        p2: u64,
        p3: u64,
        p4: u64,
        e1_far: Option<u64>,
        e2_far: Option<u64>,
        value_deg: f64,
    ) -> Self {
        AngleConstraint {
            p1,
            p2,
            p3,
            p4,
            e1_far,
            e2_far,
            value_deg,
        }
    }

    fn far_point(
        &self,
        first: u64,
        second: u64,
        far: Option<u64>,
        ix: f64,
        iy: f64,
        reg: &dyn PointContext,
    ) -> (f64, f64) {
        if let Some(id) = far {
            if id == first {
                return reg.xy(first);
            }
            if id == second {
                return reg.xy(second);
            }
        }
        let a = reg.xy(first);
        let b = reg.xy(second);
        let d1 = (a.0 - ix).powi(2) + (a.1 - iy).powi(2);
        let d2 = (b.0 - ix).powi(2) + (b.1 - iy).powi(2);
        if d1 > d2 {
            a
        } else {
            b
        }
    }
}

impl SketchConstraint for AngleConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        let mut ids = vec![self.p1, self.p2, self.p3, self.p4];
        for far in [self.e1_far, self.e2_far].into_iter().flatten() {
            if !ids.contains(&far) {
                ids.push(far);
            }
        }
        ids
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let intersection = get_line_line_intersection(
            Point::new(reg.xy(self.p1).0, reg.xy(self.p1).1),
            Point::new(reg.xy(self.p2).0, reg.xy(self.p2).1),
            Point::new(reg.xy(self.p3).0, reg.xy(self.p3).1),
            Point::new(reg.xy(self.p4).0, reg.xy(self.p4).1),
        );
        let (ix, iy) = match intersection {
            Some(point) => (point.x, point.y),
            None => return vec![0.0],
        };

        let far1 = self.far_point(self.p1, self.p2, self.e1_far, ix, iy, reg);
        let far2 = self.far_point(self.p3, self.p4, self.e2_far, ix, iy, reg);

        let dx1 = far1.0 - ix;
        let dy1 = far1.1 - iy;
        let dx2 = far2.0 - ix;
        let dy2 = far2.1 - iy;
        let len1_sq = dx1 * dx1 + dy1 * dy1;
        let len2_sq = dx2 * dx2 + dy2 * dy2;
        if len1_sq < 1e-12 || len2_sq < 1e-12 {
            return vec![0.0];
        }

        let anchor_dir = dy1.atan2(dx1);
        let other_dir = dy2.atan2(dx2);
        let current = normalize_angle(anchor_dir - other_dir);
        let target = self.value_deg.to_radians();
        let mut diff = current - target;
        while diff > std::f64::consts::PI {
            diff -= 2.0 * std::f64::consts::PI;
        }
        while diff <= -std::f64::consts::PI {
            diff += 2.0 * std::f64::consts::PI;
        }
        vec![diff * ANGLE_WEIGHT]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let mut entries = Vec::new();
        for pid in [self.p1, self.p2, self.p3, self.p4] {
            let (orig_x, orig_y) = reg.xy(pid);

            reg.set_xy(pid, orig_x + GRAD_FD_EPS, orig_y);
            let err_x_plus = self.error(reg)[0];
            reg.set_xy(pid, orig_x - GRAD_FD_EPS, orig_y);
            let err_x_minus = self.error(reg)[0];
            let gx = (err_x_plus - err_x_minus) / (2.0 * GRAD_FD_EPS);

            reg.set_xy(pid, orig_x, orig_y + GRAD_FD_EPS);
            let err_y_plus = self.error(reg)[0];
            reg.set_xy(pid, orig_x, orig_y - GRAD_FD_EPS);
            let err_y_minus = self.error(reg)[0];
            let gy = (err_y_plus - err_y_minus) / (2.0 * GRAD_FD_EPS);

            reg.set_xy(pid, orig_x, orig_y);
            entries.push((pid, 0, gx, gy));
        }
        entries
    }
}
