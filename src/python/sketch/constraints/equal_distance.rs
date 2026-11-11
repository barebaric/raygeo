//! Bindings for the equal_distance constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::equal_distance::EqualDistanceConstraint as CoreConstraint;

/// Enforces dist(p1, p2) == dist(p3, p4).
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.equal_distance",
    name = "EqualDistanceConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyEqualDistanceConstraint {
    /// First point of the first pair.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point of the first pair.
    #[pyo3(get)]
    pub p2: u64,
    /// First point of the second pair.
    #[pyo3(get)]
    pub p3: u64,
    /// Second point of the second pair.
    #[pyo3(get)]
    pub p4: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEqualDistanceConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p1, p2, p3, p4))]
    fn new(p1: u64, p2: u64, p3: u64, p4: u64) -> Self {
        PyEqualDistanceConstraint { p1, p2, p3, p4 }
    }
}

impl PyEqualDistanceConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.p1, self.p2, self.p3, self.p4)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEqualDistanceConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.equal_distance", m)?;
    Ok(())
}
