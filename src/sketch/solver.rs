//! Sketch constraint solver.
//!
//! Solves the point positions of a sketch so that its constraints are
//! satisfied.  A global solve partitions the constraints into
//! connected components (see [`components`]) and solves each island
//! separately; a scoped solve optimizes a restricted point subset.
//! After a successful solve the degrees of freedom are analyzed per
//! component on its own Jacobian (null-space analysis), and
//! conflicting constraints can be detected from their residuals.

use std::collections::{BTreeSet, HashMap, HashSet};

use nalgebra::{DMatrix, SVD};

use super::components::compute_constraint_components;
use super::constraint::SketchConstraint;
use super::eval::Evaluator;
use super::trf::{solve_trf_no_bounds, TrfProblem};

/// Residual threshold above which a constraint is reported
/// conflicting.
pub const CONFLICT_ERROR_THRESHOLD: f64 = 1e-3;

/// SciPy-comparable null-space cutoff for the DOF analysis.
const DOF_RCOND: f64 = 1e-9;

/// Mobility threshold below which a point counts as fully
/// constrained.
const MOBILITY_THRESHOLD: f64 = 1e-4;

/// Maximum residual function evaluations per solve, mirroring the
/// reference sketch solver.
const MAX_NFEV: usize = 5000;

/// A sketch problem: points plus hard and auxiliary constraints.
pub struct SketchSolver {
    point_ids: Vec<u64>,
    fixed: Vec<bool>,
    slot_of: HashMap<u64, usize>,
    coords: Vec<f64>,
    constrained: Vec<bool>,
    hard: Vec<Box<dyn SketchConstraint>>,
    auxiliary: Vec<Box<dyn SketchConstraint>>,
    point_filter: Option<HashSet<u64>>,
}

impl SketchSolver {
    /// Builds a solver from point definitions
    /// `(id, x, y, fixed)`, hard and auxiliary constraints, and an
    /// optional filter restricting which points may move.
    pub fn new(
        points: Vec<(u64, f64, f64, bool)>,
        hard: Vec<Box<dyn SketchConstraint>>,
        auxiliary: Vec<Box<dyn SketchConstraint>>,
        point_filter: Option<Vec<u64>>,
    ) -> Self {
        let mut point_ids = Vec::with_capacity(points.len());
        let mut fixed = Vec::with_capacity(points.len());
        let mut slot_of = HashMap::with_capacity(points.len());
        let mut coords = Vec::with_capacity(2 * points.len());
        for (id, x, y, is_fixed) in points {
            slot_of.insert(id, point_ids.len());
            point_ids.push(id);
            fixed.push(is_fixed);
            coords.push(x);
            coords.push(y);
        }
        let constrained = fixed.clone();
        SketchSolver {
            point_ids,
            fixed,
            slot_of,
            coords,
            constrained,
            hard,
            auxiliary,
            point_filter: point_filter.map(HashSet::from_iter),
        }
    }

    /// Runs the solve.  Returns the conjunction of all component
    /// results.  When `update_dof` is set, per-point constrained
    /// flags are re-analyzed on success.
    pub fn solve(&mut self, tolerance: f64, update_dof: bool) -> bool {
        if update_dof {
            self.constrained = self.fixed.clone();
        }

        if self.point_filter.is_none() {
            self.solve_decomposed(tolerance, update_dof)
        } else {
            let hard: Vec<usize> = (0..self.hard.len()).collect();
            let aux: Vec<usize> = (0..self.auxiliary.len()).collect();
            self.solve_scoped(&hard, &aux, None, tolerance, update_dof)
        }
    }

    /// Solves each connected constraint component independently.
    fn solve_decomposed(&mut self, tolerance: f64, update_dof: bool) -> bool {
        let parts = self.partition_constraints();
        if parts.is_empty() {
            return true;
        }

        let mut success = true;
        for (hard_idx, aux_idx, component) in parts {
            if !self.solve_scoped(
                &hard_idx,
                &aux_idx,
                Some(&component),
                tolerance,
                update_dof,
            ) {
                success = false;
            }
        }
        success
    }

    /// Splits the hard and auxiliary constraints into one sub-problem
    /// per connected component of the constraint graph.  Returns
    /// `(hard indices, auxiliary indices, component points)` tuples,
    /// one per non-empty part.
    fn partition_constraints(
        &self,
    ) -> Vec<(Vec<usize>, Vec<usize>, BTreeSet<u64>)> {
        let mut all_refs: Vec<&dyn SketchConstraint> = Vec::new();
        all_refs.extend(self.hard.iter().map(|c| c.as_ref()));
        all_refs.extend(self.auxiliary.iter().map(|c| c.as_ref()));
        let components = compute_constraint_components(&all_refs);

        let mut component_of: HashMap<u64, usize> = HashMap::new();
        for (index, component) in components.iter().enumerate() {
            for pid in component {
                component_of.insert(*pid, index);
            }
        }

        let mut hard_parts: Vec<Vec<usize>> =
            vec![Vec::new(); components.len()];
        let mut aux_parts: Vec<Vec<usize>> = vec![Vec::new(); components.len()];

        for (i, constraint) in self.hard.iter().enumerate() {
            for pid in constraint.referenced_points() {
                if let Some(&index) = component_of.get(&pid) {
                    hard_parts[index].push(i);
                    break;
                }
            }
        }
        for (i, constraint) in self.auxiliary.iter().enumerate() {
            for pid in constraint.referenced_points() {
                if let Some(&index) = component_of.get(&pid) {
                    aux_parts[index].push(i);
                    break;
                }
            }
        }

        let mut parts = Vec::new();
        for (index, component) in components.into_iter().enumerate() {
            let hard = std::mem::take(&mut hard_parts[index]);
            let aux = std::mem::take(&mut aux_parts[index]);
            if !hard.is_empty() || !aux.is_empty() {
                parts.push((hard, aux, component));
            }
        }
        parts
    }

    /// Solves this scoped constraint set.  `component` further
    /// restricts which points may move (used for decomposed global
    /// solves); `None` falls back to this solver's own point filter.
    fn solve_scoped(
        &mut self,
        hard_idx: &[usize],
        aux_idx: &[usize],
        component: Option<&BTreeSet<u64>>,
        tolerance: f64,
        update_dof: bool,
    ) -> bool {
        let hard_specs: Vec<&dyn SketchConstraint> =
            hard_idx.iter().map(|&i| self.hard[i].as_ref()).collect();
        let aux_specs: Vec<&dyn SketchConstraint> = aux_idx
            .iter()
            .map(|&i| self.auxiliary[i].as_ref())
            .collect();

        let mut touched: HashSet<u64> = HashSet::new();
        for spec in hard_specs.iter().chain(aux_specs.iter()) {
            touched.extend(spec.referenced_points());
        }

        let mut mutable_ids: Vec<u64> = Vec::new();
        for (i, &id) in self.point_ids.iter().enumerate() {
            let allowed = if let Some(set) = component {
                set.contains(&id)
            } else if let Some(filter) = &self.point_filter {
                filter.contains(&id)
            } else {
                true
            };
            if self.fixed[i] || !touched.contains(&id) || !allowed {
                continue;
            }
            mutable_ids.push(id);
        }
        if mutable_ids.is_empty() {
            return true;
        }

        let mut state_slots = Vec::with_capacity(mutable_ids.len());
        let mut state_col_of: HashMap<u64, usize> =
            HashMap::with_capacity(mutable_ids.len());
        let mut x0 = Vec::with_capacity(2 * mutable_ids.len());
        for (col, id) in mutable_ids.iter().enumerate() {
            let slot = self.slot_of[id];
            state_slots.push(slot);
            state_col_of.insert(*id, 2 * col);
            x0.push(self.coords[2 * slot]);
            x0.push(self.coords[2 * slot + 1]);
        }

        let all_specs: Vec<&dyn SketchConstraint> = hard_specs
            .iter()
            .copied()
            .chain(aux_specs.iter().copied())
            .collect();

        let mut residual_count = 0usize;
        for constraint in &all_specs {
            residual_count += constraint.residual_count();
        }
        if residual_count == 0 {
            residual_count = 1;
        }

        // Run the solve on a private copy of the coordinates, then
        // write the result back.  The evaluator sits in a RefCell so
        // the residual and Jacobian closures can share it.
        let evaluator = std::cell::RefCell::new(Evaluator::new(
            self.slot_of.clone(),
            self.coords.clone(),
        ));
        let n_vars = x0.len();
        let mut fun = |state: &[f64]| -> Vec<f64> {
            let mut ev = evaluator.borrow_mut();
            ev.set_state(state, &state_slots);
            ev.residuals(&all_specs)
        };
        let mut jac = |state: &[f64]| -> Vec<f64> {
            let mut ev = evaluator.borrow_mut();
            ev.set_state(state, &state_slots);
            ev.jacobian(&all_specs, n_vars, &state_col_of)
        };

        let result = solve_trf_no_bounds(
            TrfProblem {
                fun: Box::new(&mut fun),
                jac: Box::new(&mut jac),
                m: residual_count,
            },
            &x0,
            tolerance,
            1e-8,
            1e-8,
            MAX_NFEV,
        );

        evaluator.borrow_mut().set_state(&result.x, &state_slots);
        let final_coords = evaluator.borrow().coords.clone();
        for &slot in &state_slots {
            self.coords[2 * slot] = final_coords[2 * slot];
            self.coords[2 * slot + 1] = final_coords[2 * slot + 1];
        }

        let success = result.status > 0 && result.cost <= tolerance;

        if success && update_dof {
            let mut dof_evaluator =
                Evaluator::new(self.slot_of.clone(), self.coords.clone());
            let flags = analyze_dof_flags(
                &hard_specs,
                &mut dof_evaluator,
                &state_col_of,
                n_vars,
                &mutable_ids,
            );
            for (&id, flag) in mutable_ids.iter().zip(flags) {
                if let Some(&slot) = self.slot_of.get(&id) {
                    self.constrained[slot] = flag;
                }
            }
        }

        success
    }

    /// Returns the final `(x, y)` positions, aligned with the input
    /// point order.
    pub fn positions(&self) -> Vec<(f64, f64)> {
        self.point_ids
            .iter()
            .map(|&id| {
                let slot = self.slot_of[&id];
                (self.coords[2 * slot], self.coords[2 * slot + 1])
            })
            .collect()
    }

    /// Returns per-point fully-constrained flags, aligned with the
    /// input point order.
    pub fn constrained_flags(&self) -> Vec<bool> {
        self.constrained.clone()
    }

    /// Returns the indices of hard constraints whose maximum absolute
    /// residual exceeds `threshold` at the final positions.
    pub fn conflicting(&self, threshold: f64) -> Vec<usize> {
        let lookup = Evaluator::new(self.slot_of.clone(), self.coords.clone());
        let constraints: Vec<&dyn SketchConstraint> =
            self.hard.iter().map(|c| c.as_ref()).collect();
        lookup
            .max_residuals(&constraints)
            .into_iter()
            .enumerate()
            .filter(|(_, value)| *value > threshold)
            .map(|(index, _)| index)
            .collect()
    }

    /// Returns all residual values of the hard constraints at the
    /// current positions, in constraint order.  For tests and
    /// debugging.
    pub fn residuals(&self) -> Vec<f64> {
        let lookup = Evaluator::new(self.slot_of.clone(), self.coords.clone());
        let constraints: Vec<&dyn SketchConstraint> =
            self.hard.iter().map(|c| c.as_ref()).collect();
        lookup.residuals(&constraints)
    }

    /// Returns the maximum absolute residual per hard constraint at
    /// the final positions (aligned with the hard constraint order).
    pub fn max_residuals(&self) -> Vec<f64> {
        let lookup = Evaluator::new(self.slot_of.clone(), self.coords.clone());
        let constraints: Vec<&dyn SketchConstraint> =
            self.hard.iter().map(|c| c.as_ref()).collect();
        lookup.max_residuals(&constraints)
    }

    /// Returns the dense Jacobian (row-major per row vector) of the
    /// hard constraints at the current positions, over the state
    /// points (non-fixed points touched by any constraint, in input
    /// order).  For tests and debugging.
    pub fn jacobian(&self) -> Vec<Vec<f64>> {
        let mut touched: HashSet<u64> = HashSet::new();
        let all: Vec<&dyn SketchConstraint> = self
            .hard
            .iter()
            .chain(self.auxiliary.iter())
            .map(|c| c.as_ref())
            .collect();
        for constraint in &all {
            touched.extend(constraint.referenced_points());
        }
        let mut state_ids: Vec<u64> = Vec::new();
        let mut state_col_of: HashMap<u64, usize> = HashMap::new();
        for (i, &id) in self.point_ids.iter().enumerate() {
            if self.fixed[i] || !touched.contains(&id) {
                continue;
            }
            state_col_of.insert(id, 2 * state_ids.len());
            state_ids.push(id);
        }
        let mut evaluator =
            Evaluator::new(self.slot_of.clone(), self.coords.clone());
        let n_vars = 2 * state_ids.len();
        let hard: Vec<&dyn SketchConstraint> =
            self.hard.iter().map(|c| c.as_ref()).collect();
        let data = evaluator.jacobian(&hard, n_vars, &state_col_of);
        data.chunks(n_vars).map(|row| row.to_vec()).collect()
    }
}

/// Analyzes the degrees of freedom on the hard-constraint Jacobian
/// and returns per-point constrained flags for `mutable_ids`
/// (mirroring the null-space mobility rule of the reference solver).
fn analyze_dof_flags(
    hard_specs: &[&dyn SketchConstraint],
    evaluator: &mut Evaluator,
    state_col_of: &HashMap<u64, usize>,
    n_vars: usize,
    mutable_ids: &[u64],
) -> Vec<bool> {
    let mut rows = 0usize;
    for constraint in hard_specs {
        rows += constraint.residual_count();
    }
    if rows == 0 {
        return vec![false; mutable_ids.len()];
    }

    let jac_data = evaluator.jacobian(hard_specs, n_vars, state_col_of);
    let jacobian = DMatrix::from_row_slice(rows, n_vars, &jac_data);
    let null_space = null_space_basis(&jacobian, DOF_RCOND);

    let mut flags = vec![false; mutable_ids.len()];
    if null_space.ncols() == 0 {
        for flag in flags.iter_mut() {
            *flag = true;
        }
        return flags;
    }

    for (i, flag) in flags.iter_mut().enumerate() {
        let idx_x = i * 2;
        if idx_x >= null_space.nrows() {
            break;
        }
        let idx_y = idx_x + 1;
        let mut x_mobility = 0.0;
        let mut y_mobility = 0.0;
        for col in 0..null_space.ncols() {
            x_mobility += null_space[(idx_x, col)].abs();
            y_mobility += null_space[(idx_y, col)].abs();
        }
        *flag =
            x_mobility < MOBILITY_THRESHOLD && y_mobility < MOBILITY_THRESHOLD;
    }
    flags
}

/// Orthonormal basis of the null space of `j`, mirroring
/// `scipy.linalg.null_space(j, rcond)` including its cutoff formula.
fn null_space_basis(j: &DMatrix<f64>, rcond: f64) -> DMatrix<f64> {
    let (m, n) = j.shape();

    // Pad with zero rows to n rows so that the economy SVD of the
    // square matrix yields the complete right singular basis (SciPy
    // requests full_matrices=True; the cutoff uses max(m, n), which
    // is unchanged by the padding).
    let padded = if m < n {
        let mut full = DMatrix::zeros(n, n);
        full.view_mut((0, 0), (m, n)).copy_from(j);
        full
    } else {
        j.clone()
    };

    let svd = SVD::new(padded, false, true);
    let v_t = match svd.v_t {
        Some(v_t) if v_t.ncols() == n => v_t,
        _ => return DMatrix::zeros(n, 0),
    };
    let s = svd.singular_values;

    let smax = s.iter().fold(0.0f64, |acc, v| acc.max(*v));
    let tol = (m.max(n)) as f64 * smax * rcond;
    let rank = s.iter().filter(|&&value| value > tol).count();

    // SciPy returns the null-space basis as the transpose of the
    // trailing rows of VT, i.e. the columns of V past the rank.
    v_t.rows(rank, n - rank).transpose().into_owned()
}
