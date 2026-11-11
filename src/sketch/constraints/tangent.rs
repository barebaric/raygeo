//! Tangent constraint: a line tangent to a circle or arc.

use crate::geo::shape::line::get_point_line_signed_distance;
use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces a line is tangent to a circle/arc: the absolute signed
/// distance from the center to the line equals the radius.
#[derive(Clone, Debug, PartialEq)]
pub struct TangentConstraint {
    /// First point of the line.
    pub l1: u64,
    /// Second point of the line.
    pub l2: u64,
    /// Center point of the circle/arc.
    pub center: u64,
    /// Radius-bearing point (circle radius point or arc start).
    pub radius_point: u64,
}

impl TangentConstraint {
    /// Creates a tangent constraint between a line and a circle/arc.
    pub fn new(l1: u64, l2: u64, center: u64, radius_point: u64) -> Self {
        TangentConstraint {
            l1,
            l2,
            center,
            radius_point,
        }
    }
}

impl SketchConstraint for TangentConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        vec![self.l1, self.l2, self.center, self.radius_point]
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (cx, cy) = reg.xy(self.center);
        let (px, py) = reg.xy(self.radius_point);
        let signed = get_point_line_signed_distance(
            crate::geo::types::Point::new(cx, cy),
            crate::geo::types::Point::new(reg.xy(self.l1).0, reg.xy(self.l1).1),
            crate::geo::types::Point::new(reg.xy(self.l2).0, reg.xy(self.l2).1),
        );
        vec![signed.abs() - (px - cx).hypot(py - cy)]
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let (cx, cy) = reg.xy(self.center);
        let (px, py) = reg.xy(self.radius_point);
        let mut entries = Vec::new();

        // Part 1: gradient of the radius term.
        let rad_dx = px - cx;
        let rad_dy = py - cy;
        let radius = rad_dx.hypot(rad_dy);
        let (rad_ux, rad_uy) = if radius > 1e-9 {
            (rad_dx / radius, rad_dy / radius)
        } else {
            (0.0, 0.0)
        };
        entries.push((self.radius_point, 0, -rad_ux, -rad_uy));

        let (lx1, ly1) = reg.xy(self.l1);
        let (lx2, ly2) = reg.xy(self.l2);
        let line_dx = lx2 - lx1;
        let line_dy = ly2 - ly1;
        let len_sq = line_dx * line_dx + line_dy * line_dy;

        if len_sq < 1e-18 {
            // Degenerate line: gradient of the plain distance.
            let dist_c_l1 = (cx - lx1).hypot(cy - ly1);
            if dist_c_l1 < 1e-9 {
                entries.push((self.center, 0, rad_ux, rad_uy));
                return entries;
            }
            let ux = (cx - lx1) / dist_c_l1;
            let uy = (cy - ly1) / dist_c_l1;
            entries.push((self.center, 0, rad_ux + ux, rad_uy + uy));
            entries.push((self.l1, 0, -ux, -uy));
            return entries;
        }

        let length = len_sq.sqrt();
        let inv_len = 1.0 / length;
        let cross = line_dx * (cy - ly1) - line_dy * (cx - lx1);
        let sign = 1.0_f64.copysign(cross);

        // Part 2: gradient of the signed distance, combined with the
        // sign of the abs() term.
        let nx = -line_dy * inv_len;
        let ny = line_dx * inv_len;
        let pol_err = cross * inv_len;
        let inv_len_sq = inv_len * inv_len;

        let grad_pol_c_x = nx;
        let grad_pol_c_y = ny;
        let grad_pol_l2_x =
            (cy - ly1) * inv_len - pol_err * line_dx * inv_len_sq;
        let grad_pol_l2_y =
            -(cx - lx1) * inv_len - pol_err * line_dy * inv_len_sq;
        let grad_pol_l1_x = -grad_pol_c_x - grad_pol_l2_x;
        let grad_pol_l1_y = -grad_pol_c_y - grad_pol_l2_y;

        entries.push((
            self.center,
            0,
            rad_ux + sign * grad_pol_c_x,
            rad_uy + sign * grad_pol_c_y,
        ));
        entries.push((self.l1, 0, sign * grad_pol_l1_x, sign * grad_pol_l1_y));
        entries.push((self.l2, 0, sign * grad_pol_l2_x, sign * grad_pol_l2_y));
        entries
    }
}
