//! Bindings for the tangent constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::tangent::TangentConstraint as CoreConstraint;

/// Enforces a line is tangent to a circle/arc.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.tangent",
    name = "TangentConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyTangentConstraint {
    /// First point of the line.
    #[pyo3(get)]
    pub l1: u64,
    /// Second point of the line.
    #[pyo3(get)]
    pub l2: u64,
    /// Center point of the circle/arc.
    #[pyo3(get)]
    pub center: u64,
    /// Radius-bearing point (circle radius point or arc start).
    #[pyo3(get)]
    pub radius_point: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTangentConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (l1, l2, center, radius_point))]
    fn new(l1: u64, l2: u64, center: u64, radius_point: u64) -> Self {
        PyTangentConstraint {
            l1,
            l2,
            center,
            radius_point,
        }
    }
}

impl PyTangentConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.l1, self.l2, self.center, self.radius_point)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTangentConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.tangent", m)?;
    Ok(())
}
