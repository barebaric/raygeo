//! The `MarlinSerialDriver` PyO3 wrapper.

use std::sync::Arc;

use crate::python::driver::base::PyDriver;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::Driver;
use crate::driver::core::transport::TransportKind;
use crate::driver::marlin::driver::{MarlinConfig, MarlinSerialDriver};
use crate::python::driver::events::PyEvents;
use crate::python::driver::grbl::driver::{
    get_bool, get_f64, parse_dialect, serial_transport,
};
use crate::python::driver::marlin::probe::MarlinDeviceInfo;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

fn build_marlin_config(
    kind: TransportKind,
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<MarlinConfig> {
    let mut config = MarlinConfig::new(kind);
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
    if let Some(v) = get_f64(dict, "connect_settle")? {
        config.connect_settle = v;
    }
    Ok(config)
}

/// A Marlin device session using ping-pong communication.
///
/// Sends one command at a time and waits for `ok` before
/// proceeding; the device announces itself with a `start` line and
/// positions are polled with `M114`.  Marlin has no feed-hold
/// support — pausing is the caller's concern.
///
/// All async methods return asyncio futures; a typed `DriverEvent`
/// is delivered to `event_callback(event)` on the event loop thread.
#[gen_stub_pyclass]
#[pyclass(name = "MarlinSerialDriver", module = "raygeo.driver.marlin", extends = PyDriver)]
pub struct PyMarlinSerialDriver {
    core: Arc<MarlinSerialDriver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyMarlinSerialDriver {
    #[new]
    #[gen_stub(override_return_type(type_repr = "MarlinSerialDriver"))]
    #[pyo3(signature = (config, dialect=None, event_callback=None))]
    fn new(
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Union[dict, config.MarlinConfig]]",
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
        let kind = serial_transport(config.as_ref())?;
        let session_config = build_marlin_config(kind, config.as_ref())?;
        Self::initializer(session_config, dialect, event_callback)
    }

    /// Build a session around a [`MockTransport`] (for tests).
    #[staticmethod]
    #[pyo3(signature = (transport, config=None, dialect=None, event_callback=None))]
    fn with_transport(
        py: Python<'_>,
        transport: PyRef<'_, crate::python::driver::core::mock::MockTransport>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Union[dict, config.MarlinConfig]]",
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
        let session_config = build_marlin_config(kind, config.as_ref())?;
        Py::new(
            py,
            Self::initializer(session_config, dialect, event_callback)?,
        )
    }

    /// Open the connection and start the background tasks.
    /// Run the device census: query M115, M211, M503 and M149,
    /// and return the structured result as a dict. *boot_lines* are
    /// the startup messages the caller observed on the session,
    /// used to refine the firmware version.
    #[pyo3(signature = (boot_lines=None))]
    fn probe_device<'py>(
        &self,
        py: Python<'py>,
        boot_lines: Option<Vec<String>>,
    ) -> PyResult<PyAwait<'py, MarlinDeviceInfo>> {
        let boot_lines = boot_lines.unwrap_or_else(|| self.core.boot_lines());
        let core = self.core.clone();
        spawn_future(py, async move {
            let mut replies: Vec<Vec<String>> = Vec::new();
            for command in crate::driver::marlin::probe::CENSUS_COMMANDS {
                let lines = core.execute_interactive_command(command).await?;
                replies.push(crate::driver::marlin::probe::filter_ok_lines(
                    &lines,
                ));
            }
            Ok(MarlinDeviceInfo {
                inner: crate::driver::marlin::probe::describe(
                    &replies[0],
                    &replies[1],
                    &replies[2],
                    &replies[3],
                    &boot_lines,
                ),
            })
        })
    }

    /// Boot banner lines received from the device.
    #[getter]
    fn boot_lines(&self) -> Vec<String> {
        self.core.boot_lines()
    }

    fn __repr__(&self) -> String {
        format!(
            "MarlinSerialDriver(resource_uri={:?}, job_running={})",
            Driver::resource_uri(&*self.core),
            Driver::is_job_running(&*self.core)
        )
    }
}

impl PyMarlinSerialDriver {
    fn initializer(
        config: MarlinConfig,
        dialect: Option<Bound<'_, PyDict>>,
        event_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let dialect: Dialect = parse_dialect(dialect.as_ref())?;
        let events = Arc::new(PyEvents::new(event_callback));
        let core = MarlinSerialDriver::new(config, dialect, events.clone());
        let base =
            crate::python::driver::base::PyDriver::new(core.clone(), events);
        Ok(PyClassInitializer::from(base).add_subclass(Self { core }))
    }
}
