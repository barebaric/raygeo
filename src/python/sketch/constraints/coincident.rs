//! Bindings for the coincident constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::coincident::CoincidentConstraint as CoreConstraint;

/// Enforces two points are at the same location.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.coincident",
    name = "CoincidentConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyCoincidentConstraint {
    /// First point.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point.
    #[pyo3(get)]
    pub p2: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCoincidentConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p1, p2))]
    fn new(p1: u64, p2: u64) -> Self {
        PyCoincidentConstraint { p1, p2 }
    }
}

impl PyCoincidentConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.p1, self.p2)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCoincidentConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.coincident", m)?;
    Ok(())
}
