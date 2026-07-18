//! PyO3 binding for [`BidirScanOffsetSpec`](crate::ops::transform::bidir_scan_offset::BidirScanOffsetSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::bidir_scan_offset::BidirScanOffsetSpec as CoreBidirScanOffsetSpec;

/// Register the `BidirScanOffsetSpec` class on the `bidir_scan_offset`
/// submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let bidir_mod = PyModule::new(transform_mod.py(), "bidir_scan_offset")?;
    bidir_mod.add_class::<BidirScanOffsetSpec>()?;
    transform_mod.add_submodule(&bidir_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules
        .set_item("raygeo.ops.transform.bidir_scan_offset", &bidir_mod)?;

    Ok(())
}

/// Parameters for the ``BidirScanOffset`` transformer.
///
/// Construct with ``BidirScanOffsetSpec(offset_mm)``. An offset of 0.0
/// is a legitimate no-op spec.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.bidir_scan_offset",
    name = "BidirScanOffsetSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct BidirScanOffsetSpec {
    /// X offset in millimeters applied to right-to-left raster passes.
    #[pyo3(get)]
    pub offset_mm: f64,
}

impl BidirScanOffsetSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreBidirScanOffsetSpec {
        CoreBidirScanOffsetSpec {
            offset_mm: self.offset_mm,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl BidirScanOffsetSpec {
    #[new]
    fn new(offset_mm: f64) -> Self {
        Self { offset_mm }
    }
}
