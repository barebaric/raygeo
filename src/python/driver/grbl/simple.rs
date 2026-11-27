//! The `GrblSerialSimpleDriver` PyO3 wrapper: the minimal ping-pong
//! driver exposed to Python.

use std::sync::Arc;

use crate::python::driver::base::PyDriver;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::Driver;
use crate::driver::core::transport::TransportKind;
use crate::driver::grbl::simple::{GrblSerialSimpleDriver, GrblSimpleConfig};

use crate::python::driver::core::mock::MockTransport;
use crate::python::driver::events::PyEvents;
use crate::python::driver::grbl::driver::{get_bool, get_f64, parse_dialect};
use crate::python::driver::grbl::probe::GrblDeviceInfo;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

pub(crate) const MODULE_DOC: &str = "\
The minimal ping-pong GRBL driver: one line at a time, waiting for \
each ok/error acknowledgement. For hardware where the full driver \
misbehaves.
";

pyo3_stub_gen::module_doc!("raygeo.driver.grbl.simple", "{}", MODULE_DOC);

fn build_simple_config(
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<GrblSimpleConfig> {
    let mut config = GrblSimpleConfig::new();
    let Some(dict) = dict else {
        return Ok(config);
    };
    if let Some(v) = get_bool(dict, "poll_status_while_running")? {
        config.poll_status_while_running = v;
    }
    if let Some(v) = get_f64(dict, "command_timeout")? {
        config.command_timeout = v;
    }
    if let Some(v) = get_f64(dict, "homing_timeout")? {
        config.homing_timeout = v;
    }
    if let Some(v) = get_f64(dict, "stall_timeout_min")? {
        config.stall_timeout_min = v;
    }
    if let Some(v) = get_f64(dict, "stall_timeout_max")? {
        config.stall_timeout_max = v;
    }
    if let Some(v) = get_f64(dict, "stall_timeout_safety_factor")? {
        config.stall_timeout_safety_factor = v;
    }
    if let Some(v) = get_f64(dict, "stall_timeout_default")? {
        config.stall_timeout_default = v;
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

/// A minimal GRBL device driver using ping-pong communication.
///
/// Sends one G-code line at a time and waits for `ok` before
/// proceeding.  No character-counting, no buffer management, no
/// deadlock detection — for hardware where the full
/// [`GrblSerialDriver`][raydriver.grbl.GrblSerialDriver] misbehaves.
///
/// All async methods return asyncio futures; a typed `DriverEvent`
/// is delivered to `event_callback(event)` on the event loop thread.
#[gen_stub_pyclass]
#[pyclass(name = "GrblSerialSimpleDriver", module = "raygeo.driver.grbl.simple", extends = PyDriver)]
pub struct PyGrblSerialSimpleDriver {
    core: Arc<GrblSerialSimpleDriver>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyGrblSerialSimpleDriver {
    #[new]
    #[gen_stub(override_return_type(type_repr = "GrblSerialSimpleDriver"))]
    #[pyo3(signature = (config, dialect=None, event_callback=None))]
    fn new(
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Union[dict, config.GrblSimpleConfig]]",
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
        let kind = crate::python::driver::grbl::driver::serial_transport(
            config.as_ref(),
        )?;
        let simple_config = build_simple_config(config.as_ref())?;
        Self::initializer(kind, simple_config, dialect, event_callback)
    }

    /// Build a driver around a [`MockTransport`] (for tests).
    #[staticmethod]
    #[pyo3(signature = (transport, config=None, dialect=None, event_callback=None))]
    fn with_transport(
        py: Python<'_>,
        transport: PyRef<'_, MockTransport>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[typing.Union[dict, config.GrblSimpleConfig]]",
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
        let simple_config = build_simple_config(config.as_ref())?;
        Py::new(
            py,
            Self::initializer(kind, simple_config, dialect, event_callback)?,
        )
    }

    /// Run the device census: query `$I` and `$$`, and return the
    /// structured result.
    fn probe_device<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, GrblDeviceInfo>> {
        let core = self.core.clone();
        spawn_future(py, async move {
            let build_info =
                Driver::execute_interactive_command(&*core, "$I").await?;
            let settings_lines =
                Driver::execute_interactive_command(&*core, "$$").await?;
            Ok(GrblDeviceInfo {
                inner: crate::driver::grbl::probe::describe(
                    &build_info,
                    &settings_lines,
                ),
            })
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "GrblSerialSimpleDriver(resource_uri={:?}, job_running={})",
            Driver::resource_uri(&*self.core),
            Driver::is_job_running(&*self.core),
        )
    }
}

impl PyGrblSerialSimpleDriver {
    fn initializer(
        kind: TransportKind,
        config: GrblSimpleConfig,
        dialect: Option<Bound<'_, PyDict>>,
        event_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let dialect: Dialect = parse_dialect(dialect.as_ref())?;
        let events = Arc::new(PyEvents::new(event_callback));
        let core =
            GrblSerialSimpleDriver::new(kind, config, dialect, events.clone());
        let base =
            crate::python::driver::base::PyDriver::new(core.clone(), events);
        Ok(PyClassInitializer::from(base).add_subclass(Self { core }))
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__doc__", MODULE_DOC)?;
    m.add("__all__", vec!["GrblSerialSimpleDriver"])?;
    m.add_class::<PyGrblSerialSimpleDriver>()?;
    Ok(())
}
