//! Python bindings for the remote wire codec.

use std::future::Future;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};
use serde_json::Value;

use crate::driver::core::remote::protocol as core;
use crate::python::driver::core::json::{value_to_dict, JsonDict};
use crate::python::driver::core::types::DeviceState as PyDeviceState;
use crate::python::driver::events::DriverEvent;
use crate::python::driver::pyfuture::{spawn_future, PyAwait};
use crate::python::driver::remote::bridge::{
    call_on_loop, py_to_json, running_loop,
};
use crate::python::driver::remote::events::{event_to_wire, wire_to_py_event};

pyo3::create_exception!(
    rayforge_raydriver_remote,
    ProtocolError,
    pyo3::exceptions::PyException,
    "A wire-level error, surfaced to the peer as an error frame."
);

fn protocol_error(py: Python<'_>, err: &core::ProtocolError) -> PyErr {
    let py_err = ProtocolError::new_err(err.message.clone());
    let value = py_err.value(py);
    let _ = value.setattr("code", &err.code);
    let _ = value.setattr("message", &err.message);
    py_err
}

fn resolve_max_bytes(
    py: Python<'_>,
    override_max: Option<usize>,
) -> PyResult<usize> {
    if let Some(max_bytes) = override_max {
        return Ok(max_bytes);
    }
    let module = py.import("raygeo.driver.remote.protocol")?;
    module.getattr("MAX_FRAME_BYTES")?.extract()
}

fn encode_value(
    message: &Value,
    max_bytes: usize,
) -> Result<Vec<u8>, core::ProtocolError> {
    let payload = serde_json::to_vec(message).map_err(|err| {
        core::ProtocolError::new(
            core::BAD_REQUEST,
            format!("cannot encode frame: {err}"),
        )
    })?;
    if payload.len() > max_bytes {
        return Err(core::ProtocolError::new(
            core::FRAME_TOO_LARGE,
            format!(
                "frame of {} bytes exceeds the {max_bytes}-byte limit",
                payload.len()
            ),
        ));
    }
    let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(&payload);
    Ok(frame)
}

fn decode_value(
    payload: &[u8],
    max_bytes: usize,
) -> Result<Value, core::ProtocolError> {
    if payload.len() > max_bytes {
        return Err(core::ProtocolError::new(
            core::FRAME_TOO_LARGE,
            format!(
                "frame of {} bytes exceeds the {max_bytes}-byte limit",
                payload.len()
            ),
        ));
    }
    let value: Value = serde_json::from_slice(payload).map_err(|_| {
        core::ProtocolError::new(
            core::BAD_REQUEST,
            "frame is not valid UTF-8 JSON",
        )
    })?;
    if !value.is_object() {
        return Err(core::ProtocolError::new(
            core::BAD_REQUEST,
            "frame must be a JSON object",
        ));
    }
    Ok(value)
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
#[pyo3(signature = (message, max_bytes = None))]
pub(crate) fn encode_frame(
    py: Python<'_>,
    #[gen_stub(override_type(
        type_repr = "dict[builtins.str, builtins.object]",
        imports = ("builtins",)
    ))]
    message: Py<PyAny>,
    max_bytes: Option<usize>,
) -> PyResult<Py<PyBytes>> {
    let max_bytes = resolve_max_bytes(py, max_bytes)?;
    let value = py_to_json(message.bind(py))?;
    let frame = encode_value(&value, max_bytes)
        .map_err(|err| protocol_error(py, &err))?;
    Ok(PyBytes::new(py, &frame).unbind())
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
#[pyo3(signature = (payload, max_bytes = None))]
pub(crate) fn decode_frame(
    py: Python<'_>,
    payload: Vec<u8>,
    max_bytes: Option<usize>,
) -> PyResult<JsonDict> {
    let max_bytes = resolve_max_bytes(py, max_bytes)?;
    let value = decode_value(&payload, max_bytes)
        .map_err(|err| protocol_error(py, &err))?;
    value_to_dict(py, &value)
}

#[gen_stub_pyclass]
#[pyclass(module = "raygeo.driver.remote.protocol")]
pub(crate) struct LengthPrefix;

#[gen_stub_pymethods]
#[pymethods]
impl LengthPrefix {
    #[getter]
    fn size(&self) -> usize {
        4
    }

    fn pack(&self, py: Python<'_>, value: u32) -> Py<PyBytes> {
        PyBytes::new(py, &value.to_be_bytes()).unbind()
    }
}

async fn read_exactly(
    loop_: &Py<PyAny>,
    reader: &Py<PyAny>,
    size: usize,
) -> PyResult<Py<PyAny>> {
    let future = Python::attach(|py| {
        let reader = reader.clone_ref(py);
        call_on_loop(py, loop_, move |py| {
            Ok(reader
                .bind(py)
                .call_method1("readexactly", (size,))?
                .unbind())
        })
    })?;
    future.await
}

async fn drain(writer: &Py<PyAny>, loop_: &Py<PyAny>) -> PyResult<()> {
    let future: std::pin::Pin<
        Box<dyn Future<Output = PyResult<Py<PyAny>>> + Send>,
    > = Python::attach(|py| {
        let writer = writer.clone_ref(py);
        let future = call_on_loop(py, loop_, move |py| {
            Ok(writer.bind(py).call_method0("drain")?.unbind())
        })?;
        Ok::<_, PyErr>(Box::pin(future))
    })?;
    future.await?;
    Ok(())
}

fn read_error(py: Python<'_>, err: PyErr) -> PyErr {
    let incomplete = py
        .import("asyncio")
        .and_then(|asyncio| asyncio.getattr("IncompleteReadError"));
    if let Ok(incomplete) = incomplete {
        if err.matches(py, &incomplete).unwrap_or(false) {
            return pyo3::exceptions::PyConnectionError::new_err(
                "connection closed",
            );
        }
    }
    err
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
pub(crate) fn read_frame<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(
        type_repr = "builtins.object",
        imports = ("builtins",)
    ))]
    reader: Py<PyAny>,
) -> PyResult<PyAwait<'py, JsonDict>> {
    let loop_ = running_loop(py)?;
    let max_bytes = resolve_max_bytes(py, None)?;
    spawn_future(py, async move {
        let prefix = read_exactly(&loop_, &reader, 4)
            .await
            .map_err(|err| Python::attach(|py| read_error(py, err)))?;
        let prefix: Vec<u8> = Python::attach(|py| prefix.extract(py))?;
        let size =
            u32::from_be_bytes([prefix[0], prefix[1], prefix[2], prefix[3]])
                as usize;
        if size > max_bytes {
            return Err(Python::attach(|py| {
                protocol_error(
                    py,
                    &core::ProtocolError::new(
                        core::FRAME_TOO_LARGE,
                        format!(
                            "frame of {size} bytes exceeds the {max_bytes}-byte limit"
                        ),
                    ),
                )
            }));
        }
        let payload = read_exactly(&loop_, &reader, size)
            .await
            .map_err(|err| Python::attach(|py| read_error(py, err)))?;
        let payload: Vec<u8> = Python::attach(|py| payload.extract(py))?;
        let value = decode_value(&payload, max_bytes)
            .map_err(|err| Python::attach(|py| protocol_error(py, &err)))?;
        Python::attach(|py| value_to_dict(py, &value))
    })
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
pub(crate) fn write_frame<'py>(
    py: Python<'py>,
    #[gen_stub(override_type(
        type_repr = "builtins.object",
        imports = ("builtins",)
    ))]
    writer: Py<PyAny>,
    #[gen_stub(override_type(
        type_repr = "dict[builtins.str, builtins.object]",
        imports = ("builtins",)
    ))]
    message: Py<PyAny>,
) -> PyResult<PyAwait<'py, ()>> {
    let loop_ = running_loop(py)?;
    let max_bytes = resolve_max_bytes(py, None)?;
    let frame = encode_value(&py_to_json(message.bind(py))?, max_bytes)
        .map_err(|err| protocol_error(py, &err))?;
    spawn_future(py, async move {
        Python::attach(|py| -> PyResult<()> {
            writer
                .bind(py)
                .call_method1("write", (PyBytes::new(py, &frame),))?;
            Ok(())
        })?;
        drain(&writer, &loop_).await?;
        Ok(())
    })
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
#[pyo3(signature = (msg_id, type_, payload = None))]
pub(crate) fn request(
    py: Python<'_>,
    msg_id: i64,
    type_: &str,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[dict[builtins.str, builtins.object]]",
        imports = ("typing", "builtins")
    ))]
    payload: Option<Py<PyAny>>,
) -> PyResult<JsonDict> {
    let payload = match payload {
        Some(payload) => py_to_json(payload.bind(py))?,
        None => Value::Object(Default::default()),
    };
    value_to_dict(py, &core::request(msg_id, type_, payload))
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
#[pyo3(signature = (msg_id, payload = None))]
pub(crate) fn response(
    py: Python<'_>,
    msg_id: i64,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[dict[builtins.str, builtins.object]]",
        imports = ("typing", "builtins")
    ))]
    payload: Option<Py<PyAny>>,
) -> PyResult<JsonDict> {
    let payload = match payload {
        Some(payload) => py_to_json(payload.bind(py))?,
        None => Value::Object(Default::default()),
    };
    value_to_dict(py, &core::response(msg_id, payload))
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
pub(crate) fn error_response(
    py: Python<'_>,
    msg_id: i64,
    code: &str,
    message: &str,
) -> PyResult<JsonDict> {
    value_to_dict(py, &core::error_response(msg_id, code, message))
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
pub(crate) fn event_frame(
    py: Python<'_>,
    session_id: &str,
    seq: i64,
    #[gen_stub(override_type(
        type_repr = "dict[builtins.str, builtins.object]",
        imports = ("builtins",)
    ))]
    event: Py<PyAny>,
) -> PyResult<JsonDict> {
    let event = py_to_json(event.bind(py))?;
    value_to_dict(py, &core::event_frame(session_id, seq, event))
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
pub(crate) fn welcome_payload(
    py: Python<'_>,
    raydriver_version: &str,
    auth_required: bool,
) -> PyResult<JsonDict> {
    value_to_dict(py, &core::welcome_payload(raydriver_version, auth_required))
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
pub(crate) fn encode_state(
    py: Python<'_>,
    state: PyRef<'_, PyDeviceState>,
) -> PyResult<JsonDict> {
    value_to_dict(py, &core::encode_state(&state.inner))
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
pub(crate) fn decode_state(
    py: Python<'_>,
    #[gen_stub(override_type(
        type_repr = "dict[builtins.str, builtins.object]",
        imports = ("builtins",)
    ))]
    payload: Py<PyAny>,
) -> PyResult<PyDeviceState> {
    let value = py_to_json(payload.bind(py))?;
    let state =
        core::decode_state(&value).map_err(|err| protocol_error(py, &err))?;
    Ok(PyDeviceState::new(state))
}

#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[pyfunction]
pub(crate) fn encode_event(
    py: Python<'_>,
    event: PyRef<'_, DriverEvent>,
) -> PyResult<JsonDict> {
    value_to_dict(py, &event_to_wire(&event).to_json())
}

/// Decodes an event frame payload; ``None`` when it is not an event.
#[gen_stub_pyfunction(module = "raygeo.driver.remote.protocol")]
#[gen_stub(override_return_type(
    type_repr = "typing.Optional[core.DriverEvent]",
    imports = ("typing", "raygeo.driver.core")
))]
#[pyfunction]
pub(crate) fn decode_event(
    py: Python<'_>,
    #[gen_stub(override_type(
        type_repr = "dict[builtins.str, builtins.object]",
        imports = ("builtins",)
    ))]
    payload: Py<PyAny>,
) -> PyResult<Option<Py<PyAny>>> {
    let value = py_to_json(payload.bind(py))?;
    let Some(event) = core::WireEvent::from_json(&value) else {
        return Ok(None);
    };
    wire_to_py_event(py, &event)
}

pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.protocol",
    "PROTOCOL_VERSION",
    i64,
    core::PROTOCOL_VERSION
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.protocol",
    "MAX_FRAME_BYTES",
    usize,
    core::MAX_FRAME_BYTES
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.protocol",
    "DEFAULT_KEEPALIVE_INTERVAL",
    f64,
    core::DEFAULT_KEEPALIVE_INTERVAL
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.protocol",
    "DEFAULT_KEEPALIVE_MISSES",
    u32,
    core::DEFAULT_KEEPALIVE_MISSES
);
macro_rules! protocol_string {
    ($name:literal, $value:expr) => {
        pyo3_stub_gen::module_variable!(
            "raygeo.driver.remote.protocol",
            $name,
            &'static str,
            $value
        );
    };
}
protocol_string!("AUTH", core::AUTH);
protocol_string!("PING", core::PING);
protocol_string!("PONG", core::PONG);
protocol_string!("DEVICE_LIST", core::DEVICE_LIST);
protocol_string!("DRIVER_LIST", core::DRIVER_LIST);
protocol_string!("SESSION_LIST", core::SESSION_LIST);
protocol_string!("SESSION_OPEN", core::SESSION_OPEN);
protocol_string!("SESSION_ATTACH", core::SESSION_ATTACH);
protocol_string!("SESSION_DETACH", core::SESSION_DETACH);
protocol_string!("SESSION_CLOSE", core::SESSION_CLOSE);
protocol_string!("SESSION_COMMAND", core::SESSION_COMMAND);
protocol_string!("SESSION_PROPERTY", core::SESSION_PROPERTY);
protocol_string!("CLIENT_LIST", core::CLIENT_LIST);
protocol_string!("WELCOME", core::WELCOME);
protocol_string!("EVENT", core::EVENT);
protocol_string!("BAD_REQUEST", core::BAD_REQUEST);
protocol_string!("BAD_VERSION", core::BAD_VERSION);
protocol_string!("AUTH_FAILED", core::AUTH_FAILED);
protocol_string!("UNKNOWN_TYPE", core::UNKNOWN_TYPE);
protocol_string!("UNKNOWN_SESSION", core::UNKNOWN_SESSION);
protocol_string!("UNKNOWN_DEVICE", core::UNKNOWN_DEVICE);
protocol_string!("UNKNOWN_METHOD", core::UNKNOWN_METHOD);
protocol_string!("UNKNOWN_PROPERTY", core::UNKNOWN_PROPERTY);
protocol_string!("LEASE_TAKEN", core::LEASE_TAKEN);
protocol_string!("LEASE_HELD", core::LEASE_HELD);
protocol_string!("JOB_RUNNING", core::JOB_RUNNING);
protocol_string!("FRAME_TOO_LARGE", core::FRAME_TOO_LARGE);
protocol_string!("INTERNAL", core::INTERNAL);
protocol_string!("KIND_PROGRESS", core::KIND_PROGRESS);
protocol_string!("KIND_JOB_ERROR", core::KIND_JOB_ERROR);
protocol_string!("KIND_LEASE", core::KIND_LEASE);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.protocol",
    "ProtocolError",
    pyo3::types::PyType
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.protocol",
    "LENGTH_PREFIX",
    LengthPrefix
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.protocol",
    "REQUEST_TYPES",
    Vec<&'static str>
);
pyo3_stub_gen::module_variable!(
    "raygeo.driver.remote.protocol",
    "DRIVERS",
    Vec<&'static str>
);
