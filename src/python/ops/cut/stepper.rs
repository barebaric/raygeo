use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::ops::cut;
use crate::ops::cut::stepper::EngagementMetric;
use crate::python::geo::flex_point::polygons_from_tuples;
use crate::python::ops::cut::cleared_area::PyClearedArea;
use crate::types::Point;

/// Options for the stepping solver.
///
/// Controls disk radius, step length, target engagement angle,
/// solver tolerance, max steering deflection, and iteration budget.
#[gen_stub_pyclass(module = "raygeo.ops.cut.stepper")]
#[pyclass(name = "StepperOptions", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyStepperOptions {
    pub inner: cut::StepperOptions,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStepperOptions {
    /// :param radius: Disk radius in mm (default 3.0).
    /// :param step_length: Forward step length in mm (default 0.6).
    /// :param target_engagement: Target engagement angle in radians
    ///     (default π).
    /// :param engagement_tol: Engagement tolerance in radians
    ///     (default 0.01).
    /// :param max_deflection: Maximum steering deflection per step in
    ///     radians (default π/6).
    /// :param max_solver_iters: Maximum solver iterations per step
    ///     (default 6).
    #[new]
    #[pyo3(signature = (
        radius = 3.0,
        step_length = 0.6,
        target_engagement = None,
        engagement_tol = 0.01,
        max_deflection = None,
        max_solver_iters = 6,
    ))]
    pub fn new(
        radius: f64,
        step_length: f64,
        target_engagement: Option<f64>,
        engagement_tol: f64,
        max_deflection: Option<f64>,
        max_solver_iters: usize,
    ) -> Self {
        let target = target_engagement.unwrap_or(std::f64::consts::PI);
        let max_def = max_deflection.unwrap_or(std::f64::consts::FRAC_PI_6);
        PyStepperOptions {
            inner: cut::StepperOptions {
                radius,
                step_length,
                target_engagement: target,
                engagement_tol,
                max_deflection: max_def,
                max_solver_iters,
                valid_area: None,
                metric: EngagementMetric::Angle,
            },
        }
    }

    /// Disk radius in mm.
    #[getter]
    pub fn get_radius(&self) -> f64 {
        self.inner.radius
    }
    #[setter]
    pub fn set_radius(&mut self, v: f64) {
        self.inner.radius = v;
    }
    /// Forward step length in mm.
    #[getter]
    pub fn get_step_length(&self) -> f64 {
        self.inner.step_length
    }
    #[setter]
    pub fn set_step_length(&mut self, v: f64) {
        self.inner.step_length = v;
    }
    /// Target engagement angle in radians.
    #[getter]
    pub fn get_target_engagement(&self) -> f64 {
        self.inner.target_engagement
    }
    #[setter]
    pub fn set_target_engagement(&mut self, v: f64) {
        self.inner.target_engagement = v;
    }
    /// Engagement tolerance in radians.
    #[getter]
    pub fn get_engagement_tol(&self) -> f64 {
        self.inner.engagement_tol
    }
    #[setter]
    pub fn set_engagement_tol(&mut self, v: f64) {
        self.inner.engagement_tol = v;
    }
    /// Maximum steering deflection per step in radians.
    #[getter]
    pub fn get_max_deflection(&self) -> f64 {
        self.inner.max_deflection
    }
    #[setter]
    pub fn set_max_deflection(&mut self, v: f64) {
        self.inner.max_deflection = v;
    }
    /// Maximum solver iterations per step.
    #[getter]
    pub fn get_max_solver_iters(&self) -> usize {
        self.inner.max_solver_iters
    }
    #[setter]
    pub fn set_max_solver_iters(&mut self, v: usize) {
        self.inner.max_solver_iters = v;
    }
    /// Engagement metric: ``"angle"`` (default) or ``"area"``.
    #[getter]
    pub fn get_metric(&self) -> String {
        match self.inner.metric {
            EngagementMetric::Angle => "angle".to_string(),
            EngagementMetric::Area => "area".to_string(),
        }
    }
    #[setter]
    pub fn set_metric(&mut self, v: &str) {
        self.inner.metric = match v {
            "area" => EngagementMetric::Area,
            _ => EngagementMetric::Angle,
        };
    }

    fn __repr__(&self) -> String {
        format!(
            "StepperOptions(R={}, step={}, target={:.3}, \
             max_def={:.3}, iters={}, metric={})",
            self.inner.radius,
            self.inner.step_length,
            self.inner.target_engagement,
            self.inner.max_deflection,
            self.inner.max_solver_iters,
            self.get_metric(),
        )
    }
}

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
    /// Solver steering angle (radians). Non-zero for ``step_adaptive``;
    /// always 0 for ``step``.
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

/// Perform one forward step.
///
/// Starting from *pos* with the given *heading* (radians), proposes
/// candidate positions and solves for the heading that maintains the
/// target engagement.
///
/// :param cleared: ``ClearedArea`` instance.
/// :param pos: Current centre position ``(x, y)``.
/// :param heading: Current heading angle in radians.
/// :param opts: ``StepperOptions`` controlling the solver.
/// :returns: ``StepResult`` with the next position and updated heading.
#[gen_stub_pyfunction(module = "raygeo.ops.cut.stepper")]
#[pyfunction(name = "step")]
fn step_py(
    cleared: &PyClearedArea,
    pos: (f64, f64),
    heading: f64,
    opts: &PyStepperOptions,
) -> PyStepResult {
    let r = cut::step(
        &cleared.inner,
        Point::new(pos.0, pos.1),
        heading,
        &opts.inner,
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

/// Drive the disk forward until a non-Ok status or *max_steps*.
///
/// Does **not** modify the ClearedArea — the caller is responsible for
/// committing swept polygons.
///
/// :param cleared: ``ClearedArea`` instance.
/// :param start: Starting position ``(x, y)``.
/// :param initial_heading: Initial heading angle (radians).
/// :param opts: ``StepperOptions`` controlling the solver.
/// :param max_steps: Maximum number of steps.
/// :returns: ``(path, status_string)``.
#[gen_stub_pyfunction(module = "raygeo.ops.cut.stepper")]
#[pyfunction(name = "run_segment")]
fn run_segment_py(
    cleared: &PyClearedArea,
    start: (f64, f64),
    initial_heading: f64,
    opts: &PyStepperOptions,
    max_steps: usize,
) -> (Vec<(f64, f64)>, String) {
    let (path, status) = cut::run_segment(
        &cleared.inner,
        Point::new(start.0, start.1),
        initial_heading,
        &opts.inner,
        max_steps,
    );
    let path_out: Vec<(f64, f64)> =
        path.into_iter().map(|p| (p.x, p.y)).collect();
    (path_out, format!("{status:?}"))
}

/// Derive the target engagement angle from the advance ratio.
///
/// :param advance: Per-step forward distance (mm).
/// :param radius: Disk radius (mm).
/// :returns: Engagement angle in radians.
#[gen_stub_pyfunction(module = "raygeo.ops.cut.stepper")]
#[pyfunction(name = "target_engagement_from_advance")]
fn target_engagement_from_advance_py(advance: f64, radius: f64) -> f64 {
    cut::target_engagement_from_advance(advance, radius)
}

pub fn register(cut_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = cut_mod.py();
    let m = PyModule::new(py, "stepper")?;

    m.add_class::<PyStepperOptions>()?;
    m.add_class::<PyStepStatus>()?;
    m.add_class::<PyStepResult>()?;
    m.add_function(wrap_pyfunction!(step_py, &m)?)?;
    m.add_function(wrap_pyfunction!(step_adaptive_py, &m)?)?;
    m.add_function(wrap_pyfunction!(run_segment_py, &m)?)?;
    m.add_function(wrap_pyfunction!(target_engagement_from_advance_py, &m)?)?;

    cut_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.cut.stepper", &m)?;

    Ok(())
}
