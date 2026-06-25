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
    /// Travel polyline through cleared territory.
    #[pyo3(get)]
    pub link_path: Vec<(f64, f64)>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyResumePoint {
    /// :param pos: Position on the frontier ``(x, y)``.
    /// :param heading: Outward-normal heading in radians.
    /// :param link_path: Travel polyline through cleared territory.
    #[new]
    #[pyo3(signature = (pos, heading, link_path))]
    pub fn new(
        pos: (f64, f64),
        heading: f64,
        link_path: Vec<(f64, f64)>,
    ) -> Self {
        PyResumePoint {
            pos,
            heading,
            link_path,
        }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "ResumePoint(pos=({:.3},{:.3}), heading={:.3}, link_len={})",
            self.pos.0,
            self.pos.1,
            self.heading,
            self.link_path.len(),
        )
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyResumePoint>()?;
    Ok(())
}
