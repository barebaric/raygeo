//! Bindings for the aspect_ratio constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::aspect_ratio::AspectRatioConstraint as CoreConstraint;

/// Enforces dist(p1, p2) == ratio * dist(p3, p4).
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.aspect_ratio",
    name = "AspectRatioConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyAspectRatioConstraint {
    /// First point of the reference segment.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point of the reference segment.
    #[pyo3(get)]
    pub p2: u64,
    /// First point of the scaled segment.
    #[pyo3(get)]
    pub p3: u64,
    /// Second point of the scaled segment.
    #[pyo3(get)]
    pub p4: u64,
    /// Resolved ratio value.
    #[pyo3(get)]
    pub ratio: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAspectRatioConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p1, p2, p3, p4, ratio))]
    fn new(p1: u64, p2: u64, p3: u64, p4: u64, ratio: f64) -> Self {
        PyAspectRatioConstraint {
            p1,
            p2,
            p3,
            p4,
            ratio,
        }
    }
}

impl PyAspectRatioConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.p1, self.p2, self.p3, self.p4, self.ratio)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAspectRatioConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.aspect_ratio", m)?;
    Ok(())
}
