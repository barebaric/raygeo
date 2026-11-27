//! The `raygeo.driver.grbl` PyO3 module: the four GRBL driver classes,
//! shared config parsing and the dialect helpers every protocol
//! binding reuses.

use std::collections::HashMap;
use std::sync::Arc;

use crate::python::driver::base::PyDriver;
use pyo3::exceptions::{
    PyConnectionError, PyNotImplementedError, PyRuntimeError, PyTimeoutError,
    PyValueError,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::{Driver, DriverError};
use crate::driver::core::transport::TransportKind;
use crate::driver::esp3d::transport::{
    Esp3dConfig, Esp3dTransport, ProtocolVariant,
};
use crate::driver::grbl::bluetooth::GrblBluetoothDriver;
use crate::driver::grbl::network::GrblNetworkDriver;
use crate::driver::grbl::serial::GrblSerialDriver;
use crate::driver::grbl::stream::GrblStreamConfig;
use crate::driver::grbl::telnet::GrblTelnetDriver;

use crate::python::driver::core::mock::MockTransport;
use crate::python::driver::events::PyEvents;
use crate::python::driver::grbl::probe::GrblDeviceInfo;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};

impl From<DriverError> for PyErr {
    fn from(err: DriverError) -> Self {
        match err {
            DriverError::Connection(msg) => PyConnectionError::new_err(msg),
            DriverError::Timeout(message) => PyTimeoutError::new_err(message),
            DriverError::Device(msg) => PyRuntimeError::new_err(msg),
            DriverError::BufferStall(msg) => {
                PyConnectionError::new_err(format!("buffer stall: {msg}"))
            }
            DriverError::Unsupported(msg) => {
                PyNotImplementedError::new_err(msg)
            }
            DriverError::ResourceBusy { resource, owner } => {
                PyConnectionError::new_err(format!(
                    "{resource} is already in use by {owner}"
                ))
            }
            DriverError::Other(msg) => PyRuntimeError::new_err(msg),
        }
    }
}

pub(crate) fn get_bool(
    dict: &Bound<'_, PyDict>,
    key: &str,
) -> PyResult<Option<bool>> {
    match dict.get_item(key)? {
        Some(v) => Ok(Some(v.extract::<bool>()?)),
        None => Ok(None),
    }
}

pub(crate) fn get_f64(
    dict: &Bound<'_, PyDict>,
    key: &str,
) -> PyResult<Option<f64>> {
    match dict.get_item(key)? {
        Some(v) => {
            if let Ok(b) = v.extract::<bool>() {
                return Ok(Some(if b { 1.0 } else { 0.0 }));
            }
            Ok(Some(v.extract::<f64>()?))
        }
        None => Ok(None),
    }
}

pub(crate) fn get_i64(
    dict: &Bound<'_, PyDict>,
    key: &str,
) -> PyResult<Option<i64>> {
    match dict.get_item(key)? {
        Some(v) => Ok(Some(v.extract::<i64>()?)),
        None => Ok(None),
    }
}

pub(crate) fn get_str(
    dict: &Bound<'_, PyDict>,
    key: &str,
) -> PyResult<Option<String>> {
    match dict.get_item(key)? {
        Some(v) => Ok(Some(v.extract::<String>()?)),
        None => Ok(None),
    }
}

pub(crate) fn build_config(
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<GrblStreamConfig> {
    let mut config = GrblStreamConfig::new();
    let Some(dict) = dict else {
        return Ok(config);
    };
    if let Some(v) = get_bool(dict, "poll_status_while_running")? {
        config.poll_status_while_running = v;
    }
    if let Some(v) = get_bool(dict, "deadlock_detection")? {
        config.deadlock_detection = v;
    }
    if let Some(v) = get_i64(dict, "rx_buffer_size_override")? {
        config.rx_buffer_size_override = v;
    }
    if let Some(v) = get_i64(dict, "cached_rx_buffer_size")? {
        config.cached_rx_buffer_size = if v > 0 { Some(v) } else { None };
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
    if let Some(v) = get_f64(dict, "safety_shutdown_delay")? {
        config.safety_shutdown_delay = v;
    }
    if let Some(v) = get_i64(dict, "unanswered_poll_limit")? {
        config.unanswered_poll_limit = v.max(0) as u32;
    }
    if let Some(v) = get_i64(dict, "poll_response_attempts")? {
        config.poll_response_attempts = v.max(0) as u32;
    }
    if let Some(v) = get_f64(dict, "poll_response_interval")? {
        config.poll_response_interval = v;
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
    if let Some(v) = get_f64(dict, "command_timeout")? {
        config.command_timeout = v;
    }
    if let Some(v) = get_f64(dict, "homing_timeout")? {
        config.homing_timeout = v;
    }
    Ok(config)
}

pub(crate) fn serial_transport(
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<TransportKind> {
    let Some(dict) = dict else {
        return Err(PyRuntimeError::new_err(
            "config with a 'port' is required",
        ));
    };
    let port = get_str(dict, "port")?.unwrap_or_default();
    let baudrate = get_i64(dict, "baudrate")?.unwrap_or(115200).max(0) as u32;
    Ok(TransportKind::Serial { port, baudrate })
}

pub(crate) fn telnet_transport(
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<TransportKind> {
    let Some(dict) = dict else {
        return Err(PyRuntimeError::new_err(
            "config with a 'host' is required",
        ));
    };
    let host = get_str(dict, "host")?.unwrap_or_default();
    let port = get_i64(dict, "tcp_port")?.unwrap_or(23).max(0) as u16;
    Ok(TransportKind::Telnet { host, port })
}

fn bluetooth_transport(
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<TransportKind> {
    let (address, channel) = match dict {
        Some(dict) => (
            get_str(dict, "address")?.unwrap_or_default(),
            get_i64(dict, "channel")?.unwrap_or(1),
        ),
        None => (String::new(), 1),
    };
    if !(1..=30).contains(&channel) {
        return Err(PyValueError::new_err(
            "Bluetooth channel must be between 1 and 30",
        ));
    }
    let address =
        crate::driver::core::transport::bluetooth::normalize_address(&address)
            .map_err(PyValueError::new_err)?;
    Ok(TransportKind::Bluetooth {
        address,
        channel: channel as u8,
    })
}

fn network_transport(
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<TransportKind> {
    let Some(dict) = dict else {
        return Err(PyRuntimeError::new_err(
            "config with a 'host' is required",
        ));
    };
    let host = get_str(dict, "host")?.unwrap_or_default();
    let mut esp3d = Esp3dConfig::new(&host);
    esp3d.http_port = get_i64(dict, "http_port")?
        .or(get_i64(dict, "port")?)
        .unwrap_or(80)
        .max(0) as u16;
    esp3d.ws_port = get_i64(dict, "ws_port")?.unwrap_or(81).max(0) as u16;
    esp3d.protocol = ProtocolVariant::from_name(
        get_str(dict, "protocol")?.as_deref().unwrap_or_default(),
    );
    if let Some(timeout) = get_f64(dict, "request_timeout")? {
        esp3d.request_timeout = timeout;
    }
    if let Some(timeout) = get_f64(dict, "connect_timeout")? {
        esp3d.connect_timeout = timeout;
    }
    Ok(TransportKind::Custom(Esp3dTransport::new(esp3d)))
}

pub(crate) fn parse_dialect(
    dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<Dialect> {
    let mut safety_off_commands = None;
    let mut map: HashMap<String, String> = HashMap::new();
    if let Some(dict) = dict {
        for (key, value) in dict.iter() {
            let key: String = key.extract()?;
            if key == "safety_off_commands" {
                safety_off_commands = Some(value.extract::<Vec<String>>()?);
                continue;
            }
            if let Ok(text) = value.extract::<String>() {
                map.insert(key, text);
            }
        }
    }
    let mut dialect = Dialect::from_map(&map);
    if let Some(commands) = safety_off_commands {
        dialect.safety_off_commands = commands;
    }
    Ok(dialect)
}

macro_rules! grbl_driver {
    (
        $cls:ident,
        $py_name:literal,
        $doc:literal,
        $config_repr:literal,
        $driver_ty:ty,
        $transport_fn:ident
    ) => {
        #[doc = $doc]
        #[gen_stub_pyclass]
        #[pyclass(name = $py_name, module = "raygeo.driver.grbl", extends = PyDriver)]
        pub struct $cls {
            core: Arc<$driver_ty>,
        }

        #[gen_stub_pymethods]
        #[pymethods]
        impl $cls {
            #[new]
            #[gen_stub(override_return_type(type_repr = $py_name))]
            #[pyo3(signature = (config, dialect=None, event_callback=None))]
            fn new(
                #[gen_stub(override_type(
                    type_repr = $config_repr,
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
                let kind = $transport_fn(config.as_ref())?;
                let stream_config = build_config(config.as_ref())?;
                Self::initializer(kind, stream_config, dialect, event_callback)
            }

            /// Build a driver around a [`MockTransport`] (for
            /// tests).
            #[staticmethod]
            #[pyo3(signature = (transport, config=None, dialect=None, event_callback=None))]
            fn with_transport(
                py: Python<'_>,
                transport: PyRef<'_, MockTransport>,
                #[gen_stub(override_type(
                    type_repr = $config_repr,
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
                let stream_config = build_config(config.as_ref())?;
                let initializer = Self::initializer(
                    kind,
                    stream_config,
                    dialect,
                    event_callback,
                )?;
                Py::new(py, initializer)
            }

            /// Run the device census: query `$I` and `$$`.
            fn probe_device<'py>(
                &self,
                py: Python<'py>,
            ) -> PyResult<PyAwait<'py, GrblDeviceInfo>> {
                let core = self.core.clone();
                spawn_future(py, async move {
                    let build_info = Driver::execute_interactive_command(
                        &*core, "$I",
                    )
                    .await?;
                    let settings_lines =
                        Driver::execute_interactive_command(&*core, "$$")
                            .await?;
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
                    concat!(
                        $py_name,
                        "(resource_uri={:?}, job_running={})"
                    ),
                    Driver::resource_uri(&*self.core),
                    Driver::is_job_running(&*self.core),
                )
            }
        }

        impl $cls {
            fn initializer(
                kind: TransportKind,
                config: GrblStreamConfig,
                dialect: Option<Bound<'_, PyDict>>,
                event_callback: Option<Py<PyAny>>,
            ) -> PyResult<PyClassInitializer<Self>> {
                let dialect = parse_dialect(dialect.as_ref())?;
                let events = Arc::new(PyEvents::new(event_callback));
                let core = <$driver_ty>::with_transport(
                    kind,
                    config,
                    dialect,
                    events.clone(),
                );
                let base = crate::python::driver::base::PyDriver::new(
                    core.clone(),
                    events,
                );
                Ok(PyClassInitializer::from(base).add_subclass(Self { core }))
            }
        }
    };
}

grbl_driver!(
    PyGrblSerialDriver,
    "GrblSerialDriver",
    "A GRBL-compatible device on a serial port: connection lifecycle, \
     streaming and interactive commands, entirely in Rust.",
    "typing.Optional[typing.Union[dict, config.GrblSerialConfig]]",
    GrblSerialDriver,
    serial_transport
);

grbl_driver!(
    PyGrblTelnetDriver,
    "GrblTelnetDriver",
    "A GRBL-compatible device over a telnet connection.",
    "typing.Optional[typing.Union[dict, config.GrblTelnetConfig]]",
    GrblTelnetDriver,
    telnet_transport
);

grbl_driver!(
    PyGrblBluetoothDriver,
    "GrblBluetoothDriver",
    "A GRBL-compatible device on a Bluetooth serial channel.",
    "typing.Optional[typing.Union[dict, config.GrblBluetoothConfig]]",
    GrblBluetoothDriver,
    bluetooth_transport
);

grbl_driver!(
    PyGrblNetworkDriver,
    "GrblNetworkDriver",
    "A GRBL-compatible device on the FluidNC / ESP3D HTTP + WebSocket \
     interface.",
    "typing.Optional[typing.Union[dict, config.Esp3dConfig]]",
    GrblNetworkDriver,
    network_transport
);
