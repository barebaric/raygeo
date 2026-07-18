//! PyO3 binding for [`CropSpec`](crate::ops::transform::clip::CropSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::clip::CropSpec as CoreCropSpec;

/// Register the `CropSpec` class on the `clip` submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let clip_mod = PyModule::new(transform_mod.py(), "clip")?;
    clip_mod.add_class::<CropSpec>()?;
    transform_mod.add_submodule(&clip_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform.clip", &clip_mod)?;

    Ok(())
}

/// Parameters for the ``Crop`` transformer.
///
/// The ``regions`` are pre-resolved clip regions in ops-local space,
/// computed by Python from stock geometries + workpiece transform.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.clip",
    name = "CropSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct CropSpec {
    /// Approximation tolerance for primitive refitting.
    #[pyo3(get)]
    pub tolerance: f64,
    /// Offset (mm) that was applied when growing the stock geometry;
    /// kept for traceability but not used by the dispatch.
    #[pyo3(get)]
    pub offset: f64,
    /// Pre-resolved clip regions, each a polygon of ``(x, y)`` vertices.
    #[pyo3(get)]
    pub regions: Vec<Vec<(f64, f64)>>,
}

impl CropSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreCropSpec {
        CoreCropSpec {
            tolerance: self.tolerance,
            offset: self.offset,
            regions: crate::python::geo::flex_point::polygons_from_tuples(
                self.regions,
            ),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl CropSpec {
    #[new]
    fn new(tolerance: f64, offset: f64, regions: Vec<Vec<(f64, f64)>>) -> Self {
        Self {
            tolerance,
            offset,
            regions,
        }
    }
}
