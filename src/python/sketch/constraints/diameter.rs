//! Bindings for the diameter constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::diameter::DiameterConstraint as CoreConstraint;

/// Enforces the diameter of a circle.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.diameter",
    name = "DiameterConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyDiameterConstraint {
    /// Center point of the circle.
    #[pyo3(get)]
    pub center: u64,
    /// Radius-bearing point of the circle.
    #[pyo3(get)]
    pub radius_point: u64,
    /// Resolved diameter value.
    #[pyo3(get)]
    pub value: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDiameterConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (center, radius_point, value))]
    fn new(center: u64, radius_point: u64, value: f64) -> Self {
        PyDiameterConstraint {
            center,
            radius_point,
            value,
        }
    }
}

impl PyDiameterConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.center, self.radius_point, self.value)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDiameterConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.diameter", m)?;
    Ok(())
}
