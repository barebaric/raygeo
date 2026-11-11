//! Bindings for the point-on-line constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

use crate::sketch::constraints::point_on_line::{
    PointOnLineConstraint as CoreConstraint, PointOnTarget,
};

/// The resolved target geometry: an infinite line, or a circle/arc
/// circumference.
#[gen_stub_pyclass_complex_enum]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.point_on_line",
    name = "PointOnTarget"
)]
#[derive(Clone, PartialEq)]
pub enum PyPointOnTarget {
    /// The infinite line through two points.
    Line { l1: u64, l2: u64 },
    /// The circle/arc through center and radius-bearing point.
    RadiusCurve { center: u64, radius_point: u64 },
}

impl From<&PyPointOnTarget> for PointOnTarget {
    fn from(value: &PyPointOnTarget) -> Self {
        match value {
            PyPointOnTarget::Line { l1, l2 } => {
                PointOnTarget::Line { l1: *l1, l2: *l2 }
            }
            PyPointOnTarget::RadiusCurve {
                center,
                radius_point,
            } => PointOnTarget::RadiusCurve {
                center: *center,
                radius_point: *radius_point,
            },
        }
    }
}

/// Enforces a point lies on a line or circle/arc circumference.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.point_on_line",
    name = "PointOnLineConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyPointOnLineConstraint {
    /// The constrained point.
    #[pyo3(get)]
    pub point: u64,
    /// Resolved target geometry.
    #[pyo3(get)]
    pub target: PyPointOnTarget,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPointOnLineConstraint {
    /// Creates the point-on-line/curve constraint.
    #[new]
    #[pyo3(signature = (point, target))]
    fn new(point: u64, target: PyPointOnTarget) -> Self {
        PyPointOnLineConstraint { point, target }
    }
}

impl PyPointOnLineConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.point, PointOnTarget::from(&self.target))
    }
}

/// Registers the constraint classes with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPointOnTarget>()?;
    m.add_class::<PyPointOnLineConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.point_on_line", m)?;
    Ok(())
}
