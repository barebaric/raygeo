//! Bindings for the equal-length constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::equal_length::EqualLengthConstraint as CoreConstraint;

/// Enforces equal characteristic lengths, one residual per
/// `(base_a, base_b, other_a, other_b)` tuple.  The caller expands
/// the entities' characteristic-length pairs (including the
/// shorter-side fallback) into flat tuples.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.equal_length",
    name = "EqualLengthConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyEqualLengthConstraint {
    /// Resolved characteristic-length pairs.
    #[pyo3(get)]
    pub pairs: Vec<[u64; 4]>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEqualLengthConstraint {
    /// Creates the constraint from resolved pairs.
    #[new]
    #[pyo3(signature = (pairs))]
    fn new(pairs: Vec<[u64; 4]>) -> Self {
        PyEqualLengthConstraint { pairs }
    }
}

impl PyEqualLengthConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.pairs)
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEqualLengthConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.equal_length", m)?;
    Ok(())
}
