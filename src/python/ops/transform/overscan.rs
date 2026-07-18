//! PyO3 binding for [`OverscanSpec`](crate::ops::transform::overscan::OverscanSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::overscan::OverscanSpec as CoreOverscanSpec;

/// Register the `OverscanSpec` class on the `overscan` submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let overscan_mod = PyModule::new(transform_mod.py(), "overscan")?;
    overscan_mod.add_class::<OverscanSpec>()?;
    transform_mod.add_submodule(&overscan_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform.overscan", &overscan_mod)?;

    Ok(())
}

/// Parameters for the ``Overscan`` transformer.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.overscan",
    name = "OverscanSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct OverscanSpec {
    /// Overscan distance in millimeters.
    #[pyo3(get)]
    pub distance_mm: f64,
}

impl OverscanSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreOverscanSpec {
        CoreOverscanSpec {
            distance_mm: self.distance_mm,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl OverscanSpec {
    #[new]
    fn new(distance_mm: f64) -> Self {
        Self { distance_mm }
    }
}
