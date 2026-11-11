//! Python bindings for the sketch constraint solver.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraint::SketchConstraint;
use crate::sketch::solver::SketchSolver;

use super::constraints::{
    angle, aspect_ratio, coincident, collinear, diameter, distance, drag,
    equal_distance, equal_length, horizontal, parallelogram, perpendicular,
    point_on_curve, point_on_line, radius, rotational, symmetry, tangent,
    vertical,
};
use super::point::PyPointDef;

/// Converts a Python constraint object into its core representation.
fn constraint_from_py(
    obj: &Bound<'_, PyAny>,
) -> PyResult<Box<dyn SketchConstraint>> {
    macro_rules! try_extract {
        ($ty:ty) => {
            if let Ok(c) = obj.extract::<$ty>() {
                return Ok(Box::new(c.into_constraint()));
            }
        };
    }
    try_extract!(angle::PyAngleConstraint);
    try_extract!(aspect_ratio::PyAspectRatioConstraint);
    try_extract!(coincident::PyCoincidentConstraint);
    try_extract!(collinear::PyCollinearConstraint);
    try_extract!(diameter::PyDiameterConstraint);
    try_extract!(distance::PyDistanceConstraint);
    try_extract!(drag::PyDragConstraint);
    try_extract!(equal_distance::PyEqualDistanceConstraint);
    try_extract!(equal_length::PyEqualLengthConstraint);
    try_extract!(horizontal::PyHorizontalConstraint);
    try_extract!(parallelogram::PyParallelogramConstraint);
    try_extract!(perpendicular::PyPerpendicularConstraint);
    try_extract!(point_on_curve::PyPointOnCurveConstraint);
    try_extract!(point_on_line::PyPointOnLineConstraint);
    try_extract!(radius::PyRadiusConstraint);
    try_extract!(rotational::PyRotationalConstraint);
    try_extract!(symmetry::PySymmetryConstraint);
    try_extract!(tangent::PyTangentConstraint);
    try_extract!(vertical::PyVerticalConstraint);
    Err(PyTypeError::new_err(
        "expected a raygeo.sketch.constraints constraint object",
    ))
}

fn constraints_from_py(
    list: Vec<Bound<'_, PyAny>>,
) -> PyResult<Vec<Box<dyn SketchConstraint>>> {
    list.iter().map(|obj| constraint_from_py(obj)).collect()
}

/// Trust-region least-squares solver for sketch constraint systems.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch", name = "Solver", skip_from_py_object)]
pub struct PySketchSolver {
    inner: SketchSolver,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySketchSolver {
    /// Builds a solver from point definitions, hard constraints,
    /// auxiliary (weak) constraints, and an optional point filter.
    #[new]
    #[pyo3(signature = (points, hard, auxiliary = None, point_filter = None))]
    fn new(
        points: Vec<PyPointDef>,
        hard: Vec<Bound<'_, PyAny>>,
        auxiliary: Option<Vec<Bound<'_, PyAny>>>,
        point_filter: Option<Vec<u64>>,
    ) -> PyResult<Self> {
        let point_defs =
            points.iter().map(|p| (p.id, p.x, p.y, p.fixed)).collect();
        let hard_constraints = constraints_from_py(hard)?;
        let aux_constraints = match auxiliary {
            Some(list) => constraints_from_py(list)?,
            None => Vec::new(),
        };
        Ok(PySketchSolver {
            inner: SketchSolver::new(
                point_defs,
                hard_constraints,
                aux_constraints,
                point_filter,
            ),
        })
    }

    /// Runs the solve.  Returns True when all components converged.
    ///
    /// :param tolerance: Cost threshold and ftol of the optimizer.
    /// :param update_dof: Whether to re-analyze per-point constrained
    ///     flags after a successful solve.
    fn solve(&mut self, tolerance: f64, update_dof: bool) -> bool {
        self.inner.solve(tolerance, update_dof)
    }

    /// Indices of hard constraints whose residual exceeds
    /// `threshold` at the final positions.
    fn conflicting(&self, threshold: f64) -> Vec<usize> {
        self.inner.conflicting(threshold)
    }

    /// Final `(x, y)` positions, aligned with the input point order.
    fn positions(&self) -> Vec<(f64, f64)> {
        self.inner.positions()
    }

    /// Per-point fully-constrained flags, aligned with the input
    /// point order.
    fn constrained(&self) -> Vec<bool> {
        self.inner.constrained_flags()
    }

    /// All residual values of the hard constraints at the current
    /// positions, in constraint order.
    fn residuals(&self) -> Vec<f64> {
        self.inner.residuals()
    }

    /// Maximum absolute residual per hard constraint.
    fn max_residuals(&self) -> Vec<f64> {
        self.inner.max_residuals()
    }

    /// Dense Jacobian rows of the hard constraints at the current
    /// positions, over the state points (non-fixed points touched by
    /// any constraint, in input order).  For tests and debugging.
    fn jacobian(&self) -> Vec<Vec<f64>> {
        self.inner.jacobian()
    }
}
