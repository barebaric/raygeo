//! Python bindings for sketch point definitions.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

/// Definition of one sketch point.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch",
    name = "PointDef"
)]
#[derive(Clone, PartialEq)]
pub struct PyPointDef {
    /// Unique point id, stable across solves.
    #[pyo3(get)]
    pub id: u64,
    /// Current X coordinate.
    #[pyo3(get)]
    pub x: f64,
    /// Current Y coordinate.
    #[pyo3(get)]
    pub y: f64,
    /// Whether the point is pinned and never moves.
    #[pyo3(get)]
    pub fixed: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPointDef {
    /// Creates a point definition.
    #[new]
    fn new(id: u64, x: f64, y: f64, fixed: bool) -> Self {
        PyPointDef { id, x, y, fixed }
    }
}
