//! The typed session configuration pyclasses (`raydriver.config`)
//! and the conversion from typed configs to the mapping form
//! consumed by the session bindings.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

/// Accept either a plain mapping or a typed config pyclass
/// (anything exposing `to_dict()`), and return the mapping.
pub(crate) fn config_dict<'py>(
    config: Option<&Bound<'py, PyAny>>,
) -> PyResult<Option<Bound<'py, PyDict>>> {
    let Some(config) = config else {
        return Ok(None);
    };
    if let Ok(dict) = config.cast::<PyDict>() {
        return Ok(Some(dict.clone()));
    }
    let dict = config.call_method0("to_dict")?;
    Ok(Some(dict.cast_into::<PyDict>()?))
}

/// Generates a frozen config pyclass: a keyword-only `#[new]` with
/// per-field defaults, a getter per field, and `to_dict()` with the
/// None fields omitted — the surface [`config_dict`] consumes.
///
/// `required` fields have no default; `fields` always appear in
/// `to_dict()`; `optional` fields appear only when `Some`.
macro_rules! config_pyclass {
    (
        $(#[$doc:meta])*
        $name:ident,
        required: [$($req:ident: $reqty:ty),* $(,)?],
        fields: [$($field:ident: $fieldty:ty = $fielddefault:expr),* $(,)?],
        optional: [$($opt:ident: $optty:ty = $optdefault:expr),* $(,)?] $(,)?
    ) => {
        $(#[$doc])*
        #[gen_stub_pyclass]
        #[pyclass(frozen, module = "raygeo.driver.config")]
        pub struct $name {
            $(pub $req: $reqty,)*
            $(pub $field: $fieldty,)*
            $(pub $opt: $optty,)*
        }

        #[gen_stub_pymethods]
        #[pymethods]
        #[allow(clippy::too_many_arguments, clippy::clone_on_copy)]
        impl $name {
            #[new]
            #[pyo3(signature = (*,
                $($req,)*
                $($field = $fielddefault,)*
                $($opt = $optdefault,)*
            ))]
            fn new(
                $($req: $reqty,)*
                $($field: $fieldty,)*
                $($opt: $optty,)*
            ) -> Self {
                Self { $($req,)* $($field,)* $($opt,)* }
            }

            $(
            #[getter]
            fn $req(&self) -> $reqty {
                self.$req.clone()
            }
            )*
            $(
            #[getter]
            fn $field(&self) -> $fieldty {
                self.$field.clone()
            }
            )*
            $(
            #[getter]
            fn $opt(&self) -> $optty {
                self.$opt.clone()
            }
            )*

            /// The mapping form expected by the session constructors;
            /// fields that are None are omitted.
            fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
                let dict = PyDict::new(py);
                $(dict.set_item(stringify!($req), self.$req.clone())?;)*
                $(dict.set_item(stringify!($field), self.$field.clone())?;)*
                $(if let Some(value) = &self.$opt {
                    dict.set_item(stringify!($opt), value.clone())?;
                })*
                Ok(dict)
            }
        }
    };
}

config_pyclass!(
    /// Timing and flow-control options shared by GRBL drivers.
    GrblOptions,
    required: [],
    fields: [
        poll_status_while_running: bool = false,
        deadlock_detection: bool = false,
        rx_buffer_size_override: i64 = 0,
        stall_timeout_min: f64 = 5.0,
        stall_timeout_max: f64 = 120.0,
        stall_timeout_safety_factor: f64 = 3.0,
        stall_timeout_default: f64 = 30.0,
        safety_shutdown_delay: f64 = 0.2,
        unanswered_poll_limit: u32 = 3,
        poll_response_attempts: u32 = 10,
        poll_response_interval: f64 = 0.1,
        handshake_timeout: f64 = 6.0,
        handshake_poll_interval: f64 = 0.5,
        status_poll_interval: f64 = 0.5,
        reconnect_delay: f64 = 5.0,
        command_timeout: f64 = 10.0,
        homing_timeout: f64 = 0.0,
    ],
    optional: [cached_rx_buffer_size: Option<i64> = None],
);

config_pyclass!(
    /// GRBL serial session configuration.
    GrblSerialConfig,
    required: [port: String],
    fields: [
        baudrate: u32 = 115200,
        poll_status_while_running: bool = false,
        deadlock_detection: bool = false,
        rx_buffer_size_override: i64 = 0,
        stall_timeout_min: f64 = 5.0,
        stall_timeout_max: f64 = 120.0,
        stall_timeout_safety_factor: f64 = 3.0,
        stall_timeout_default: f64 = 30.0,
        safety_shutdown_delay: f64 = 0.2,
        unanswered_poll_limit: u32 = 3,
        poll_response_attempts: u32 = 10,
        poll_response_interval: f64 = 0.1,
        handshake_timeout: f64 = 6.0,
        handshake_poll_interval: f64 = 0.5,
        status_poll_interval: f64 = 0.5,
        reconnect_delay: f64 = 5.0,
        command_timeout: f64 = 10.0,
        homing_timeout: f64 = 0.0,
    ],
    optional: [cached_rx_buffer_size: Option<i64> = None],
);

config_pyclass!(
    /// GRBL telnet session configuration.
    GrblTelnetConfig,
    required: [host: String],
    fields: [
        tcp_port: u16 = 23,
        poll_status_while_running: bool = false,
        deadlock_detection: bool = false,
        rx_buffer_size_override: i64 = 0,
        stall_timeout_min: f64 = 5.0,
        stall_timeout_max: f64 = 120.0,
        stall_timeout_safety_factor: f64 = 3.0,
        stall_timeout_default: f64 = 30.0,
        safety_shutdown_delay: f64 = 0.2,
        unanswered_poll_limit: u32 = 3,
        poll_response_attempts: u32 = 10,
        poll_response_interval: f64 = 0.1,
        handshake_timeout: f64 = 6.0,
        handshake_poll_interval: f64 = 0.5,
        status_poll_interval: f64 = 0.5,
        reconnect_delay: f64 = 5.0,
        command_timeout: f64 = 10.0,
        homing_timeout: f64 = 0.0,
    ],
    optional: [cached_rx_buffer_size: Option<i64> = None],
);

config_pyclass!(
    /// GRBL over Bluetooth Classic SPP/RFCOMM.
    GrblBluetoothConfig,
    required: [address: String],
    fields: [
        channel: u8 = 1,
        poll_status_while_running: bool = false,
        deadlock_detection: bool = false,
        rx_buffer_size_override: i64 = 0,
        stall_timeout_min: f64 = 5.0,
        stall_timeout_max: f64 = 120.0,
        stall_timeout_safety_factor: f64 = 3.0,
        stall_timeout_default: f64 = 30.0,
        safety_shutdown_delay: f64 = 0.2,
        unanswered_poll_limit: u32 = 3,
        poll_response_attempts: u32 = 10,
        poll_response_interval: f64 = 0.1,
        handshake_timeout: f64 = 6.0,
        handshake_poll_interval: f64 = 0.5,
        status_poll_interval: f64 = 0.5,
        reconnect_delay: f64 = 5.0,
        command_timeout: f64 = 10.0,
        homing_timeout: f64 = 0.0,
    ],
    optional: [cached_rx_buffer_size: Option<i64> = None],
);

config_pyclass!(
    /// Configuration for the minimal ping-pong GRBL session.
    GrblSimpleConfig,
    required: [port: String],
    fields: [
        baudrate: u32 = 115200,
        poll_status_while_running: bool = false,
        deadlock_detection: bool = false,
        rx_buffer_size_override: i64 = 0,
        stall_timeout_min: f64 = 5.0,
        stall_timeout_max: f64 = 120.0,
        stall_timeout_safety_factor: f64 = 3.0,
        stall_timeout_default: f64 = 30.0,
        safety_shutdown_delay: f64 = 0.2,
        unanswered_poll_limit: u32 = 3,
        poll_response_attempts: u32 = 10,
        poll_response_interval: f64 = 0.1,
        handshake_timeout: f64 = 6.0,
        handshake_poll_interval: f64 = 0.5,
        status_poll_interval: f64 = 0.5,
        reconnect_delay: f64 = 5.0,
        command_timeout: f64 = 10.0,
        homing_timeout: f64 = 0.0,
    ],
    optional: [cached_rx_buffer_size: Option<i64> = None],
);

config_pyclass!(
    /// ESP3D / FluidNC network session configuration.
    Esp3dConfig,
    required: [host: String],
    fields: [
        http_port: u16 = 80,
        ws_port: u16 = 81,
        protocol: String = "ESP3D".to_string(),
        request_timeout: f64 = 10.0,
        connect_timeout: f64 = 6.0,
        poll_status_while_running: bool = false,
        deadlock_detection: bool = false,
        rx_buffer_size_override: i64 = 0,
        stall_timeout_min: f64 = 5.0,
        stall_timeout_max: f64 = 120.0,
        stall_timeout_safety_factor: f64 = 3.0,
        stall_timeout_default: f64 = 30.0,
        safety_shutdown_delay: f64 = 0.2,
        unanswered_poll_limit: u32 = 3,
        poll_response_attempts: u32 = 10,
        poll_response_interval: f64 = 0.1,
        handshake_timeout: f64 = 6.0,
        handshake_poll_interval: f64 = 0.5,
        status_poll_interval: f64 = 0.5,
        reconnect_delay: f64 = 5.0,
        command_timeout: f64 = 10.0,
        homing_timeout: f64 = 0.0,
    ],
    optional: [cached_rx_buffer_size: Option<i64> = None],
);

config_pyclass!(
    /// Smoothieware telnet session configuration.
    SmoothieConfig,
    required: [host: String],
    fields: [
        tcp_port: u16 = 23,
        poll_status_while_running: bool = false,
        command_timeout: f64 = 30.0,
        handshake_timeout: f64 = 15.0,
        handshake_poll_interval: f64 = 0.5,
        status_poll_interval: f64 = 1.0,
        reconnect_delay: f64 = 5.0,
        safety_shutdown_delay: f64 = 0.2,
    ],
    optional: [],
);

config_pyclass!(
    /// Marlin serial session configuration.
    MarlinConfig,
    required: [port: String],
    fields: [
        baudrate: u32 = 115200,
        poll_status_while_running: bool = false,
        command_timeout: f64 = 30.0,
        handshake_timeout: f64 = 5.0,
        handshake_poll_interval: f64 = 0.5,
        status_poll_interval: f64 = 2.0,
        reconnect_delay: f64 = 5.0,
        connect_settle: f64 = 1.0,
    ],
    optional: [],
);

config_pyclass!(
    /// xTool D-series WiFi-API session configuration.
    XtoolConfig,
    required: [host: String],
    fields: [
        http_port: u16 = 8080,
        ws_port: u16 = 8081,
        request_timeout: f64 = 5.0,
        connect_timeout: f64 = 6.0,
        reconnect_delay: f64 = 5.0,
        homing_timeout: f64 = 120.0,
        job_start_timeout: f64 = 15.0,
        poll_interval: f64 = 0.5,
    ],
    optional: [],
);

config_pyclass!(
    /// Connection settings for an OctoPrint server.
    OctoPrintConfig,
    required: [host: String],
    fields: [
        port: u16 = 80,
        path: String = "/".to_string(),
        api_key: String = String::new(),
        request_timeout: f64 = 30.0,
        connect_timeout: f64 = 10.0,
        poll_interval: f64 = 2.0,
        reconnect_delay: f64 = 5.0,
        ws_ping_interval: f64 = 30.0,
    ],
    optional: [],
);

config_pyclass!(
    /// M2 WS-V2 monitoring session configuration.
    M2Config,
    required: [host: String],
    fields: [
        port: u16 = 28900,
        request_timeout: f64 = 10.0,
        connect_timeout: f64 = 15.0,
        reconnect_delay: f64 = 5.0,
        poll_interval: f64 = 3.0,
        tls_allow_invalid_certs: bool = false,
    ],
    optional: [],
);

config_pyclass!(
    /// The daemon endpoint used by remote sessions.
    RemoteEndpoint,
    required: [remote_host: String],
    fields: [remote_port: u16 = 8765],
    optional: [
        token: Option<String> = None,
        tls_ca: Option<String> = None,
        tls_hostname: Option<String> = None,
    ],
);

/// Register the config pyclasses as the `raydriver.config`
/// submodule (stub-only: no Python file shadows it).
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let module = PyModule::new(py, "config")?;
    module.setattr(
        "__doc__",
        "Typed session configuration classes: transport and timing \
         fields per protocol, rejecting typos at type-checking time.",
    )?;
    module.add_class::<GrblOptions>()?;
    module.add_class::<GrblSerialConfig>()?;
    module.add_class::<GrblTelnetConfig>()?;
    module.add_class::<GrblBluetoothConfig>()?;
    module.add_class::<GrblSimpleConfig>()?;
    module.add_class::<Esp3dConfig>()?;
    module.add_class::<SmoothieConfig>()?;
    module.add_class::<MarlinConfig>()?;
    module.add_class::<XtoolConfig>()?;
    module.add_class::<OctoPrintConfig>()?;
    module.add_class::<M2Config>()?;
    module.add_class::<RemoteEndpoint>()?;
    crate::python::driver::core::discovery::helpers::attach_submodule(
        m,
        &module,
        "raygeo.driver.config",
    )?;
    Ok(())
}
