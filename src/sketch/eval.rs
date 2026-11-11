//! Residual and Jacobian assembly over sketch constraints.

use std::collections::HashMap;

use crate::sketch::constraint::{PointContext, SketchConstraint};

/// Read/write coordinate table over the sketch's points, addressed by
/// point id.  Implements [`PointContext`] for the constraints and
/// provides the residual/Jacobian assembly used by the solver.
pub struct Evaluator {
    slot_of: HashMap<u64, usize>,
    pub(crate) coords: Vec<f64>,
}

impl Evaluator {
    /// Builds an evaluator over `coords` (two floats per point,
    /// indexed through `slot_of`).
    pub fn new(slot_of: HashMap<u64, usize>, coords: Vec<f64>) -> Self {
        Evaluator { slot_of, coords }
    }

    /// Overwrites the coordinates of the given slots from the flat
    /// state vector (two floats per slot).
    pub fn set_state(&mut self, state: &[f64], state_slots: &[usize]) {
        for (i, &slot) in state_slots.iter().enumerate() {
            self.coords[2 * slot] = state[2 * i];
            self.coords[2 * slot + 1] = state[2 * i + 1];
        }
    }

    /// Returns all residual values of the given constraints, in
    /// order.  Falls back to a single zero residual when no
    /// constraint contributes residuals.
    pub fn residuals(&self, constraints: &[&dyn SketchConstraint]) -> Vec<f64> {
        let mut residuals = Vec::new();
        for constraint in constraints {
            if constraint.residual_count() == 0 {
                continue;
            }
            residuals.extend(constraint.error(self));
        }
        if residuals.is_empty() {
            residuals.push(0.0);
        }
        residuals
    }

    /// Builds the dense Jacobian (row-major, `rows * n_vars`) of the
    /// given constraints at the current coordinates.  Entries for
    /// points outside `state_col_of` are skipped; `n_vars` is twice
    /// the number of state points.
    pub fn jacobian(
        &mut self,
        constraints: &[&dyn SketchConstraint],
        n_vars: usize,
        state_col_of: &HashMap<u64, usize>,
    ) -> Vec<f64> {
        let mut rows = 0;
        for constraint in constraints {
            rows += constraint.residual_count();
        }
        if rows == 0 {
            rows = 1;
        }
        let mut jac = vec![0.0; rows * n_vars];
        let mut row_offset = 0;
        for constraint in constraints {
            let count = constraint.residual_count();
            if count == 0 {
                continue;
            }
            for (pid, row, gx, gy) in constraint.gradient(self) {
                if let Some(&col) = state_col_of.get(&pid) {
                    let r = row_offset + row;
                    jac[r * n_vars + col] += gx;
                    jac[r * n_vars + col + 1] += gy;
                }
            }
            row_offset += count;
        }
        jac
    }

    /// Returns the maximum absolute residual per constraint (in list
    /// order).  Constraints without residuals yield 0.
    pub fn max_residuals(
        &self,
        constraints: &[&dyn SketchConstraint],
    ) -> Vec<f64> {
        constraints
            .iter()
            .map(|constraint| {
                if constraint.residual_count() == 0 {
                    return 0.0;
                }
                constraint
                    .error(self)
                    .iter()
                    .fold(0.0f64, |acc, v| acc.max(v.abs()))
            })
            .collect()
    }
}

impl PointContext for Evaluator {
    fn xy(&self, id: u64) -> (f64, f64) {
        let slot = self.slot_of[&id];
        (self.coords[2 * slot], self.coords[2 * slot + 1])
    }

    fn set_xy(&mut self, id: u64, x: f64, y: f64) {
        let slot = self.slot_of[&id];
        self.coords[2 * slot] = x;
        self.coords[2 * slot + 1] = y;
    }
}
