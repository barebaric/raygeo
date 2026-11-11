//! Bindings for the perpendicular constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

use crate::sketch::constraints::perpendicular::{
    PerpendicularConstraint as CoreConstraint, PerpendicularTarget,
};

/// The resolved geometry of the two perpendicular operands: two
/// lines, a line through a circle/arc center, or two circles meeting
/// at a right angle.
#[gen_stub_pyclass_complex_enum]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.perpendicular",
    name = "PerpendicularTarget"
)]
#[derive(Clone, PartialEq)]
pub enum PyPerpendicularTarget {
    /// Line-line: direction dot product.
    LineLine { a1: u64, a2: u64, b1: u64, b2: u64 },
    /// Line and circle/arc: the line passes through the center.
    LineCenter { l1: u64, l2: u64, center: u64 },
    /// Circle/arc pair: r1² + r2² == dist(c1, c2)².
    Circles {
        c1: u64,
        r1_point: u64,
        c2: u64,
        r2_point: u64,
    },
}

impl From<&PyPerpendicularTarget> for PerpendicularTarget {
    fn from(value: &PyPerpendicularTarget) -> Self {
        match value {
            PyPerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                PerpendicularTarget::LineLine {
                    a1: *a1,
                    a2: *a2,
                    b1: *b1,
                    b2: *b2,
                }
            }
            PyPerpendicularTarget::LineCenter { l1, l2, center } => {
                PerpendicularTarget::LineCenter {
                    l1: *l1,
                    l2: *l2,
                    center: *center,
                }
            }
            PyPerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => PerpendicularTarget::Circles {
                c1: *c1,
                r1_point: *r1_point,
                c2: *c2,
                r2_point: *r2_point,
            },
        }
    }
}

/// Enforces perpendicularity between two entities.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.perpendicular",
    name = "PerpendicularConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyPerpendicularConstraint {
    /// Resolved operand geometry.
    #[pyo3(get)]
    pub target: PyPerpendicularTarget,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPerpendicularConstraint {
    /// Creates the constraint from resolved operands.
    #[new]
    #[pyo3(signature = (target))]
    fn new(target: PyPerpendicularTarget) -> Self {
        PyPerpendicularConstraint { target }
    }
}

impl PyPerpendicularConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(PerpendicularTarget::from(&self.target))
    }
}

/// Registers the constraint classes with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPerpendicularTarget>()?;
    m.add_class::<PyPerpendicularConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.perpendicular", m)?;
    Ok(())
}
