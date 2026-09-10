//! PyO3 binding for
//! [`TangentialKnifeSpec`](crate::ops::transform::tangential_knife::TangentialKnifeSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::tangential_knife::TangentialKnifeSpec as CoreTangentialKnifeSpec;

/// Register the `TangentialKnifeSpec` class on the `tangential_knife`
/// submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let tangential_knife_mod =
        PyModule::new(transform_mod.py(), "tangential_knife")?;
    tangential_knife_mod.add_class::<TangentialKnifeSpec>()?;
    transform_mod.add_submodule(&tangential_knife_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item(
        "raygeo.ops.transform.tangential_knife",
        &tangential_knife_mod,
    )?;

    Ok(())
}

/// Parameters for the ``TangentialKnife`` transformer.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.tangential_knife",
    name = "TangentialKnifeSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct TangentialKnifeSpec {
    /// Maximum heading change (degrees) rotated with the knife down.
    #[pyo3(get)]
    pub angle_tolerance_deg: f64,
    /// Arcs tighter than this radius (millimeters) force a lift.
    #[pyo3(get)]
    pub radius_tolerance_mm: f64,
    /// Z height used for lifting the knife at sharp corners.
    #[pyo3(get)]
    pub safe_z: f64,
}

impl TangentialKnifeSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreTangentialKnifeSpec {
        CoreTangentialKnifeSpec {
            angle_tolerance_deg: self.angle_tolerance_deg,
            radius_tolerance_mm: self.radius_tolerance_mm,
            safe_z: self.safe_z,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl TangentialKnifeSpec {
    #[new]
    fn new(
        angle_tolerance_deg: f64,
        radius_tolerance_mm: f64,
        safe_z: f64,
    ) -> Self {
        Self {
            angle_tolerance_deg,
            radius_tolerance_mm,
            safe_z,
        }
    }
}
