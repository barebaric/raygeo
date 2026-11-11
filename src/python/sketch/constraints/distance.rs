//! Bindings for the distance constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::distance::DistanceConstraint as CoreConstraint;

/// Enforces a fixed distance between two points.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.distance",
    name = "DistanceConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyDistanceConstraint {
    /// First point.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point.
    #[pyo3(get)]
    pub p2: u64,
    /// Resolved distance value.
    #[pyo3(get)]
    pub value: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDistanceConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p1, p2, value))]
    fn new(p1: u64, p2: u64, value: f64) -> Self {
        PyDistanceConstraint { p1, p2, value }
    }
}

impl PyDistanceConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.p1, self.p2, self.value)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDistanceConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.distance", m)?;
    Ok(())
}
