//! The `OctoPrintDriver` PyO3 wrapper: the server's REST API and
//! SockJS WebSocket, exposed through the uniform session surface.

use std::sync::Arc;

use crate::python::driver::base::PyDriver;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::core::driver::Driver;
use crate::driver::octoprint::driver::{OctoPrintConfig, OctoPrintDriver};
use crate::python::driver::core::json::{value_to_dict, JsonDict};
use crate::python::driver::events::PyEvents;
use crate::python::driver::grbl::driver::{get_f64, get_i64, get_str};
use crate::python::driver::octoprint::probe::OctoPrintDeviceInfo;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

fn build_config(dict: Option<&Bound<'_, PyDict>>) -> PyResult<OctoPrintConfig> {
    let host = dict
        .and_then(|dict| get_str(dict, "host").ok().flatten())
        .unwrap_or_default();
    let mut config = OctoPrintConfig::new(&host);
    let Some(dict) = dict else {
        return Ok(config);
    };
    if let Some(v) = get_i64(dict, "port")? {
        config.port = v.max(0) as u16;
    }
    if let Some(v) = get_str(dict, "path").ok().flatten() {
        config.path_prefix =
            crate::driver::octoprint::driver::normalize_path_prefix(&v);
    }
    if let Some(v) = get_str(dict, "api_key").ok().flatten() {
        config.api_key = v;
    }
    if let Some(v) = get_f64(dict, "request_timeout")? {
        config.request_timeout = v;
    }
    if let Some(v) = get_f64(dict, "connect_timeout")? {
        config.connect_timeout = v;
    }
    if let Some(v) = get_f64(dict, "poll_interval")? {
        config.poll_interval = v;
    }
    if let Some(v) = get_f64(dict, "reconnect_delay")? {
        config.reconnect_delay = v;
    }
    if let Some(v) = get_f64(dict, "ws_ping_interval")? {
        config.ws_ping_interval = v;
    }
    Ok(config)
}

/// An OctoPrint server session: job submission and live printer
/// state over the REST API and SockJS WebSocket.
///
/// All async methods return asyncio futures; a typed `DriverEvent`
/// is delivered to `event_callback(event)` on the event loop thread.
/// Operations the OctoPrint API cannot perform (e.g. writing
/// firmware settings) raise `NotImplementedError`.
#[gen_stub_pyclass(module = "raygeo.driver.octoprint")]
#[pyclass(name = "OctoPrintDriver", module = "raygeo.driver.octoprint", extends = PyDriver)]
pub struct PyOctoPrintDriver {
    core: Arc<OctoPrintDriver>,
}

impl PyOctoPrintDriver {
    fn initializer(
        config: OctoPrintConfig,
        event_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let events = Arc::new(PyEvents::new(event_callback));
        let core = OctoPrintDriver::new(config, events.clone());
        let base =
            crate::python::driver::base::PyDriver::new(core.clone(), events);
        Ok(PyClassInitializer::from(base).add_subclass(Self { core }))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyOctoPrintDriver {
    #[new]
    #[gen_stub(override_return_type(type_repr = "OctoPrintDriver"))]
    #[pyo3(signature = (config, dialect=None, event_callback=None))]
    fn new(
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Union[dict, config.OctoPrintConfig]]",
            imports = ("typing", "raygeo.driver.config")
        ))]
        config: Option<Bound<'_, PyAny>>,

        #[gen_stub(override_type(
            type_repr = "typing.Optional[dict]",
            imports = ("typing",)
        ))]
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
        Self::initializer(config, event_callback)
    }

    /// Move to an absolute position through the printhead jog API.
    /// Without a speed the server uses its configured default.
    #[pyo3(signature = (x, y, z=None, speed=None))]
    fn move_to_absolute<'py>(
        &self,
        py: Python<'py>,
        x: f64,
        y: f64,
        z: Option<f64>,
        speed: Option<f64>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            core.jog_absolute(x, y, z, speed).await?;
            Ok(())
        })
    }

    /// Fetch the census payloads (the parsed ``/api/version`` and
    /// ``/api/printer`` responses) over the session, so local and
    /// remote probing use the same code path.
    fn probe_info<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, Vec<JsonDict>>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            let (version, printer) = core.probe_info().await?;
            Python::attach(|py| {
                Ok(vec![
                    value_to_dict(py, &version)?,
                    value_to_dict(py, &printer)?,
                ])
            })
        })
    }

    /// Run the device census over the session: the parsed
    /// `/api/version` and `/api/printer` responses, interpreted
    /// into the structured census dict. Local and remote sessions
    /// probe identically.
    fn probe_device<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, OctoPrintDeviceInfo>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            let (version, printer) = core.probe_info().await?;
            Ok(OctoPrintDeviceInfo {
                inner: crate::driver::octoprint::probe::describe(
                    &version, &printer,
                ),
            })
        })
    }
    fn __repr__(&self) -> String {
        format!(
            "OctoPrintDriver(resource_uri={:?}, job_running={})",
            self.core.resource_uri(),
            self.core.is_job_running(),
        )
    }
}
