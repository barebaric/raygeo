use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::cut;
use crate::python::geo::flex_point::polygons_from_tuples;
use crate::python::ops::cut::cleared_area::PyClearedArea;
use crate::types::Point;

/// Status of a single step or cut segment.
///
/// One of ``Ok`` (normal), ``BoundaryHit`` (hit pocket boundary),
/// ``LostEngagement`` (no uncut material), or ``NoConvergence``
/// (solver failed to converge).
#[gen_stub_pyclass(module = "raygeo.ops.cut.stepper")]
#[pyclass(name = "StepStatus", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyStepStatus {
    pub inner: cut::StepStatus,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStepStatus {
    /// Normal step completion.
    /// :returns: ``StepStatus.ok``
    #[classmethod]
    fn ok(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: cut::StepStatus::Ok,
        }
    }
    /// Hit pocket boundary.
    /// :returns: ``StepStatus.boundary_hit``
    #[classmethod]
    fn boundary_hit(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: cut::StepStatus::BoundaryHit,
        }
    }
    /// No uncut material found.
    /// :returns: ``StepStatus.lost_engagement``
    #[classmethod]
    fn lost_engagement(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: cut::StepStatus::LostEngagement,
        }
    }
    /// Solver failed to converge.
    /// :returns: ``StepStatus.no_convergence``
    #[classmethod]
    fn no_convergence(_cls: &Bound<'_, PyType>) -> Self {
        PyStepStatus {
            inner: cut::StepStatus::NoConvergence,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Result of a single forward step.
///
/// Contains the next centre position, updated heading,
/// solver iteration count, and the final status.
#[gen_stub_pyclass(module = "raygeo.ops.cut.stepper")]
#[pyclass(name = "StepResult", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyStepResult {
    /// Next centre position ``(x, y)``.
    #[pyo3(get)]
    pub next: (f64, f64),
    /// Updated heading angle in radians.
    #[pyo3(get)]
    pub heading: f64,
    /// The incremental cut area (crescent) for this step.
    #[pyo3(get)]
    pub cut_area: f64,
    /// Number of solver iterations used.
    #[pyo3(get)]
    pub iters: usize,
    /// Solver steering angle (radians). Only non-zero for
    /// ``step_adaptive``.
    #[pyo3(get)]
    pub iteration_angle: f64,
    /// Step completion status.
    #[pyo3(get)]
    pub status: PyStepStatus,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStepResult {
    fn __repr__(&self) -> String {
        format!(
            "StepResult(next=({:.3},{:.3}), heading={:.3}, \
             status={:?})",
            self.next.0, self.next.1, self.heading, self.status.inner,
        )
    }
}

/// Perform one forward step using the area-based adaptive solver.
///
/// Like :func:`step`, but targets **cut-area per unit distance**
/// rather than an engagement angle.  Used internally by
/// ``adaptive_clearing``.
///
/// :param cleared: ``ClearedArea`` instance.
/// :param pos: Current centre position ``(x, y)``.
/// :param heading: Smoothed heading angle (radians).
/// :param predicted_angle: Predicted steering angle from history.
/// :param target_area_pd: Target cut-area per unit distance.
/// :param step_length: Forward step length in mm.
/// :param radius: Disk radius in mm.
/// :param max_deflection: Max steering deflection in radians.
/// :param valid_area: Valid tool-centre region polygons.
/// :param angle_min: Minimum trial deflection angle in radians (default -π/4).
/// :param angle_max: Maximum trial deflection angle in radians (default +π/4).
/// :param dir_sign: Directional bias sign (default ``0.0``).  ``+1.0``
///    to prefer positive angles (CW), ``−1.0`` to prefer negative
///    angles (CCW).  The bias penalises fresh material on the wrong
///    side when the tool breaks through a web between two cleared
///    regions.  Has no effect during normal one-sided cutting.
/// :returns: ``StepResult`` with the next position and updated heading.
#[gen_stub_pyfunction(module = "raygeo.ops.cut.stepper")]
#[pyfunction(name = "step_adaptive")]
#[pyo3(signature = (
    cleared,
    pos,
    heading,
    predicted_angle,
    target_area_pd,
    step_length,
    radius,
    max_deflection,
    valid_area,
    angle_min = -std::f64::consts::FRAC_PI_4,
    angle_max = std::f64::consts::FRAC_PI_4,
    dir_sign = 0.0,
))]
#[allow(clippy::too_many_arguments)]
fn step_adaptive_py(
    cleared: &PyClearedArea,
    pos: (f64, f64),
    heading: f64,
    predicted_angle: f64,
    target_area_pd: f64,
    step_length: f64,
    radius: f64,
    max_deflection: f64,
    valid_area: Vec<Vec<(f64, f64)>>,
    angle_min: f64,
    angle_max: f64,
    dir_sign: f64,
) -> PyStepResult {
    let valid = polygons_from_tuples(valid_area);
    let r = cut::stepper::step_adaptive(
        &cleared.inner,
        Point::new(pos.0, pos.1),
        heading,
        predicted_angle,
        target_area_pd,
        step_length,
        radius,
        max_deflection,
        &valid,
        angle_min,
        angle_max,
        dir_sign,
    );
    PyStepResult {
        next: (r.next.x, r.next.y),
        heading: r.heading,
        cut_area: r.cut_area,
        iters: r.iters,
        iteration_angle: r.iteration_angle,
        status: PyStepStatus { inner: r.status },
    }
}

pub fn register(cut_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = cut_mod.py();
    let m = PyModule::new(py, "stepper")?;

    m.add_class::<PyStepStatus>()?;
    m.add_class::<PyStepResult>()?;
    m.add_function(wrap_pyfunction!(step_adaptive_py, &m)?)?;

    cut_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.cut.stepper", &m)?;

    Ok(())
}
