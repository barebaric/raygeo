//! Bindings for the rotational constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::rotational::RotationalConstraint as CoreConstraint;

/// Enforces p2 is p1 rotated by angle_rad about center.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.rotational",
    name = "RotationalConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyRotationalConstraint {
    /// Rotation center.
    #[pyo3(get)]
    pub center: u64,
    /// Source point.
    #[pyo3(get)]
    pub p1: u64,
    /// Rotated point.
    #[pyo3(get)]
    pub p2: u64,
    /// Resolved rotation angle in radians.
    #[pyo3(get)]
    pub angle_rad: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyRotationalConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (center, p1, p2, angle_rad))]
    fn new(center: u64, p1: u64, p2: u64, angle_rad: f64) -> Self {
        PyRotationalConstraint {
            center,
            p1,
            p2,
            angle_rad,
        }
    }
}

impl PyRotationalConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.center, self.p1, self.p2, self.angle_rad)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRotationalConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.rotational", m)?;
    Ok(())
}
