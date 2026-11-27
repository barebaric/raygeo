//! The `SmoothieDriver` PyO3 wrapper.

use std::sync::Arc;

use crate::python::driver::base::PyDriver;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::Driver;
use crate::driver::core::transport::TransportKind;
use crate::driver::smoothie::driver::{SmoothieConfig, SmoothieDriver};
use crate::python::driver::events::PyEvents;
use crate::python::driver::grbl::driver::{
    get_bool, get_f64, parse_dialect, telnet_transport,
};
use crate::python::driver::pyfuture::{spawn_future, PyAwait};
use crate::python::driver::smoothie::probe::SmoothieDeviceInfo;

fn build_smoothie_config(
    kind: TransportKind,
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<SmoothieConfig> {
    let mut config = SmoothieConfig::new(kind);
    let Some(dict) = dict else {
        return Ok(config);
    };
    if let Some(v) = get_bool(dict, "poll_status_while_running")? {
        config.poll_status_while_running = v;
    }
    if let Some(v) = get_f64(dict, "command_timeout")? {
        config.command_timeout = v;
    }
    if let Some(v) = get_f64(dict, "handshake_timeout")? {
        config.handshake_timeout = v;
    }
    if let Some(v) = get_f64(dict, "handshake_poll_interval")? {
        config.handshake_poll_interval = v;
    }
    if let Some(v) = get_f64(dict, "status_poll_interval")? {
        config.status_poll_interval = v;
    }
    if let Some(v) = get_f64(dict, "reconnect_delay")? {
        config.reconnect_delay = v;
    }
    if let Some(v) = get_f64(dict, "safety_shutdown_delay")? {
        config.safety_shutdown_delay = v;
    }
    Ok(config)
}

/// A Smoothieware device session using ping-pong communication.
///
/// Sends one command at a time and waits for `ok` before
/// proceeding; status reports are Grbl-style `<...>` replies to `?`.
///
/// All async methods return asyncio futures; a typed `DriverEvent`
/// is delivered to `event_callback(event)` on the event loop thread.
#[gen_stub_pyclass]
#[pyclass(name = "SmoothieDriver", module = "raygeo.driver.smoothie", extends = PyDriver)]
pub struct PySmoothieDriver {
    core: Arc<SmoothieDriver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySmoothieDriver {
    #[new]
    #[gen_stub(override_return_type(type_repr = "SmoothieDriver"))]
    #[pyo3(signature = (config, dialect=None, event_callback=None))]
    fn new(
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Union[dict, config.SmoothieConfig]]",
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
        let config =
            crate::python::driver::config::config_dict(config.as_ref())?;
        let kind = telnet_transport(config.as_ref())?;
        let session_config = build_smoothie_config(kind, config.as_ref())?;
        Self::initializer(session_config, dialect, event_callback)
    }

    /// Build a session around a [`MockTransport`] (for tests).
    #[staticmethod]
    #[pyo3(signature = (transport, config=None, dialect=None, event_callback=None))]
    fn with_transport(
        py: Python<'_>,
        transport: PyRef<'_, crate::python::driver::core::mock::MockTransport>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Union[dict, config.SmoothieConfig]]",
            imports = ("typing", "raygeo.driver.config")
        ))]
        config: Option<Bound<'_, PyAny>>,

        dialect: Option<Bound<'_, PyDict>>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[collections.abc.Callable[[core.DriverEvent], builtins.object]]",
            imports = ("builtins", "collections.abc", "typing", "raygeo.driver.core")
        ))]
        event_callback: Option<Py<PyAny>>,
    ) -> PyResult<Py<Self>> {
        let config =
            crate::python::driver::config::config_dict(config.as_ref())?;
        let kind = TransportKind::Mock(transport.inner.clone());
        let session_config = build_smoothie_config(kind, config.as_ref())?;
        Py::new(
            py,
            Self::initializer(session_config, dialect, event_callback)?,
        )
    }
    /// Send a command and await its full response.
    /// Run the device census: query ``version`` and the
    /// ``config-get`` motion keys, and return the structured result
    /// as a dict.
    fn probe_device<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, SmoothieDeviceInfo>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            let mut replies: Vec<Vec<String>> = Vec::new();
            for command in crate::driver::smoothie::probe::CENSUS_COMMANDS {
                replies.push(core.execute_interactive_command(command).await?);
            }
            Ok(SmoothieDeviceInfo {
                inner: crate::driver::smoothie::probe::describe(
                    &replies[0],
                    &replies[1],
                    &replies[2],
                    &replies[3],
                    &replies[4],
                    &replies[5],
                ),
            })
        })
    }
    fn __repr__(&self) -> String {
        format!(
            "SmoothieDriver(resource_uri={:?}, job_running={})",
            Driver::resource_uri(&*self.core),
            Driver::is_job_running(&*self.core)
        )
    }
}

impl PySmoothieDriver {
    fn initializer(
        config: SmoothieConfig,
        dialect: Option<Bound<'_, PyDict>>,
        event_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let dialect: Dialect = parse_dialect(dialect.as_ref())?;
        let events = Arc::new(PyEvents::new(event_callback));
        let core = SmoothieDriver::new(config, dialect, events.clone());
        let base =
            crate::python::driver::base::PyDriver::new(core.clone(), events);
        Ok(PyClassInitializer::from(base).add_subclass(Self { core }))
    }
}
