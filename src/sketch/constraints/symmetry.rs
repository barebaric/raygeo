//! Symmetry constraint: two points mirrored about a point or a line.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// The symmetry reference, mirroring the reference constraint's two
/// cases: point symmetry about a center, or line symmetry about an
/// axis line.
#[derive(Clone, Debug, PartialEq)]
pub enum SymmetryTarget {
    /// The center is the midpoint of `p1` and `p2`.
    Point { center: u64 },
    /// `p1` and `p2` are mirrored across the line `(l1, l2)`.
    Line { l1: u64, l2: u64 },
}

/// Enforces two points are symmetric about a point or a line.
#[derive(Clone, Debug, PartialEq)]
pub struct SymmetryConstraint {
    /// First point.
    pub p1: u64,
    /// Second point.
    pub p2: u64,
    /// Symmetry reference.
    pub target: SymmetryTarget,
}

impl SymmetryConstraint {
    /// Creates a symmetry constraint.
    pub fn new(p1: u64, p2: u64, target: SymmetryTarget) -> Self {
        SymmetryConstraint { p1, p2, target }
    }
}

impl SketchConstraint for SymmetryConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        let mut ids = vec![self.p1, self.p2];
        match &self.target {
            SymmetryTarget::Point { center } => ids.push(*center),
            SymmetryTarget::Line { l1, l2 } => {
                ids.push(*l1);
                ids.push(*l2);
            }
        }
        ids
    }

    fn residual_count(&self) -> usize {
        2
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let (x1, y1) = reg.xy(self.p1);
        let (x2, y2) = reg.xy(self.p2);
        match &self.target {
            SymmetryTarget::Point { center } => {
                let (cx, cy) = reg.xy(*center);
                vec![(x1 + x2) - 2.0 * cx, (y1 + y2) - 2.0 * cy]
            }
            SymmetryTarget::Line { l1, l2 } => {
                let (lx1, ly1) = reg.xy(*l1);
                let (lx2, ly2) = reg.xy(*l2);
                let dx_l = lx2 - lx1;
                let dy_l = ly2 - ly1;
                let dx_p = x2 - x1;
                let dy_p = y2 - y1;
                let err_perp = dx_p * dx_l + dy_p * dy_l;
                let mx = (x1 + x2) * 0.5;
                let my = (y1 + y2) * 0.5;
                let err_collinear = (mx - lx1) * dy_l - (my - ly1) * dx_l;
                vec![err_perp, err_collinear]
            }
        }
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        match &self.target {
            SymmetryTarget::Point { center } => vec![
                (self.p1, 0, 1.0, 0.0),
                (self.p1, 1, 0.0, 1.0),
                (self.p2, 0, 1.0, 0.0),
                (self.p2, 1, 0.0, 1.0),
                (*center, 0, -2.0, 0.0),
                (*center, 1, 0.0, -2.0),
            ],
            SymmetryTarget::Line { l1, l2 } => {
                let (x1, y1) = reg.xy(self.p1);
                let (x2, y2) = reg.xy(self.p2);
                let (lx1, ly1) = reg.xy(*l1);
                let (lx2, ly2) = reg.xy(*l2);
                let dxl = lx2 - lx1;
                let dyl = ly2 - ly1;
                let mx = (x1 + x2) * 0.5;
                let my = (y1 + y2) * 0.5;
                vec![
                    (self.p1, 0, -dxl, -dyl),
                    (self.p2, 0, dxl, dyl),
                    (*l1, 0, -(x2 - x1), -(y2 - y1)),
                    (*l2, 0, x2 - x1, y2 - y1),
                    (self.p1, 1, 0.5 * dyl, -0.5 * dxl),
                    (self.p2, 1, 0.5 * dyl, -0.5 * dxl),
                    (*l1, 1, my - ly1 - dyl, dxl - (mx - lx1)),
                    (*l2, 1, -(my - ly1), mx - lx1),
                ]
            }
        }
    }
}
