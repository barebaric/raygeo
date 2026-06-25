use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

/// A resume point found on the cleared-area frontier.
#[gen_stub_pyclass(module = "raygeo.ops.area")]
#[pyclass(name = "ResumePoint", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyResumePoint {
    /// Position on the frontier ``(x, y)``.
    #[pyo3(get)]
    pub pos: (f64, f64),
    /// Outward-normal heading (radians).
    #[pyo3(get)]
    pub heading: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyResumePoint {
    /// :param pos: Position on the frontier ``(x, y)``.
    /// :param heading: Outward-normal heading in radians.
    #[new]
    #[pyo3(signature = (pos, heading))]
    pub fn new(pos: (f64, f64), heading: f64) -> Self {
        PyResumePoint { pos, heading }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "ResumePoint(pos=({:.3},{:.3}), heading={:.3})",
            self.pos.0, self.pos.1, self.heading,
        )
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyResumePoint>()?;
    Ok(())
}
