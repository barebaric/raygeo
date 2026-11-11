//! Bindings for the symmetry constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

use crate::sketch::constraints::symmetry::{
    SymmetryConstraint as CoreConstraint, SymmetryTarget,
};

/// The symmetry reference: point symmetry about a center, or line
/// symmetry about an axis line.
#[gen_stub_pyclass_complex_enum]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.symmetry",
    name = "SymmetryTarget"
)]
#[derive(Clone, PartialEq)]
pub enum PySymmetryTarget {
    /// The center is the midpoint of p1 and p2.
    Point { center: u64 },
    /// p1 and p2 are mirrored across the line (l1, l2).
    Line { l1: u64, l2: u64 },
}

impl From<&PySymmetryTarget> for SymmetryTarget {
    fn from(value: &PySymmetryTarget) -> Self {
        match value {
            PySymmetryTarget::Point { center } => {
                SymmetryTarget::Point { center: *center }
            }
            PySymmetryTarget::Line { l1, l2 } => {
                SymmetryTarget::Line { l1: *l1, l2: *l2 }
            }
        }
    }
}

/// Enforces two points are symmetric about a point or a line.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.symmetry",
    name = "SymmetryConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PySymmetryConstraint {
    /// First point.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point.
    #[pyo3(get)]
    pub p2: u64,
    /// Symmetry reference.
    #[pyo3(get)]
    pub target: PySymmetryTarget,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySymmetryConstraint {
    /// Creates the symmetry constraint.
    #[new]
    #[pyo3(signature = (p1, p2, target))]
    fn new(p1: u64, p2: u64, target: PySymmetryTarget) -> Self {
        PySymmetryConstraint { p1, p2, target }
    }
}

impl PySymmetryConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(
            self.p1,
            self.p2,
            SymmetryTarget::from(&self.target),
        )
    }
}

/// Registers the constraint classes with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySymmetryTarget>()?;
    m.add_class::<PySymmetryConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.symmetry", m)?;
    Ok(())
}
