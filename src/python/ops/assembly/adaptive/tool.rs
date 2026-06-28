//! Python wrapper for the adaptive-clearing [`Tool`] state.

use crate::ops::assembly::adaptive::tool::Tool;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

/// Cutting-tool state for adaptive clearing.
///
/// Holds the tool centre position, heading, and the steering
/// predictor / gyroscope buffers used to smooth the walking path.
/// Construct with ``Tool(pos, heading, radius)`` and feed direction
/// vectors via ``push_gyro`` / ``push_angle`` between solver steps.
#[gen_stub_pyclass(module = "raygeo.ops.assembly.adaptive.tool")]
#[pyclass(name = "Tool", skip_from_py_object)]
#[derive(Clone, Copy)]
pub struct PyTool {
    pub(crate) inner: Tool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTool {
    /// Create a new tool at *pos* with the given *heading* (radians)
    /// and *radius*.
    #[new]
    #[pyo3(signature = (pos, heading, radius))]
    fn new(pos: (f64, f64), heading: f64, radius: f64) -> Self {
        PyTool {
            inner: Tool::new(Point::new(pos.0, pos.1), heading, radius),
        }
    }

    /// Tool centre position ``(x, y)``.
    #[getter]
    fn pos(&self) -> (f64, f64) {
        (self.inner.pos.x, self.inner.pos.y)
    }

    /// Set the tool centre position ``(x, y)``.
    #[setter]
    fn set_pos(&mut self, value: (f64, f64)) {
        self.inner.pos = Point::new(value.0, value.1);
    }

    /// Current heading angle in radians.
    #[getter]
    fn heading(&self) -> f64 {
        self.inner.heading
    }

    /// Set the heading angle in radians.
    #[setter]
    fn set_heading(&mut self, value: f64) {
        self.inner.heading = value;
    }

    /// Tool radius in mm.
    #[getter]
    fn radius(&self) -> f64 {
        self.inner.radius
    }

    /// Gyroscope-smoothed heading (radians), averaged over recent
    /// direction vectors.
    fn smoothed_heading(&self) -> f64 {
        self.inner.smoothed_heading()
    }

    /// Push a direction vector ``(dx, dy)`` into the gyroscope buffer.
    fn push_gyro(&mut self, dir: (f64, f64)) {
        self.inner.push_gyro(Point::new(dir.0, dir.1));
    }

    /// Reset the gyroscope and predictor history to the current heading.
    fn reset_gyro(&mut self) {
        self.inner.reset_gyro();
    }

    /// Push a solver iteration-angle delta (radians) into the predictor
    /// ring buffer.
    fn push_angle(&mut self, delta: f64) {
        self.inner.push_angle(delta);
    }

    /// Update the decayed steering predictor with a converged deflection.
    fn update_predictor(&mut self, delta: f64) {
        self.inner.update_predictor(delta);
    }

    /// Predictor seed for the engagement solver, clamped to a fraction
    /// of *max_deflection*.
    fn predicted_angle(&self, max_deflection: f64) -> f64 {
        self.inner.predicted_angle(max_deflection)
    }

    /// Raw (un-clamped) predictor value.
    fn raw_predictor(&self) -> f64 {
        self.inner.raw_predictor()
    }

    fn __repr__(&self) -> String {
        format!(
            "Tool(pos=({:.3},{:.3}), heading={:.3}, radius={:.3})",
            self.inner.pos.x,
            self.inner.pos.y,
            self.inner.heading,
            self.inner.radius,
        )
    }
}

pub(crate) fn register(adaptive_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let tool_mod = PyModule::new(adaptive_mod.py(), "tool")?;
    tool_mod.add_class::<PyTool>()?;
    adaptive_mod.add_submodule(&tool_mod)?;

    let sys_modules = adaptive_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.adaptive.tool", &tool_mod)?;

    Ok(())
}
