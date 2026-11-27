//! Python bindings for the remote client proxy.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use pyo3::exceptions::{
    PyConnectionError, PyNotImplementedError, PyTimeoutError, PyValueError,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};
use serde_json::{json, Value};

use crate::driver::core::remote::client::{
    self as core, RemoteEndpoint, RemoteError, RemoteErrorKind, RemoteEvents,
    RemoteSession as CoreRemoteSession,
};
use crate::driver::core::remote::protocol::WireEvent;
use crate::driver::core::types::WcsOffsets;
use crate::python::driver::census::{census_from_json, DeviceCensus};
use crate::python::driver::core::json::{value_to_dict, value_to_py, JsonDict};
use crate::python::driver::core::types::DeviceState as PyDeviceState;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};
use crate::python::driver::remote::bridge::{py_to_json, running_loop};
use crate::python::driver::remote::events::wire_to_py_event;
use crate::python::driver::remote::session::PySessionFactory;

const DEFAULT_PORT: u16 = 8765;

pub(crate) struct PyRemoteEvents {
    event_callback: Option<Py<PyAny>>,
    progress_callback: Mutex<Option<Py<PyAny>>>,
    loop_: Mutex<Option<Py<PyAny>>>,
}

impl PyRemoteEvents {
    fn new(
        event_callback: Option<Py<PyAny>>,
        loop_: Option<Py<PyAny>>,
    ) -> Self {
        Self {
            event_callback,
            progress_callback: Mutex::new(None),
            loop_: Mutex::new(loop_),
        }
    }

    fn set_progress_callback(&self, callback: Option<Py<PyAny>>) {
        *self.progress_callback.lock().unwrap() = callback;
    }

    fn capture_loop(&self, py: Python<'_>) -> PyResult<()> {
        let mut slot = self.loop_.lock().unwrap();
        if slot.is_none() {
            *slot = Some(running_loop(py)?);
        }
        Ok(())
    }
}

impl RemoteEvents for PyRemoteEvents {
    fn event(&self, event: WireEvent) {
        let Some(callback) = &self.event_callback else {
            return;
        };
        Python::attach(|py| {
            let Ok(Some(event)) = wire_to_py_event(py, &event) else {
                return;
            };
            let loop_ = self
                .loop_
                .lock()
                .unwrap()
                .as_ref()
                .map(|loop_| loop_.clone_ref(py));
            let Some(loop_) = loop_ else {
                return;
            };
            let _ = loop_.bind(py).call_method1(
                "call_soon_threadsafe",
                (callback.clone_ref(py), event),
            );
        });
    }

    fn progress(&self, op: i64) {
        Python::attach(|py| {
            let callback = self
                .progress_callback
                .lock()
                .unwrap()
                .as_ref()
                .map(|callback| callback.clone_ref(py));
            let Some(callback) = callback else {
                return;
            };
            let loop_ = self
                .loop_
                .lock()
                .unwrap()
                .as_ref()
                .map(|loop_| loop_.clone_ref(py));
            let Some(loop_) = loop_ else {
                return;
            };
            let _ = loop_
                .bind(py)
                .call_method1("call_soon_threadsafe", (callback, op));
        });
    }
}

/// The proxy with the union of the sessions' method surfaces.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.remote.client")]
pub struct RemoteSession {
    inner: Arc<CoreRemoteSession>,
    events: Arc<PyRemoteEvents>,
}

#[gen_stub_pymethods]
#[pymethods]
impl RemoteSession {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        protocol_name,
        config,
        dialect,
        event_callback,
        endpoint,
        device = None,
        session_id = None,
    ))]
    fn new(
        py: Python<'_>,
        protocol_name: Option<String>,
        #[gen_stub(override_type(
            type_repr = "dict[builtins.str, builtins.object]",
            imports = ("builtins",)
        ))]
        config: Py<PyAny>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[dict[builtins.str, builtins.object]]",
            imports = ("typing", "builtins")
        ))]
        dialect: Option<Py<PyAny>>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[collections.abc.Callable[[core.DriverEvent], builtins.object]]",
            imports = ("typing", "collections.abc", "raygeo.driver.core", "builtins")
        ))]
        event_callback: Option<Py<PyAny>>,
        #[gen_stub(override_type(
            type_repr = "typing.Union[dict[builtins.str, builtins.object], builtins.str]",
            imports = ("typing", "builtins")
        ))]
        endpoint: Py<PyAny>,
        device: Option<String>,
        session_id: Option<String>,
    ) -> PyResult<Self> {
        let config = config_to_value(py, &config)?;
        let dialect = dialect
            .map(|dialect| config_to_value(py, &dialect))
            .transpose()?;
        let endpoint = parse_endpoint(py, &endpoint)?;
        let loop_ = running_loop(py).ok();
        let events = Arc::new(PyRemoteEvents::new(event_callback, loop_));
        let inner = CoreRemoteSession::new(
            protocol_name,
            config,
            dialect,
            endpoint,
            device,
            session_id,
            events.clone(),
        );
        Ok(Self { inner, events })
    }

    #[getter]
    fn protocol(&self) -> Option<String> {
        self.inner.protocol()
    }

    #[getter]
    fn metadata(&self, py: Python<'_>) -> PyResult<Option<JsonDict>> {
        match self.inner.metadata() {
            Some(metadata) => Ok(Some(value_to_dict(py, &metadata)?)),
            None => Ok(None),
        }
    }

    #[getter]
    fn state(&self) -> PyDeviceState {
        PyDeviceState::new(self.inner.state())
    }

    #[getter]
    fn job_running(&self) -> bool {
        self.inner.job_running()
    }

    #[getter]
    fn resource_uri(&self) -> Option<String> {
        self.inner.resource_uri()
    }

    #[getter]
    fn buffer_count(&self) -> i64 {
        self.inner.buffer_count()
    }

    #[getter]
    fn rx_buffer_size(&self) -> i64 {
        self.inner.rx_buffer_size()
    }

    #[getter]
    fn pending_commands(&self) -> Vec<String> {
        self.inner.pending_commands()
    }

    #[getter]
    fn _endpoint(&self, py: Python<'_>) -> PyResult<JsonDict> {
        let endpoint = self.inner.endpoint();
        value_to_dict(
            py,
            &json!({
                "remote_host": endpoint.host,
                "remote_port": endpoint.port,
                "token": endpoint.token,
                "tls_ca": endpoint.tls_ca,
                "tls_hostname": endpoint.tls_hostname,
            }),
        )
    }

    #[pyo3(signature = (message = None))]
    fn drop_connection(&self, message: Option<String>) {
        self.inner
            .force_disconnect(message.as_deref().unwrap_or("connection lost"));
    }

    fn connect<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        self.events.capture_loop(py)?;
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.connect().await.map_err(connection)?;
            Ok(())
        })
    }

    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.disconnect().await.map_err(connection)?;
            Ok(())
        })
    }

    fn attach<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        self.events.capture_loop(py)?;
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.attach().await.map_err(connection)?;
            Ok(())
        })
    }

    fn aclose<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.aclose().await;
            Ok(())
        })
    }

    fn execute_interactive_command<'py>(
        &self,
        py: Python<'py>,
        command: String,
    ) -> PyResult<PyAwait<'py, Vec<String>>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let value = inner
                .execute_interactive_command(&command)
                .await
                .map_err(connection)?;
            decode_lines(value)
        })
    }

    fn execute_command<'py>(
        &self,
        py: Python<'py>,
        command: String,
    ) -> PyResult<PyAwait<'py, Vec<String>>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let value =
                inner.execute_command(&command).await.map_err(connection)?;
            decode_lines(value)
        })
    }

    #[pyo3(signature = (
        gcode,
        op_line_map = None,
        op_estimates = None,
        progress_callback = None,
    ))]
    fn run<'py>(
        &self,
        py: Python<'py>,
        gcode: String,
        op_line_map: Option<HashMap<usize, i64>>,
        op_estimates: Option<Vec<f64>>,
        #[gen_stub(override_type(
            type_repr = "typing.Optional[collections.abc.Callable[[builtins.int], builtins.object]]",
            imports = ("typing", "collections.abc")
        ))]
        progress_callback: Option<Py<PyAny>>,
    ) -> PyResult<PyAwait<'py, ()>> {
        self.events.capture_loop(py)?;
        self.events.set_progress_callback(progress_callback);
        let inner = self.inner.clone();
        let events = self.events.clone();
        spawn_future(py, async move {
            let result = inner.run(&gcode, op_line_map, op_estimates).await;
            events.set_progress_callback(None);
            result.map_err(connection)?;
            Ok(())
        })
    }

    fn run_raw<'py>(
        &self,
        py: Python<'py>,
        machine_code: String,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.run_raw(&machine_code).await.map_err(connection)?;
            Ok(())
        })
    }

    #[pyo3(signature = (hold = true))]
    fn set_hold<'py>(
        &self,
        py: Python<'py>,
        hold: bool,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.set_hold(hold).await.map_err(connection)?;
            Ok(())
        })
    }

    #[pyo3(signature = (emergency = false))]
    fn cancel<'py>(
        &self,
        py: Python<'py>,
        emergency: bool,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.cancel(emergency).await.map_err(connection)?;
            Ok(())
        })
    }

    #[pyo3(signature = (axes = None, active_wcs = None))]
    fn home<'py>(
        &self,
        py: Python<'py>,
        axes: Option<Vec<String>>,
        active_wcs: Option<String>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let active_wcs = active_wcs.unwrap_or_else(|| "G54".to_string());
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.home(axes, &active_wcs).await.map_err(connection)?;
            Ok(())
        })
    }

    #[pyo3(signature = (speed, x, y, z=None))]
    fn move_to<'py>(
        &self,
        py: Python<'py>,
        speed: f64,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.move_to(speed, x, y, z).await.map_err(connection)?;
            Ok(())
        })
    }

    #[pyo3(signature = (speed, deltas=None))]
    fn jog<'py>(
        &self,
        py: Python<'py>,
        speed: f64,
        deltas: Option<Vec<(String, f64)>>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let deltas = serde_json::to_value(deltas.unwrap_or_default())
                .map_err(|err| PyValueError::new_err(err.to_string()))?;
            inner.jog(speed, deltas).await.map_err(connection)?;
            Ok(())
        })
    }

    fn select_tool<'py>(
        &self,
        py: Python<'py>,
        tool_number: i64,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.select_tool(tool_number).await.map_err(connection)?;
            Ok(())
        })
    }

    #[pyo3(signature = (power = None))]
    fn set_power<'py>(
        &self,
        py: Python<'py>,
        power: Option<f64>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.set_power(power).await.map_err(connection)?;
            Ok(())
        })
    }

    #[pyo3(signature = (power = None))]
    fn set_focus_power<'py>(
        &self,
        py: Python<'py>,
        power: Option<f64>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.set_focus_power(power).await.map_err(connection)?;
            Ok(())
        })
    }

    /// Run the device census over the session; marlin sessions
    /// accept the boot lines captured at connect time.
    #[pyo3(signature = (boot_lines = None))]
    fn probe_device<'py>(
        &self,
        py: Python<'py>,
        boot_lines: Option<Vec<String>>,
    ) -> PyResult<PyAwait<'py, DeviceCensus>> {
        let protocol = self.inner.protocol().unwrap_or_default();
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let args = match boot_lines {
                Some(lines) => json!({"boot_lines": lines}),
                None => json!({}),
            };
            let value = inner
                .command("probe_device", args)
                .await
                .map_err(connection)?;
            Python::attach(|py| census_from_json(py, &protocol, &value))
        })
    }

    fn read_settings<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, Vec<(String, String)>>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let value = inner.read_settings().await.map_err(connection)?;
            decode_settings(value)
        })
    }

    fn write_setting<'py>(
        &self,
        py: Python<'py>,
        key: String,
        value: String,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner
                .write_setting(&key, &value)
                .await
                .map_err(connection)?;
            Ok(())
        })
    }

    #[pyo3(signature = (wcs_slot, x, y, z=None))]
    fn set_wcs_offset<'py>(
        &self,
        py: Python<'py>,
        wcs_slot: String,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner
                .set_wcs_offset(&wcs_slot, x, y, z)
                .await
                .map_err(connection)?;
            Ok(())
        })
    }

    fn read_wcs_offsets<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, WcsOffsets>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let value = inner.read_wcs_offsets().await.map_err(connection)?;
            decode_wcs_offsets(value)
        })
    }

    fn read_parser_state<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, Option<String>>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let value = inner.read_parser_state().await.map_err(connection)?;
            decode_optional_string(value)
        })
    }

    fn run_probe_cycle<'py>(
        &self,
        py: Python<'py>,
        axis_letter: String,
        max_travel: f64,
        feed_rate: f64,
    ) -> PyResult<PyAwait<'py, Option<(f64, f64, f64)>>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let value = inner
                .run_probe_cycle(&axis_letter, max_travel, feed_rate)
                .await
                .map_err(connection)?;
            decode_probe_point(value)
        })
    }

    fn detect_unit_system<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<PyAwait<'py, Option<String>>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            let value = inner.detect_unit_system().await.map_err(connection)?;
            decode_optional_string(value)
        })
    }

    fn clear_alarm<'py>(&self, py: Python<'py>) -> PyResult<PyAwait<'py, ()>> {
        let inner = self.inner.clone();
        spawn_future(py, async move {
            inner.clear_alarm().await.map_err(connection)?;
            Ok(())
        })
    }

    fn update_dialect(
        &self,
        py: Python<'_>,
        #[gen_stub(override_type(
            type_repr = "dict[builtins.str, builtins.object]",
            imports = ("builtins",)
        ))]
        dialect: Py<PyAny>,
    ) -> PyResult<()> {
        let dialect = config_to_value(py, &dialect)?;
        self.inner.update_dialect(dialect);
        Ok(())
    }
}

fn connection(err: RemoteError) -> PyErr {
    match err.kind {
        RemoteErrorKind::Timeout => PyTimeoutError::new_err(err.message),
        RemoteErrorKind::Unsupported => {
            PyNotImplementedError::new_err(err.message)
        }
        RemoteErrorKind::Connection
        | RemoteErrorKind::Device
        | RemoteErrorKind::Internal => PyConnectionError::new_err(err.message),
    }
}

fn config_to_value(py: Python<'_>, value: &Py<PyAny>) -> PyResult<Value> {
    let bound = value.bind(py);
    if let Ok(dict) = bound.cast::<PyDict>() {
        return py_to_json(dict.as_any());
    }
    if bound.hasattr("to_dict")? {
        let converted = bound.call_method0("to_dict")?;
        return py_to_json(&converted);
    }
    py_to_json(bound)
}

/// Decodes the response lines a hosted session returns for
/// `execute_command` / `execute_interactive_command`.
fn decode_lines(value: Value) -> PyResult<Vec<String>> {
    let lines = value.as_array().ok_or_else(|| {
        PyValueError::new_err("expected a list of response lines")
    })?;
    lines
        .iter()
        .map(|line| {
            line.as_str().map(str::to_string).ok_or_else(|| {
                PyValueError::new_err("response line is not a string")
            })
        })
        .collect()
}

/// Decodes the `[key, value]` pairs `read_settings` returns.
fn decode_settings(value: Value) -> PyResult<Vec<(String, String)>> {
    let entries = value.as_array().ok_or_else(|| {
        PyValueError::new_err("expected a list of [key, value] pairs")
    })?;
    entries
        .iter()
        .map(|entry| {
            let pair = entry.as_array().ok_or_else(|| {
                PyValueError::new_err("setting is not a [key, value] pair")
            })?;
            if pair.len() != 2 {
                return Err(PyValueError::new_err(
                    "setting is not a [key, value] pair",
                ));
            }
            Ok((setting_text(&pair[0])?, setting_text(&pair[1])?))
        })
        .collect()
}

fn setting_text(value: &Value) -> PyResult<String> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| PyValueError::new_err("setting entry is not a string"))
}

/// Decodes the `[x, y, z]` triples `read_wcs_offsets` returns, keyed
/// by WCS slot.
fn decode_wcs_offsets(
    value: Value,
) -> PyResult<HashMap<String, (f64, f64, f64)>> {
    let entries = value.as_object().ok_or_else(|| {
        PyValueError::new_err("expected a mapping of WCS offsets")
    })?;
    let mut offsets = HashMap::with_capacity(entries.len());
    for (slot, triple) in entries {
        let mismatch = || {
            PyValueError::new_err(format!(
                "WCS offset {slot:?} is not an [x, y, z] triple"
            ))
        };
        let coords = triple.as_array().ok_or_else(&mismatch)?;
        let read = |index: usize| -> PyResult<f64> {
            coords
                .get(index)
                .and_then(Value::as_f64)
                .ok_or_else(&mismatch)
        };
        offsets.insert(slot.clone(), (read(0)?, read(1)?, read(2)?));
    }
    Ok(offsets)
}

fn decode_optional_string(value: Value) -> PyResult<Option<String>> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text)),
        other => Err(PyValueError::new_err(format!(
            "expected a string or null, got {other:?}"
        ))),
    }
}

/// Decodes the probe position `run_probe_cycle` returns (null on a
/// failed probe).
fn decode_probe_point(value: Value) -> PyResult<Option<(f64, f64, f64)>> {
    if value.is_null() {
        return Ok(None);
    }
    let coords = value.as_array().ok_or_else(|| {
        PyValueError::new_err("expected a probe position triple or null")
    })?;
    let point = coords
        .iter()
        .map(|coord| {
            coord.as_f64().ok_or_else(|| {
                PyValueError::new_err("probe coordinate is not a number")
            })
        })
        .collect::<PyResult<Vec<f64>>>()?;
    if point.len() != 3 {
        return Err(PyValueError::new_err("expected a probe position triple"));
    }
    Ok(Some((point[0], point[1], point[2])))
}

fn parse_endpoint(
    py: Python<'_>,
    endpoint: &Py<PyAny>,
) -> PyResult<RemoteEndpoint> {
    let bound = endpoint.bind(py);
    let value = if let Ok(url) = bound.extract::<String>() {
        parse_url(&url)?
    } else if bound.hasattr("to_dict")? {
        let dict = bound.call_method0("to_dict")?;
        py_to_json(&dict)?
    } else {
        py_to_json(bound)?
    };
    let host = value
        .get("remote_host")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            PyValueError::new_err(
                "endpoint needs remote_host or a raydriver:// url",
            )
        })?
        .to_string();
    Ok(RemoteEndpoint {
        host,
        port: value
            .get("remote_port")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_PORT as u64) as u16,
        token: value
            .get("token")
            .and_then(Value::as_str)
            .map(str::to_string),
        tls_ca: value
            .get("tls_ca")
            .and_then(Value::as_str)
            .map(str::to_string),
        tls_hostname: value
            .get("tls_hostname")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn parse_url(url: &str) -> PyResult<Value> {
    let Some(rest) = url.strip_prefix("raydriver://") else {
        return Err(PyValueError::new_err(format!(
            "unsupported remote url {url:?}"
        )));
    };
    let (authority, token) = match rest.split_once('#') {
        Some((authority, token)) => (authority, Some(token)),
        None => (rest, None),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host,
            port.parse::<u16>().map_err(|_| {
                PyValueError::new_err(format!("invalid remote port {port:?}"))
            })?,
        ),
        None => (authority, DEFAULT_PORT),
    };
    if host.is_empty() {
        return Err(PyValueError::new_err(format!(
            "remote url has no host: {url:?}"
        )));
    }
    Ok(json!({
        "remote_host": host,
        "remote_port": port,
        "token": token.filter(|token| !token.is_empty()),
    }))
}

fn remote_endpoint_from_config(
    config: &Value,
) -> PyResult<Option<RemoteEndpoint>> {
    if let Some(url) = config.get("url").and_then(Value::as_str) {
        let endpoint = parse_url(url)?;
        return Ok(Some(RemoteEndpoint {
            host: endpoint
                .get("remote_host")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            port: endpoint
                .get("remote_port")
                .and_then(Value::as_u64)
                .unwrap_or(DEFAULT_PORT as u64) as u16,
            token: endpoint
                .get("token")
                .and_then(Value::as_str)
                .map(str::to_string),
            tls_ca: config
                .get("tls_ca")
                .and_then(Value::as_str)
                .map(str::to_string),
            tls_hostname: config
                .get("tls_hostname")
                .and_then(Value::as_str)
                .map(str::to_string),
        }));
    }
    let Some(host) = config.get("remote_host").and_then(Value::as_str) else {
        return Ok(None);
    };
    Ok(Some(RemoteEndpoint {
        host: host.to_string(),
        port: config
            .get("remote_port")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_PORT as u64) as u16,
        token: config
            .get("token")
            .and_then(Value::as_str)
            .map(str::to_string),
        tls_ca: config
            .get("tls_ca")
            .and_then(Value::as_str)
            .map(str::to_string),
        tls_hostname: config
            .get("tls_hostname")
            .and_then(Value::as_str)
            .map(str::to_string),
    }))
}

const REMOTE_KEYS: [&str; 6] = [
    "remote_host",
    "remote_port",
    "token",
    "tls_ca",
    "tls_hostname",
    "url",
];

#[gen_stub_pyfunction(module = "raygeo.driver.remote.client")]
#[gen_stub(override_return_type(
    type_repr = "typing.Union[RemoteSession, grbl.GrblSerialDriver, \
                simple.GrblSerialSimpleDriver, smoothie.SmoothieDriver, \
                octoprint.OctoPrintDriver, marlin.MarlinSerialDriver]",
    imports = (
        "typing",
        "raygeo.driver.grbl",
        "raygeo.driver.grbl.simple",
        "raygeo.driver.smoothie",
        "raygeo.driver.octoprint",
        "raygeo.driver.marlin",
    ),
))]
#[pyfunction]
pub(crate) fn create_session(
    py: Python<'_>,
    protocol_name: String,
    #[gen_stub(override_type(
        type_repr = "typing.Union[dict[builtins.str, builtins.object], config.GrblSerialConfig, config.GrblSimpleConfig, config.GrblTelnetConfig, config.GrblBluetoothConfig, config.Esp3dConfig, config.SmoothieConfig, config.MarlinConfig, config.XtoolConfig, config.OctoPrintConfig, config.M2Config]",
        imports = ("typing", "builtins", "raygeo.driver.config")
    ))]
    config: Py<PyAny>,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[dict[builtins.str, builtins.object]]",
        imports = ("typing", "builtins")
    ))]
    dialect: Option<Py<PyAny>>,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[collections.abc.Callable[[core.DriverEvent], builtins.object]]",
        imports = ("typing", "collections.abc", "raygeo.driver.core", "builtins")
    ))]
    event_callback: Option<Py<PyAny>>,
) -> PyResult<Py<PyAny>> {
    let config_value = config_to_value(py, &config)?;
    let dialect_value = dialect
        .as_ref()
        .map(|dialect| config_to_value(py, dialect))
        .transpose()?;
    let Some(endpoint) = remote_endpoint_from_config(&config_value)? else {
        let config = value_to_py(py, &config_value)?;
        let dialect = match dialect_value {
            Some(dialect) => value_to_py(py, &dialect)?,
            None => py.None(),
        };
        return PySessionFactory::default_session(
            py,
            &protocol_name,
            config,
            dialect,
            event_callback.unwrap_or_else(|| py.None()),
        );
    };
    let local_config = match config_value {
        Value::Object(mut map) => {
            for key in REMOTE_KEYS {
                map.remove(key);
            }
            Value::Object(map)
        }
        other => other,
    };
    let loop_ = running_loop(py).ok();
    let events = Arc::new(PyRemoteEvents::new(event_callback, loop_));
    let inner = CoreRemoteSession::new(
        Some(protocol_name),
        local_config,
        dialect_value,
        endpoint,
        None,
        None,
        events.clone(),
    );
    Ok(Py::new(py, RemoteSession { inner, events })?.into_any())
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.client")]
#[pyfunction]
#[pyo3(signature = (
    endpoint,
    device = None,
    session_id = None,
    config = None,
    dialect = None,
    event_callback = None,
))]
pub(crate) fn open_session(
    py: Python<'_>,
    #[gen_stub(override_type(
        type_repr = "typing.Union[dict[builtins.str, builtins.object], builtins.str]",
        imports = ("typing", "builtins")
    ))]
    endpoint: Py<PyAny>,
    device: Option<String>,
    session_id: Option<String>,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[typing.Union[dict[builtins.str, builtins.object], config.GrblSerialConfig, config.GrblSimpleConfig, config.GrblTelnetConfig, config.GrblBluetoothConfig, config.Esp3dConfig, config.SmoothieConfig, config.MarlinConfig, config.XtoolConfig, config.OctoPrintConfig, config.M2Config]]",
        imports = ("typing", "builtins", "raygeo.driver.config")
    ))]
    config: Option<Py<PyAny>>,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[dict[builtins.str, builtins.object]]",
        imports = ("typing", "builtins")
    ))]
    dialect: Option<Py<PyAny>>,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[collections.abc.Callable[[core.DriverEvent], builtins.object]]",
        imports = ("typing", "collections.abc", "raygeo.driver.core", "builtins")
    ))]
    event_callback: Option<Py<PyAny>>,
) -> PyResult<RemoteSession> {
    if device.is_none() && session_id.is_none() {
        return Err(PyValueError::new_err(
            "open_session needs a device or session_id",
        ));
    }
    let endpoint = parse_endpoint(py, &endpoint)?;
    let config = match config {
        Some(config) => config_to_value(py, &config)?,
        None => Value::Object(Default::default()),
    };
    let dialect = dialect
        .map(|dialect| config_to_value(py, &dialect))
        .transpose()?;
    let loop_ = running_loop(py).ok();
    let events = Arc::new(PyRemoteEvents::new(event_callback, loop_));
    let inner = CoreRemoteSession::new(
        None,
        config,
        dialect,
        endpoint,
        device,
        session_id,
        events.clone(),
    );
    Ok(RemoteSession { inner, events })
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.client")]
#[pyfunction]
pub(crate) fn list_devices<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(
        type_repr = "typing.Union[dict[builtins.str, builtins.object], builtins.str]",
        imports = ("typing", "builtins")
    ))]
    endpoint: Py<PyAny>,
) -> PyResult<PyAwait<'py, Vec<JsonDict>>> {
    let endpoint = parse_endpoint(py, &endpoint)?;
    spawn_future(py, async move {
        let devices =
            core::list_devices(&endpoint).await.map_err(connection)?;
        Python::attach(|py| {
            devices
                .iter()
                .map(|device| value_to_dict(py, device))
                .collect()
        })
    })
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.client")]
#[pyfunction]
pub(crate) fn list_drivers<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(
        type_repr = "typing.Union[dict[builtins.str, builtins.object], builtins.str]",
        imports = ("typing", "builtins")
    ))]
    endpoint: Py<PyAny>,
) -> PyResult<PyAwait<'py, Vec<JsonDict>>> {
    let endpoint = parse_endpoint(py, &endpoint)?;
    spawn_future(py, async move {
        let protocols =
            core::list_drivers(&endpoint).await.map_err(connection)?;
        Python::attach(|py| {
            protocols
                .iter()
                .map(|protocol| value_to_dict(py, protocol))
                .collect()
        })
    })
}

pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.client",
    "DEFAULT_PORT",
    u16
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.client",
    "REMOTE_KEYS",
    Vec<&'static str>
);
