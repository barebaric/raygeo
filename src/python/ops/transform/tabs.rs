//! PyO3 binding for [`TabsSpec`](crate::ops::transform::tabs::TabsSpec).

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::transform::tabs::{ClipPoint, TabsSpec as CoreTabsSpec};

/// Register the `TabsSpec` class on the `tabs` submodule.
pub(crate) fn register(transform_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let tabs_mod = PyModule::new(transform_mod.py(), "tabs")?;
    tabs_mod.add_class::<TabsSpec>()?;
    transform_mod.add_submodule(&tabs_mod)?;

    let sys_modules = transform_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform.tabs", &tabs_mod)?;

    Ok(())
}

/// Parameters for the ``Tabs`` transformer.
///
/// The ``clips`` are pre-scaled clip points in ops space, computed by
/// Python from the workpiece's tab definitions.
///
/// Dispatch rule: if ``tab_power > 0.0`` the power mode is used (with
/// effective power ``tab_power * original_power``); otherwise the gap
/// mode is used.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.transform.tabs",
    name = "TabsSpec",
    frozen,
    eq,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct TabsSpec {
    /// Tab power level (0.0-1.0). 0.0 selects gap mode.
    #[pyo3(get)]
    pub tab_power: f64,
    /// Normal cutting power restored after each tab.
    #[pyo3(get)]
    pub original_power: f64,
    /// ``(x, y, width)`` tuples defining tab positions.
    #[pyo3(get)]
    pub clips: Vec<(f64, f64, f64)>,
}

impl TabsSpec {
    /// Convert into the core-layer spec.
    pub fn into_core(self) -> CoreTabsSpec {
        let clips: Vec<ClipPoint> = self
            .clips
            .into_iter()
            .map(|(x, y, width)| ClipPoint { x, y, width })
            .collect();
        CoreTabsSpec {
            tab_power: self.tab_power,
            original_power: self.original_power,
            clips,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl TabsSpec {
    #[new]
    fn new(
        tab_power: f64,
        original_power: f64,
        clips: Vec<(f64, f64, f64)>,
    ) -> Self {
        Self {
            tab_power,
            original_power,
            clips,
        }
    }
}
