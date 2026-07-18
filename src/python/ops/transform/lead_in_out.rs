//! PyO3 binding for [`LeadInOutSpec`](crate::ops::transform::lead_in_out::LeadInOutSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::lead_in_out::LeadInOutSpec as CoreLeadInOutSpec;

/// Register the `LeadInOutSpec` class on the `lead_in_out` submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let lead_in_out_mod = PyModule::new(transform_mod.py(), "lead_in_out")?;
    lead_in_out_mod.add_class::<LeadInOutSpec>()?;
    transform_mod.add_submodule(&lead_in_out_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules
        .set_item("raygeo.ops.transform.lead_in_out", &lead_in_out_mod)?;

    Ok(())
}

/// Parameters for the ``LeadInOut`` transformer.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.lead_in_out",
    name = "LeadInOutSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct LeadInOutSpec {
    /// Lead-in distance in millimeters.
    #[pyo3(get)]
    pub lead_in_mm: f64,
    /// Lead-out distance in millimeters.
    #[pyo3(get)]
    pub lead_out_mm: f64,
}

impl LeadInOutSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreLeadInOutSpec {
        CoreLeadInOutSpec {
            lead_in_mm: self.lead_in_mm,
            lead_out_mm: self.lead_out_mm,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl LeadInOutSpec {
    #[new]
    fn new(lead_in_mm: f64, lead_out_mm: f64) -> Self {
        Self {
            lead_in_mm,
            lead_out_mm,
        }
    }
}
