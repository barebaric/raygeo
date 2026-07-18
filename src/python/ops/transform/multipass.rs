//! PyO3 binding for [`MultiPassSpec`](crate::ops::transform::multipass::MultiPassSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::multipass::MultiPassSpec as CoreMultiPassSpec;

/// Register the `MultiPassSpec` class on the `multipass` submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let multipass_mod = PyModule::new(transform_mod.py(), "multipass")?;
    multipass_mod.add_class::<MultiPassSpec>()?;
    transform_mod.add_submodule(&multipass_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform.multipass", &multipass_mod)?;

    Ok(())
}

/// Parameters for the ``MultiPass`` transformer.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.multipass",
    name = "MultiPassSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct MultiPassSpec {
    /// Total number of passes (must be >= 1).
    #[pyo3(get)]
    pub passes: u32,
    /// Z distance to move down after each pass.
    #[pyo3(get)]
    pub z_step_down: f64,
}

impl MultiPassSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreMultiPassSpec {
        CoreMultiPassSpec {
            passes: self.passes,
            z_step_down: self.z_step_down,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl MultiPassSpec {
    #[new]
    fn new(passes: u32, z_step_down: f64) -> Self {
        Self {
            passes,
            z_step_down,
        }
    }
}
