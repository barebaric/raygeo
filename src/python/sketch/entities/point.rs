//! Bindings for the sketch point.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

/// A sketch point: coordinates and solver flags.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch.entities.point", name = "SketchPoint")]
pub struct PySketchPoint {
    pub inner: crate::sketch::entities::point::SketchPoint,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySketchPoint {
    /// Unique point id, stable across solves.
    #[getter]
    fn id(&self) -> u64 {
        self.inner.id
    }

    /// X coordinate.
    #[getter]
    fn x(&self) -> f64 {
        self.inner.x
    }

    /// Y coordinate.
    #[getter]
    fn y(&self) -> f64 {
        self.inner.y
    }

    /// Whether the point is pinned and never moves.
    #[getter]
    fn fixed(&self) -> bool {
        self.inner.fixed
    }

    /// Whether the point is fully constrained, as computed by the
    /// solver.
    #[getter]
    fn constrained(&self) -> bool {
        self.inner.constrained
    }

    /// Returns a readable representation.
    fn __repr__(&self) -> String {
        format!(
            "SketchPoint(id={}, x={}, y={}, fixed={})",
            self.inner.id, self.inner.x, self.inner.y, self.inner.fixed
        )
    }
}

/// Registers the point class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySketchPoint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities.point", m)?;
    Ok(())
}
