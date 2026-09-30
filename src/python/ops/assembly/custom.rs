use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::assembly::custom::CustomSpec as CoreCustomSpec;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "custom")?;
    m.add_class::<PyCustomSpec>()?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.custom", &m)?;

    Ok(())
}

/// Parameters for the ``custom`` assembler.
///
/// Construct with ``CustomSpec(lines)``. Wrap in an
/// :class:`~raygeo.ops.assembly.Assembler` instance to drive the
/// `Assembler` trait.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.assembly.custom",
    name = "CustomSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyCustomSpec {
    /// Raw (unexpanded) machine-code lines, one command each.
    #[pyo3(get)]
    pub lines: Vec<String>,
}

impl PyCustomSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreCustomSpec {
        CoreCustomSpec { lines: self.lines }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCustomSpec {
    #[new]
    #[pyo3(signature = (lines))]
    fn new(lines: Vec<String>) -> Self {
        PyCustomSpec { lines }
    }

    fn __repr__(&self) -> String {
        format!("CustomSpec(lines={:?})", self.lines)
    }
}
