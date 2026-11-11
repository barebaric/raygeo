//! Bindings for the angle constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::angle::AngleConstraint as CoreConstraint;

/// Enforces a clockwise angle in degrees between two lines.  The
/// angle is measured at the lines' intersection between the rays
/// selected by `e1_far`/`e2_far` (endpoint ids; `None` picks the
/// endpoint farther from the intersection).
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.angle",
    name = "AngleConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyAngleConstraint {
    /// First point of the anchor line.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point of the anchor line.
    #[pyo3(get)]
    pub p2: u64,
    /// First point of the other line.
    #[pyo3(get)]
    pub p3: u64,
    /// Second point of the other line.
    #[pyo3(get)]
    pub p4: u64,
    /// Far point of the anchor line.
    #[pyo3(get)]
    pub e1_far: Option<u64>,
    /// Far point of the other line.
    #[pyo3(get)]
    pub e2_far: Option<u64>,
    /// Resolved angle value in degrees.
    #[pyo3(get)]
    pub value_deg: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAngleConstraint {
    /// Creates the angle constraint from its resolved points and
    /// value.
    #[new]
    #[pyo3(signature = (p1, p2, p3, p4, value_deg, e1_far = None, e2_far = None))]
    fn new(
        p1: u64,
        p2: u64,
        p3: u64,
        p4: u64,
        value_deg: f64,
        e1_far: Option<u64>,
        e2_far: Option<u64>,
    ) -> Self {
        PyAngleConstraint {
            p1,
            p2,
            p3,
            p4,
            e1_far,
            e2_far,
            value_deg,
        }
    }
}

impl PyAngleConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(
            self.p1,
            self.p2,
            self.p3,
            self.p4,
            self.e1_far,
            self.e2_far,
            self.value_deg,
        )
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAngleConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.angle", m)?;
    Ok(())
}
