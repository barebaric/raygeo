//! PyO3 binding for [`SmoothSpec`](crate::ops::transform::smooth::SmoothSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::smooth::SmoothSpec as CoreSmoothSpec;

/// Register the `SmoothSpec` class on the `smooth` submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let smooth_mod = PyModule::new(transform_mod.py(), "smooth")?;
    smooth_mod.add_class::<SmoothSpec>()?;
    transform_mod.add_submodule(&smooth_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform.smooth", &smooth_mod)?;

    Ok(())
}

/// Parameters for the ``Smooth`` transformer.
///
/// Construct with ``SmoothSpec(amount, corner_angle_threshold)``.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.smooth",
    name = "SmoothSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct SmoothSpec {
    /// Smoothing strength (0-100); 0 is a no-op.
    #[pyo3(get)]
    pub amount: u32,
    /// Corners with an internal angle (degrees) smaller than this are
    /// preserved.
    #[pyo3(get)]
    pub corner_angle_threshold: f64,
}

impl SmoothSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreSmoothSpec {
        CoreSmoothSpec {
            amount: self.amount,
            corner_angle_threshold: self.corner_angle_threshold,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl SmoothSpec {
    #[new]
    fn new(amount: u32, corner_angle_threshold: f64) -> Self {
        Self {
            amount,
            corner_angle_threshold,
        }
    }
}
