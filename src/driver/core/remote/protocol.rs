//! Wire protocol for remote sessions (v1).
//!
//! Length-prefixed JSON frames over TCP (optionally TLS): a 4-byte
//! big-endian payload length followed by one UTF-8 JSON object. The
//! envelope is a stable, versioned wire format, so third parties can
//! drive daemons without this library.

use std::collections::HashMap;
use std::fmt;

use base64::Engine as _;
use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::driver::core::types::{DeviceError, DeviceState, DeviceStatus, Pos};

pub const PROTOCOL_VERSION: i64 = 1;
pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_KEEPALIVE_INTERVAL: f64 = 2.0;
pub const DEFAULT_KEEPALIVE_MISSES: u32 = 3;

pub const AUTH: &str = "auth";
pub const PING: &str = "ping";
pub const PONG: &str = "pong";
pub const DEVICE_LIST: &str = "device/list";
pub const DRIVER_LIST: &str = "driver/list";
pub const SESSION_LIST: &str = "session/list";
pub const SESSION_OPEN: &str = "session/open";
pub const SESSION_ATTACH: &str = "session/attach";
pub const SESSION_DETACH: &str = "session/detach";
pub const SESSION_CLOSE: &str = "session/close";
pub const SESSION_COMMAND: &str = "session/command";
pub const SESSION_PROPERTY: &str = "session/property";
pub const CLIENT_LIST: &str = "client/list";
pub const WELCOME: &str = "welcome";
pub const EVENT: &str = "event";

pub const BAD_REQUEST: &str = "bad_request";
pub const BAD_VERSION: &str = "bad_version";
pub const AUTH_FAILED: &str = "auth_failed";
pub const UNKNOWN_TYPE: &str = "unknown_type";
pub const UNKNOWN_SESSION: &str = "unknown_session";
pub const UNKNOWN_DEVICE: &str = "unknown_device";
pub const UNKNOWN_METHOD: &str = "unknown_method";
pub const UNKNOWN_PROPERTY: &str = "unknown_property";
pub const LEASE_TAKEN: &str = "lease_taken";
pub const LEASE_HELD: &str = "lease_held";
pub const JOB_RUNNING: &str = "job_running";
pub const FRAME_TOO_LARGE: &str = "frame_too_large";
pub const INTERNAL: &str = "internal";

pub const KIND_PROGRESS: &str = "progress";
pub const KIND_JOB_ERROR: &str = "job_error";
pub const KIND_LEASE: &str = "lease";

/// A wire-level error, surfaced to the peer as an error frame.
#[derive(Debug, Clone)]
pub struct ProtocolError {
    pub code: String,
    pub message: String,
    pub kind: Option<&'static str>,
}

impl ProtocolError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            kind: None,
        }
    }

    pub fn with_kind(mut self, kind: &'static str) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn payload(&self) -> Value {
        let mut payload = json!({
            "code": self.code,
            "message": self.message,
        });
        if let Some(kind) = self.kind {
            payload["kind"] = json!(kind);
        }
        payload
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProtocolError {}

/// A transport or wire failure while reading a frame.
#[derive(Debug)]
pub enum WireError {
    Connection(String),
    Protocol(ProtocolError),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection(message) => write!(f, "{message}"),
            Self::Protocol(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for WireError {}

impl From<ProtocolError> for WireError {
    fn from(err: ProtocolError) -> Self {
        Self::Protocol(err)
    }
}

/// Serialize *message* as one length-prefixed JSON frame.
pub fn encode_frame(message: &Value) -> Result<Vec<u8>, ProtocolError> {
    let payload = serde_json::to_vec(message).map_err(|err| {
        ProtocolError::new(BAD_REQUEST, format!("cannot encode frame: {err}"))
    })?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::new(
            FRAME_TOO_LARGE,
            format!(
                "frame of {} bytes exceeds the {MAX_FRAME_BYTES}-byte limit",
                payload.len()
            ),
        ));
    }
    let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// Read one frame, raising on truncation, oversize or bad JSON.
pub async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Value, WireError> {
    let mut prefix = [0u8; 4];
    reader.read_exact(&mut prefix).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            WireError::Connection("connection closed".to_string())
        } else {
            WireError::Connection(format!("read failed: {err}"))
        }
    })?;
    let size = u32::from_be_bytes(prefix) as usize;
    if size > MAX_FRAME_BYTES {
        return Err(WireError::Protocol(ProtocolError::new(
            FRAME_TOO_LARGE,
            format!(
                "frame of {size} bytes exceeds the {MAX_FRAME_BYTES}-byte limit"
            ),
        )));
    }
    let mut payload = vec![0u8; size];
    reader.read_exact(&mut payload).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            WireError::Connection("connection closed mid-frame".to_string())
        } else {
            WireError::Connection(format!("read failed: {err}"))
        }
    })?;
    let message: Value = serde_json::from_slice(&payload).map_err(|_| {
        WireError::Protocol(ProtocolError::new(
            BAD_REQUEST,
            "frame is not valid UTF-8 JSON",
        ))
    })?;
    if !message.is_object() {
        return Err(WireError::Protocol(ProtocolError::new(
            BAD_REQUEST,
            "frame must be a JSON object",
        )));
    }
    Ok(message)
}

/// Write one frame and drain the transport.
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &Value,
) -> Result<(), WireError> {
    let frame = encode_frame(message)?;
    writer
        .write_all(&frame)
        .await
        .map_err(|err| WireError::Connection(format!("write failed: {err}")))?;
    writer
        .flush()
        .await
        .map_err(|err| WireError::Connection(format!("flush failed: {err}")))
}

pub fn request(id: i64, type_: &str, payload: Value) -> Value {
    json!({"id": id, "type": type_, "payload": payload})
}

pub fn response(id: i64, payload: Value) -> Value {
    json!({"id": id, "ok": true, "payload": payload})
}

pub fn error_response(id: i64, code: &str, message: &str) -> Value {
    error_response_with_kind(id, code, message, None)
}

pub fn error_response_with_kind(
    id: i64,
    code: &str,
    message: &str,
    kind: Option<&str>,
) -> Value {
    let mut error = json!({"code": code, "message": message});
    if let Some(kind) = kind {
        error["kind"] = json!(kind);
    }
    json!({
        "id": id,
        "ok": false,
        "error": error,
    })
}

pub fn event_frame(session_id: &str, seq: i64, event: Value) -> Value {
    json!({
        "type": EVENT,
        "payload": {"session": session_id, "seq": seq, "event": event},
    })
}

pub fn welcome_payload(version: &str, auth_required: bool) -> Value {
    json!({
        "protocol": PROTOCOL_VERSION,
        "raydriver": version,
        "auth_required": auth_required,
    })
}

fn encode_pos(pos: &Pos) -> Value {
    Value::Array(
        pos.iter()
            .map(|value| match value {
                Some(v) => json!(v),
                None => Value::Null,
            })
            .collect(),
    )
}

fn decode_pos(value: Option<&Value>) -> Pos {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_f64())
                .collect::<Vec<Option<f64>>>()
        })
        .unwrap_or_else(|| vec![None, None, None])
}

/// The wire form of a device state (status, error, positions).
pub fn encode_state(state: &DeviceState) -> Value {
    let error = state.error.as_ref().map(|error| {
        json!({
            "code": error.code,
            "title": error.title,
            "description": error.description,
        })
    });
    json!({
        "status": state.status.name().to_ascii_uppercase(),
        "error": error,
        "machine_pos": encode_pos(&state.machine_pos),
        "work_pos": encode_pos(&state.work_pos),
        "wco": encode_pos(&state.wco),
        "feed_rate": state.feed_rate,
        "spindle_speed": state.spindle_speed,
        "buffer_available": state.buffer_available,
        "buffer_rx_available": state.buffer_rx_available,
    })
}

/// Rebuild a [`DeviceState`] from its wire form.
pub fn decode_state(payload: &Value) -> Result<DeviceState, ProtocolError> {
    let status_name = payload
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProtocolError::new(BAD_REQUEST, "state has no status")
        })?;
    let mut state = DeviceState::default();
    let status = DeviceStatus::from_name(status_name);
    if status == DeviceStatus::Unknown
        && !status_name.eq_ignore_ascii_case("unknown")
    {
        return Err(ProtocolError::new(
            BAD_REQUEST,
            format!("unknown device status {status_name:?}"),
        ));
    }
    state.status = status;
    if let Some(error) = payload.get("error").filter(|v| !v.is_null()) {
        state.error = Some(DeviceError {
            code: error.get("code").and_then(Value::as_i64).unwrap_or(0) as i32,
            title: error
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            description: error
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        });
    }
    state.machine_pos = decode_pos(payload.get("machine_pos"));
    state.work_pos = decode_pos(payload.get("work_pos"));
    state.wco = decode_pos(payload.get("wco"));
    state.feed_rate = payload.get("feed_rate").and_then(Value::as_i64);
    state.spindle_speed = payload.get("spindle_speed").and_then(Value::as_i64);
    state.buffer_available =
        payload.get("buffer_available").and_then(Value::as_i64);
    state.buffer_rx_available =
        payload.get("buffer_rx_available").and_then(Value::as_i64);
    Ok(state)
}

/// One session event or daemon-synthesized event in wire form.
#[derive(Debug, Clone)]
pub enum WireEvent {
    State(DeviceState),
    ConnectionStatus {
        status: String,
        message: Option<String>,
    },
    CommandStatus {
        status: String,
        message: Option<String>,
    },
    JobFinished,
    ProbeStatus {
        message: String,
    },
    WcsUpdated {
        offsets: HashMap<String, (f64, f64, f64)>,
    },
    ConfigChanged {
        key: String,
        value: i64,
    },
    RawRx {
        line: String,
    },
    RawTx {
        line: String,
    },
    RawRxBytes {
        data: Vec<u8>,
    },
    RawTxBytes {
        data: Vec<u8>,
    },
    Progress {
        op: i64,
    },
    JobError {
        message: String,
    },
    Lease {
        holder: Option<String>,
        reason: String,
    },
}

impl WireEvent {
    pub fn to_json(&self) -> Value {
        match self {
            Self::State(state) => {
                json!({"kind": "state", "state": encode_state(state)})
            }
            Self::ConnectionStatus { status, message } => json!({
                "kind": "connection_status",
                "status": status,
                "message": message,
            }),
            Self::CommandStatus { status, message } => json!({
                "kind": "command_status",
                "status": status,
                "message": message,
            }),
            Self::JobFinished => json!({"kind": "job_finished"}),
            Self::ProbeStatus { message } => {
                json!({"kind": "probe_status", "message": message})
            }
            Self::WcsUpdated { offsets } => {
                let offsets: serde_json::Map<String, Value> = offsets
                    .iter()
                    .map(|(key, value)| {
                        (key.clone(), json!([value.0, value.1, value.2]))
                    })
                    .collect();
                json!({"kind": "wcs_updated", "offsets": offsets})
            }
            Self::ConfigChanged { key, value } => {
                json!({"kind": "config_changed", "key": key, "value": value})
            }
            Self::RawRx { line } => {
                json!({"kind": "raw_rx", "line": line})
            }
            Self::RawTx { line } => {
                json!({"kind": "raw_tx", "line": line})
            }
            Self::RawRxBytes { data } => json!({
                "kind": "raw_rx_bytes",
                "data": base64::engine::general_purpose::STANDARD
                    .encode(data),
            }),
            Self::RawTxBytes { data } => json!({
                "kind": "raw_tx_bytes",
                "data": base64::engine::general_purpose::STANDARD
                    .encode(data),
            }),
            Self::Progress { op } => {
                json!({"kind": KIND_PROGRESS, "op": op})
            }
            Self::JobError { message } => {
                json!({"kind": KIND_JOB_ERROR, "message": message})
            }
            Self::Lease { holder, reason } => json!({
                "kind": KIND_LEASE,
                "holder": holder,
                "reason": reason,
            }),
        }
    }

    /// Decode a wire event. Unknown kinds yield `None` for forward
    /// compatibility.
    pub fn from_json(payload: &Value) -> Option<Self> {
        match payload.get("kind").and_then(Value::as_str)? {
            "state" => {
                let state = decode_state(payload.get("state")?).ok()?;
                Some(Self::State(state))
            }
            "connection_status" => Some(Self::ConnectionStatus {
                status: payload
                    .get("status")
                    .and_then(Value::as_str)?
                    .to_string(),
                message: payload
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            }),
            "command_status" => Some(Self::CommandStatus {
                status: payload
                    .get("status")
                    .and_then(Value::as_str)?
                    .to_string(),
                message: payload
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            }),
            "job_finished" => Some(Self::JobFinished),
            "probe_status" => Some(Self::ProbeStatus {
                message: payload
                    .get("message")
                    .and_then(Value::as_str)?
                    .to_string(),
            }),
            "wcs_updated" => {
                let mut offsets = HashMap::new();
                if let Some(map) =
                    payload.get("offsets").and_then(Value::as_object)
                {
                    for (key, value) in map {
                        let items = value.as_array()?;
                        offsets.insert(
                            key.clone(),
                            (
                                items.first()?.as_f64()?,
                                items.get(1)?.as_f64()?,
                                items.get(2)?.as_f64()?,
                            ),
                        );
                    }
                }
                Some(Self::WcsUpdated { offsets })
            }
            "config_changed" => Some(Self::ConfigChanged {
                key: payload.get("key")?.as_str()?.to_string(),
                value: payload.get("value")?.as_i64()?,
            }),
            "raw_rx" => Some(Self::RawRx {
                line: payload.get("line")?.as_str()?.to_string(),
            }),
            "raw_tx" => Some(Self::RawTx {
                line: payload.get("line")?.as_str()?.to_string(),
            }),
            "raw_rx_bytes" => Some(Self::RawRxBytes {
                data: base64::engine::general_purpose::STANDARD
                    .decode(payload.get("data")?.as_str()?)
                    .ok()?,
            }),
            "raw_tx_bytes" => Some(Self::RawTxBytes {
                data: base64::engine::general_purpose::STANDARD
                    .decode(payload.get("data")?.as_str()?)
                    .ok()?,
            }),
            KIND_PROGRESS => Some(Self::Progress {
                op: payload.get("op").and_then(Value::as_i64).unwrap_or(0),
            }),
            KIND_JOB_ERROR => Some(Self::JobError {
                message: payload
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }),
            KIND_LEASE => Some(Self::Lease {
                holder: payload
                    .get("holder")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                reason: payload
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }),
            _ => None,
        }
    }
}
