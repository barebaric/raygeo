//! Bindings for the collinear constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::collinear::CollinearConstraint as CoreConstraint;

/// Enforces three points are collinear.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.collinear",
    name = "CollinearConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyCollinearConstraint {
    /// First point.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point.
    #[pyo3(get)]
    pub p2: u64,
    /// Third point.
    #[pyo3(get)]
    pub p3: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCollinearConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p1, p2, p3))]
    fn new(p1: u64, p2: u64, p3: u64) -> Self {
        PyCollinearConstraint { p1, p2, p3 }
    }
}

impl PyCollinearConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.p1, self.p2, self.p3)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCollinearConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.collinear", m)?;
    Ok(())
}
