//! The `raydriver.simulator` module: a transport-free machine for UI
//! previews and tests.

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::simulator::{NoDeviceConfig, NoDeviceDriver};
use crate::python::driver::base::PyDriver;
use crate::python::driver::events::PyEvents;
use crate::python::driver::grbl::driver::{get_f64, get_str};

const MODULE_DOC: &str = "\
A simulated machine with no physical device behind it.

Runs jobs with a configurable per-op delay, accepts moves and
settings, mirrors work-coordinate offsets and returns a fixed probe
point. Intended for UI previews and tests.
";

pyo3_stub_gen::module_doc!("raygeo.driver.simulator", "{}", MODULE_DOC);

fn build_config(dict: Option<&Bound<'_, PyDict>>) -> PyResult<NoDeviceConfig> {
    let mut config = NoDeviceConfig::default();
    let Some(dict) = dict else {
        return Ok(config);
    };
    if let Some(delay) = get_f64(dict, "step_delay")? {
        config.step_delay = delay.max(0.0);
    }
    if let Some(wcs) = get_str(dict, "active_wcs")? {
        config.active_wcs = wcs;
    }
    if let Some(item) = dict.get_item("coordinate_systems")? {
        let systems = item.cast_into::<PyDict>()?;
        for (slot, offsets) in systems.iter() {
            let slot: String = slot.extract()?;
            let values: Vec<f64> = offsets.extract()?;
            let get = |index: usize| values.get(index).copied().unwrap_or(0.0);
            config
                .coordinate_systems
                .insert(slot, (get(0), get(1), get(2)));
        }
    }
    Ok(config)
}

/// A simulated machine: no connection, no transport.
#[gen_stub_pyclass]
#[pyclass(name = "NoDeviceDriver", module = "raygeo.driver.simulator", extends = PyDriver)]
pub struct PyNoDeviceDriver;

#[gen_stub_pymethods]
#[pymethods]
impl PyNoDeviceDriver {
    #[new]
    #[gen_stub(override_return_type(type_repr = "NoDeviceDriver"))]
    #[pyo3(signature = (config=None, dialect=None, event_callback=None))]
    fn new(
        #[gen_stub(override_type(
            type_repr = "typing.Optional[dict]",
            imports = ("typing",)
        ))]
        config: Option<Bound<'_, PyAny>>,

        dialect: Option<Bound<'_, PyDict>>,

        #[gen_stub(override_type(
            type_repr = "typing.Optional[collections.abc.Callable[[core.DriverEvent], builtins.object]]",
            imports = ("builtins", "collections.abc", "typing", "raygeo.driver.core")
        ))]
        event_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let _ = dialect;
        let config =
            crate::python::driver::config::config_dict(config.as_ref())?;
        let config = build_config(config.as_ref())?;
        let events = Arc::new(PyEvents::new(event_callback));
        let core = NoDeviceDriver::new(config, events.clone());
        let base = PyDriver::new(core.clone(), events);
        Ok(PyClassInitializer::from(base).add_subclass(Self))
    }

    fn __repr__(&self) -> String {
        "NoDeviceDriver()".to_string()
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let module = PyModule::new(py, "simulator")?;
    module.setattr("__doc__", MODULE_DOC)?;
    module.add("__all__", vec!["NoDeviceDriver"])?;
    module.add_class::<PyNoDeviceDriver>()?;
    crate::python::driver::core::discovery::helpers::attach_submodule(
        m,
        &module,
        "raygeo.driver.simulator",
    )?;
    Ok(())
}
