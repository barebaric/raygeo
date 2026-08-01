use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::geo::types::Point;
use crate::ops::cut;
use crate::python::geo::flex_point::polygons_from_tuples;
use crate::python::ops::part::cleared_area::PyClearedArea;

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
    /// ``step``.
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

/// Configuration options for :func:`step`.
///
/// Holds the constant parameters for the adaptive stepper.
#[gen_stub_pyclass(module = "raygeo.ops.cut.stepper")]
#[pyclass(name = "StepperOptions", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyStepperOptions {
    /// Target cut-area per unit distance.
    #[pyo3(get, set)]
    pub target_area_pd: f64,
    /// Forward step length in mm.
    #[pyo3(get, set)]
    pub step_length: f64,
    /// Disk radius in mm.
    #[pyo3(get, set)]
    pub radius: f64,
    /// Maximum steering deflection in radians.
    #[pyo3(get, set)]
    pub max_deflection: f64,
    /// Valid tool-centre region polygons.
    #[pyo3(get, set)]
    pub valid_area: Vec<Vec<(f64, f64)>>,
    /// Minimum trial deflection angle in radians.
    #[pyo3(get, set)]
    pub angle_min: f64,
    /// Maximum trial deflection angle in radians.
    #[pyo3(get, set)]
    pub angle_max: f64,
    /// Directional bias sign: ``+1.0`` for CW, ``-1.0`` for CCW,
    /// ``0.0`` for no bias.
    #[pyo3(get, set)]
    pub dir_sign: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStepperOptions {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        target_area_pd,
        step_length,
        radius,
        max_deflection,
        valid_area,
        angle_min = -cut::stepper::STEP_ANGLE_BOUND,
        angle_max = cut::stepper::STEP_ANGLE_BOUND,
        dir_sign = 0.0,
    ))]
    fn new(
        target_area_pd: f64,
        step_length: f64,
        radius: f64,
        max_deflection: f64,
        valid_area: Vec<Vec<(f64, f64)>>,
        angle_min: f64,
        angle_max: f64,
        dir_sign: f64,
    ) -> Self {
        Self {
            target_area_pd,
            step_length,
            radius,
            max_deflection,
            valid_area,
            angle_min,
            angle_max,
            dir_sign,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "StepperOptions(target_apd={:.4}, step_len={:.3}, R={:.1}, \
             max_def={:.2}, angle_min={:.3}, angle_max={:.3}, dir_sign={:+.1})",
            self.target_area_pd,
            self.step_length,
            self.radius,
            self.max_deflection,
            self.angle_min,
            self.angle_max,
            self.dir_sign,
        )
    }
}

/// Perform one forward step using the area-based adaptive solver.
///
/// :param cleared: ``ClearedArea`` instance.
/// :param pos: Current centre position ``(x, y)``.
/// :param heading: Smoothed heading angle (radians).
/// :param predicted_angle: Predicted steering angle from history.
/// :param opts: ``StepperOptions`` instance with fixed parameters.
/// :returns: ``StepResult`` with the next position and updated heading.
#[gen_stub_pyfunction(module = "raygeo.ops.cut.stepper")]
#[pyfunction(name = "step")]
fn step_py(
    cleared: &PyClearedArea,
    pos: (f64, f64),
    heading: f64,
    predicted_angle: f64,
    opts: &PyStepperOptions,
) -> PyStepResult {
    let valid = polygons_from_tuples(opts.valid_area.clone());
    let rust_opts = cut::StepperOptions {
        target_area_pd: opts.target_area_pd,
        step_length: opts.step_length,
        radius: opts.radius,
        max_deflection: opts.max_deflection,
        valid_area: &valid,
        angle_min: opts.angle_min,
        angle_max: opts.angle_max,
        dir_sign: opts.dir_sign,
    };
    let r = cut::stepper::step(
        &cleared.inner,
        Point::new(pos.0, pos.1),
        heading,
        predicted_angle,
        &rust_opts,
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
    m.add_class::<PyStepperOptions>()?;
    m.add_function(wrap_pyfunction!(step_py, &m)?)?;

    cut_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.cut.stepper", &m)?;

    Ok(())
}
