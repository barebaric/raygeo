//! PyO3 binding for
//! [`MergeScanlinesSpec`](crate::ops::transform::merge_scanlines::MergeScanlinesSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::merge_scanlines::MergeScanlinesSpec as CoreMergeScanlinesSpec;

/// Register the `MergeScanlinesSpec` class on the
/// `merge_scanlines` submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let merge_scanlines_mod =
        PyModule::new(transform_mod.py(), "merge_scanlines")?;
    merge_scanlines_mod.add_class::<MergeScanlinesSpec>()?;
    transform_mod.add_submodule(&merge_scanlines_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item(
        "raygeo.ops.transform.merge_scanlines",
        &merge_scanlines_mod,
    )?;

    Ok(())
}

/// Parameters for the ``MergeScanlines`` transformer.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.merge_scanlines",
    name = "MergeScanlinesSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct MergeScanlinesSpec {
    /// Machine acceleration in mm/s².
    #[pyo3(get)]
    pub acceleration: f64,
    /// Fallback cut speed in mm/min.
    #[pyo3(get)]
    pub cut_speed: f64,
    /// Fallback rapid speed in mm/min.
    #[pyo3(get)]
    pub rapid_speed: f64,
    /// Manual ceiling on bridged gap length in mm (0 = unlimited).
    #[pyo3(get)]
    pub max_gap_mm: f64,
    /// Maximum perpendicular distance for two parallel lines to be
    /// considered part of the same row, in mm.
    #[pyo3(get)]
    pub tolerance: f64,
}

impl MergeScanlinesSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreMergeScanlinesSpec {
        CoreMergeScanlinesSpec {
            acceleration: self.acceleration,
            cut_speed: self.cut_speed,
            rapid_speed: self.rapid_speed,
            max_gap_mm: self.max_gap_mm,
            tolerance: self.tolerance,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl MergeScanlinesSpec {
    #[new]
    fn new(
        acceleration: f64,
        cut_speed: f64,
        rapid_speed: f64,
        max_gap_mm: f64,
        tolerance: f64,
    ) -> Self {
        Self {
            acceleration,
            cut_speed,
            rapid_speed,
            max_gap_mm,
            tolerance,
        }
    }
}
