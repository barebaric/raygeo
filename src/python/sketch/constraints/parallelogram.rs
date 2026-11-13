//! Bindings for the parallelogram constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::sketch::constraints::parallelogram::ParallelogramConstraint as CoreConstraint;

/// Enforces (p_width - p_origin) == (p4 - p_height).
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.parallelogram",
    name = "ParallelogramConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyParallelogramConstraint {
    /// Origin point.
    #[pyo3(get)]
    pub p_origin: u64,
    /// Width point.
    #[pyo3(get)]
    pub p_width: u64,
    /// Height point.
    #[pyo3(get)]
    pub p_height: u64,
    /// Fourth corner.
    #[pyo3(get)]
    pub p4: u64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyParallelogramConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p_origin, p_width, p_height, p4))]
    fn new(p_origin: u64, p_width: u64, p_height: u64, p4: u64) -> Self {
        PyParallelogramConstraint {
            p_origin,
            p_width,
            p_height,
            p4,
        }
    }
}

impl PyParallelogramConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.p_origin, self.p_width, self.p_height, self.p4)
    }

    pub(crate) fn from_core(core: CoreConstraint) -> Self {
        PyParallelogramConstraint {
            p_origin: core.p_origin,
            p_width: core.p_width,
            p_height: core.p_height,
            p4: core.p4,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyParallelogramConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.parallelogram", m)?;
    Ok(())
}
