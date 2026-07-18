//! PyO3 binding for [`MergeLinesSpec`](crate::ops::transform::merge_lines::MergeLinesSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::merge_lines::MergeLinesSpec as CoreMergeLinesSpec;

/// Register the `MergeLinesSpec` class on the `merge_lines` submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let merge_lines_mod = PyModule::new(transform_mod.py(), "merge_lines")?;
    merge_lines_mod.add_class::<MergeLinesSpec>()?;
    transform_mod.add_submodule(&merge_lines_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules
        .set_item("raygeo.ops.transform.merge_lines", &merge_lines_mod)?;

    Ok(())
}

/// Parameters for the ``MergeLines`` transformer.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.merge_lines",
    name = "MergeLinesSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct MergeLinesSpec {
    /// Maximum distance for considering lines collinear.
    #[pyo3(get)]
    pub tolerance: f64,
}

impl MergeLinesSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreMergeLinesSpec {
        CoreMergeLinesSpec {
            tolerance: self.tolerance,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl MergeLinesSpec {
    #[new]
    fn new(tolerance: f64) -> Self {
        Self { tolerance }
    }
}
