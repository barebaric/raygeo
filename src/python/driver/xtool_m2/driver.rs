//! The `raygeo.driver.xtool_m2.session` submodule: the `XtoolM2Driver`
//! pyclass mirroring the Rust `M2Core` surface (config parsing,
//! asyncio futures, event marshaling).

use std::sync::Arc;

use crate::python::driver::base::PyDriver;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::core::driver::Driver;
use crate::driver::xtool_m2::driver::{M2Config, XtoolM2Driver};
use crate::python::driver::events::PyEvents;
use crate::python::driver::grbl::driver::{
    get_bool, get_f64, get_i64, get_str,
};
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

fn build_config(dict: Option<&Bound<'_, PyDict>>) -> PyResult<M2Config> {
    let host = dict
        .and_then(|dict| get_str(dict, "host").ok().flatten())
        .unwrap_or_default();
    let mut config = M2Config::new(&host);
    let Some(dict) = dict else {
        return Ok(config);
    };
    if let Some(value) = get_i64(dict, "port")? {
        config.port = value.clamp(0, u16::MAX as i64) as u16;
    }
    if let Some(value) = get_f64(dict, "request_timeout")? {
        config.request_timeout = value;
    }
    if let Some(value) = get_f64(dict, "connect_timeout")? {
        config.connect_timeout = value;
    }
    if let Some(value) = get_f64(dict, "reconnect_delay")? {
        config.reconnect_delay = value;
    }
    if let Some(value) = get_f64(dict, "poll_interval")? {
        config.poll_interval = value;
    }
    if let Some(value) = get_bool(dict, "tls_allow_invalid_certs")? {
        config.tls_allow_invalid_certs = value;
    }
    Ok(config)
}

#[gen_stub_pyclass]
/// Monitor and control an xTool M2 over its WS-V2 TLS service.
/// This session does not upload G-code jobs. `start` controls a job
/// that is already staged on the device, such as a job sent by xTool
/// Studio.
#[pyclass(name = "XtoolM2Driver", module = "raygeo.driver.xtool_m2", extends = PyDriver)]
pub struct PyXtoolM2Driver {
    core: Arc<XtoolM2Driver>,
}

impl PyXtoolM2Driver {
    fn initializer(
        config: M2Config,
        event_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let events = Arc::new(PyEvents::new(event_callback));
        let core = XtoolM2Driver::new(config, events.clone());
        let base =
            crate::python::driver::base::PyDriver::new(core.clone(), events);
        Ok(PyClassInitializer::from(base).add_subclass(Self { core }))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyXtoolM2Driver {
    #[new]
    #[gen_stub(override_return_type(type_repr = "XtoolM2Driver"))]
    #[pyo3(signature = (config, dialect=None, event_callback=None))]
    /// Create an M2 session for the configured host.
    fn new(
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Union[dict, config.M2Config]]",
            imports = ("typing", "raygeo.driver.config")
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
        Self::initializer(build_config(config.as_ref())?, event_callback)
    }
    /// Start a job already staged on the device.
    fn start<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.start().await?;
            Ok(())
        })
    }
    #[getter]
    /// Whether the state currently indicates an active or held job.
    fn is_job_running(&self) -> bool {
        self.core.is_job_running()
    }

    fn __repr__(&self) -> String {
        format!(
            "XtoolM2Driver(resource_uri={:?}, job_running={})",
            self.core.resource_uri(),
            self.core.is_job_running(),
        )
    }
}
