//! Bindings for the horizontal constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::horizontal::HorizontalConstraint as CoreConstraint;

/// Enforces two points have the same Y coordinate.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.horizontal",
    name = "HorizontalConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyHorizontalConstraint {
    /// First point.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point.
    #[pyo3(get)]
    pub p2: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyHorizontalConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p1, p2))]
    fn new(p1: u64, p2: u64) -> Self {
        PyHorizontalConstraint { p1, p2 }
    }
}

impl PyHorizontalConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.p1, self.p2)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyHorizontalConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.horizontal", m)?;
    Ok(())
}
