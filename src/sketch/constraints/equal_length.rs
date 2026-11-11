//! Equal-length constraint between characteristic lengths.

use crate::sketch::constraint::{
    JacobianEntry, PointContext, SketchConstraint,
};

/// Enforces equal characteristic lengths.  Each pair is
/// `(base_a, base_b, other_a, other_b)` with one residual per pair;
/// the caller expands the entities' characteristic-length pairs
/// (including the shorter-side fallback) into flat pairs.
#[derive(Clone, Debug, PartialEq)]
pub struct EqualLengthConstraint {
    /// Resolved characteristic-length pairs.
    pub pairs: Vec<[u64; 4]>,
}

impl EqualLengthConstraint {
    /// Creates an equal-length constraint from resolved pairs.
    pub fn new(pairs: Vec<[u64; 4]>) -> Self {
        EqualLengthConstraint { pairs }
    }

    fn pair_length(&self, a: u64, b: u64, reg: &dyn PointContext) -> f64 {
        let (ax, ay) = reg.xy(a);
        let (bx, by) = reg.xy(b);
        (bx - ax).hypot(by - ay)
    }
}

impl SketchConstraint for EqualLengthConstraint {
    fn referenced_points(&self) -> Vec<u64> {
        let mut ids = Vec::new();
        for pair in &self.pairs {
            for id in pair {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        ids
    }

    fn residual_count(&self) -> usize {
        self.pairs.len()
    }

    fn error(&self, reg: &dyn PointContext) -> Vec<f64> {
        let mut errors = Vec::with_capacity(self.pairs.len());
        for pair in &self.pairs {
            let base = self.pair_length(pair[0], pair[1], reg);
            let other = self.pair_length(pair[2], pair[3], reg);
            errors.push(other - base);
        }
        errors
    }

    fn gradient(&self, reg: &mut dyn PointContext) -> Vec<JacobianEntry> {
        let mut entries = Vec::new();
        for (row, pair) in self.pairs.iter().enumerate() {
            let base_dx = reg.xy(pair[1]).0 - reg.xy(pair[0]).0;
            let base_dy = reg.xy(pair[1]).1 - reg.xy(pair[0]).1;
            let (bux, buy) = unit_or_zero(base_dx, base_dy);
            let other_dx = reg.xy(pair[3]).0 - reg.xy(pair[2]).0;
            let other_dy = reg.xy(pair[3]).1 - reg.xy(pair[2]).1;
            let (eux, euy) = unit_or_zero(other_dx, other_dy);
            entries.push((pair[0], row, bux, buy));
            entries.push((pair[1], row, -bux, -buy));
            entries.push((pair[2], row, -eux, -euy));
            entries.push((pair[3], row, eux, euy));
        }
        entries
    }
}

/// Unit vector, defaulting to zero for degenerate segments.
fn unit_or_zero(dx: f64, dy: f64) -> (f64, f64) {
    let dist = dx.hypot(dy);
    if dist > 1e-9 {
        (dx / dist, dy / dist)
    } else {
        (0.0, 0.0)
    }
}
