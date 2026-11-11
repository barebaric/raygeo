//! Perpendicular constraint between two entities.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// The resolved geometry of the two perpendicular operands, mirroring
/// the reference constraint's entity-type cases: two lines, a line
/// through a circle/arc center, or two circles meeting at a right
/// angle.
#[derive(Clone, Debug, PartialEq)]
pub enum PerpendicularTarget {
    /// Line-line: direction dot product.
    LineLine { a1: u64, a2: u64, b1: u64, b2: u64 },
    /// Line and circle/arc: the line passes through the center.
    LineCenter { l1: u64, l2: u64, center: u64 },
    /// Circle/arc pair: `r1^2 + r2^2 == dist(c1, c2)^2`.
    Circles {
        c1: u64,
        r1_point: u64,
        c2: u64,
        r2_point: u64,
    },
}

/// Enforces perpendicularity between two entities.
#[derive(Clone, Debug, PartialEq)]
pub struct PerpendicularConstraint {
    /// Resolved operand geometry.
    pub target: PerpendicularTarget,
}

impl PerpendicularConstraint {
    /// Creates a perpendicular constraint from resolved operands.
    pub fn new(target: PerpendicularTarget) -> Self {
        PerpendicularConstraint { target }
    }
}

impl SketchConstraint for PerpendicularConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        let mut ids = Vec::new();
        let mut push = |id: u64| {
            if !ids.contains(&id) {
                ids.push(id);
            }
        };
        match &self.target {
            PerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                push(*a1);
                push(*a2);
                push(*b1);
                push(*b2);
            }
            PerpendicularTarget::LineCenter { l1, l2, center } => {
                push(*l1);
                push(*l2);
                push(*center);
            }
            PerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => {
                push(*c1);
                push(*r1_point);
                push(*c2);
                push(*r2_point);
            }
        }
        ids
    }

    fn residual_count(&self) -> usize {
        1
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        match &self.target {
            PerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                let (ax1, ay1) = reg.xy(*a1);
                let (ax2, ay2) = reg.xy(*a2);
                let (bx1, by1) = reg.xy(*b1);
                let (bx2, by2) = reg.xy(*b2);
                let dx1 = ax2 - ax1;
                let dy1 = ay2 - ay1;
                let dx2 = bx2 - bx1;
                let dy2 = by2 - by1;
                vec![dx1 * dx2 + dy1 * dy2]
            }
            PerpendicularTarget::LineCenter { l1, l2, center } => {
                let (x1, y1) = reg.xy(*l1);
                let (x2, y2) = reg.xy(*l2);
                let (cx, cy) = reg.xy(*center);
                vec![(x2 - x1) * (cy - y1) - (cx - x1) * (y2 - y1)]
            }
            PerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => {
                let (c1x, c1y) = reg.xy(*c1);
                let (p1x, p1y) = reg.xy(*r1_point);
                let (c2x, c2y) = reg.xy(*c2);
                let (p2x, p2y) = reg.xy(*r2_point);
                let r1_sq = (p1x - c1x).hypot(p1y - c1y).powi(2);
                let r2_sq = (p2x - c2x).hypot(p2y - c2y).powi(2);
                let dist_sq = (c2x - c1x).powi(2) + (c2y - c1y).powi(2);
                vec![r1_sq + r2_sq - dist_sq]
            }
        }
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let mut entries = Vec::new();
        match &self.target {
            PerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                let (ax1, ay1) = reg.xy(*a1);
                let (ax2, ay2) = reg.xy(*a2);
                let (bx1, by1) = reg.xy(*b1);
                let (bx2, by2) = reg.xy(*b2);
                let dx1 = ax2 - ax1;
                let dy1 = ay2 - ay1;
                let dx2 = bx2 - bx1;
                let dy2 = by2 - by1;
                entries.push((*a1, 0, -dx2, -dy2));
                entries.push((*a2, 0, dx2, dy2));
                entries.push((*b1, 0, -dx1, -dy1));
                entries.push((*b2, 0, dx1, dy1));
            }
            PerpendicularTarget::LineCenter { l1, l2, center } => {
                let (x1, y1) = reg.xy(*l1);
                let (x2, y2) = reg.xy(*l2);
                let (cx, cy) = reg.xy(*center);
                let dx = x2 - x1;
                let dy = y2 - y1;
                entries.push((*center, 0, -dy, dx));
                entries.push((*l1, 0, dy - (cy - y1), -dx + (cx - x1)));
                entries.push((*l2, 0, cy - y1, -(cx - x1)));
            }
            PerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => {
                let (c1x, c1y) = reg.xy(*c1);
                let (p1x, p1y) = reg.xy(*r1_point);
                let (c2x, c2y) = reg.xy(*c2);
                let (p2x, p2y) = reg.xy(*r2_point);
                let d_dist_x = 2.0 * (c2x - c1x);
                let d_dist_y = 2.0 * (c2y - c1y);
                entries.push((
                    *r1_point,
                    0,
                    2.0 * (p1x - c1x),
                    2.0 * (p1y - c1y),
                ));
                entries.push((
                    *c1,
                    0,
                    d_dist_x - 2.0 * (p1x - c1x),
                    d_dist_y - 2.0 * (p1y - c1y),
                ));
                entries.push((
                    *r2_point,
                    0,
                    2.0 * (p2x - c2x),
                    2.0 * (p2y - c2y),
                ));
                entries.push((
                    *c2,
                    0,
                    -d_dist_x - 2.0 * (p2x - c2x),
                    -d_dist_y - 2.0 * (p2y - c2y),
                ));
            }
        }
        entries
    }
}
