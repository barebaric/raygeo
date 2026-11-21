//! Residual and Jacobian assembly over sketch constraints.

use std::collections::HashMap;

use crate::sketch::constraint::{PointContext, SketchConstraint};
use crate::sketch::draw::DrawPoint;
use crate::sketch::entities::entity::Entity;

/// Read/write coordinate table over the sketch's points, addressed by
/// point id.  Implements [`PointContext`] for the constraints and
/// provides the residual/Jacobian assembly used by the solver.
pub struct Evaluator {
    slot_of: HashMap<u64, usize>,
    pub(crate) coords: Vec<f64>,
    /// Structure snapshot (entity kinds and their point ids) used by
    /// entity-referencing constraints; positions always come from
    /// `coords`.
    registry: crate::sketch::registry::EntityRegistry,
}

impl Evaluator {
    /// Builds an evaluator over `coords` (two floats per point,
    /// indexed through `slot_of`).
    pub fn new(
        slot_of: HashMap<u64, usize>,
        coords: Vec<f64>,
        registry: crate::sketch::registry::EntityRegistry,
    ) -> Self {
        Evaluator {
            slot_of,
            coords,
            registry,
        }
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
            let count = constraint.residual_count(self);
            if count == 0 {
                continue;
            }
            let error = constraint.error(self);
            debug_assert_eq!(
                error.len(),
                count,
                "constraint returned {} residuals, expected {}",
                error.len(),
                count
            );
            residuals.extend(error);
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
        let rows = padded_row_count(self, constraints);
        let mut jac = vec![0.0; rows * n_vars];
        let mut row_offset = 0;
        for constraint in constraints {
            let count = constraint.residual_count(self);
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
                let count = constraint.residual_count(self);
                if count == 0 {
                    return 0.0;
                }
                let error = constraint.error(self);
                debug_assert_eq!(
                    error.len(),
                    count,
                    "constraint returned {} residuals, expected {}",
                    error.len(),
                    count
                );
                error.iter().fold(0.0f64, |acc, v| acc.max(v.abs()))
            })
            .collect()
    }
}

/// Total residual row count of the given constraints; the TRF
/// kernel needs at least one row, so an empty constraint set
/// reports one (matching the zero residual `residuals` synthesizes).
pub fn padded_row_count(
    ctx: &dyn PointContext,
    constraints: &[&dyn SketchConstraint],
) -> usize {
    constraints
        .iter()
        .map(|constraint| constraint.residual_count(ctx))
        .sum::<usize>()
        .max(1)
}

impl PointContext for Evaluator {
    fn get_entity(&self, id: u64) -> Option<Box<dyn Entity>> {
        self.registry.get_entity(id).map(|e| e.clone_box())
    }

    fn point_position(&self, id: u64) -> Option<(f64, f64)> {
        let slot = *self.slot_of.get(&id)?;
        Some((self.coords[2 * slot], self.coords[2 * slot + 1]))
    }

    fn get_point(&self, id: u64) -> Option<DrawPoint> {
        let point = self.registry.get_point(id)?;
        // Positions must come from the live solver state, never from
        // the registry snapshot (see the type's docs); only the
        // fixed/constrained flags are snapshot data.
        let (x, y) = self.point_position(id)?;
        Some(DrawPoint {
            x,
            y,
            fixed: point.fixed,
            constrained: point.constrained,
        })
    }

    fn xy(&self, id: u64) -> (f64, f64) {
        let slot = self.slot_of[&id];
        (self.coords[2 * slot], self.coords[2 * slot + 1])
    }

    fn set_xy(&mut self, id: u64, x: f64, y: f64) {
        let slot = self.slot_of[&id];
        self.coords[2 * slot] = x;
        self.coords[2 * slot + 1] = y;
    }

    fn entity_ids(&self) -> Vec<u64> {
        self.registry.entity_ids()
    }

    fn entity_type(&self, id: u64) -> Option<std::borrow::Cow<'static, str>> {
        self.registry
            .entity_type(id)
            .map(std::borrow::Cow::Borrowed)
    }

    fn entity_point_ids(&self, id: u64) -> Option<Vec<u64>> {
        self.registry.entity_point_ids(id)
    }
}
