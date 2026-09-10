//! PyO3 binding for [`DragKnifeSpec`](crate::ops::transform::drag_knife::DragKnifeSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::drag_knife::DragKnifeSpec as CoreDragKnifeSpec;

/// Register the `DragKnifeSpec` class on the `drag_knife` submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let drag_knife_mod = PyModule::new(transform_mod.py(), "drag_knife")?;
    drag_knife_mod.add_class::<DragKnifeSpec>()?;
    transform_mod.add_submodule(&drag_knife_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform.drag_knife", &drag_knife_mod)?;

    Ok(())
}

/// Parameters for the ``DragKnife`` transformer.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.drag_knife",
    name = "DragKnifeSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct DragKnifeSpec {
    /// Distance between blade tip and pivot in millimeters.
    #[pyo3(get)]
    pub offset_mm: f64,
    /// Maximum direction change (degrees) swiveled with the blade down.
    #[pyo3(get)]
    pub swivel_angle_deg: f64,
}

impl DragKnifeSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreDragKnifeSpec {
        CoreDragKnifeSpec {
            offset_mm: self.offset_mm,
            swivel_angle_deg: self.swivel_angle_deg,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl DragKnifeSpec {
    #[new]
    fn new(offset_mm: f64, swivel_angle_deg: f64) -> Self {
        Self {
            offset_mm,
            swivel_angle_deg,
        }
    }
}
