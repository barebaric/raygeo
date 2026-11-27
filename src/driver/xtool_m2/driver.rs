//! The xTool M2 session: a JSON-RPC-style protocol over TLS
//! WebSocket (`wss://<host>:28900/websocket`).
//!
//! Requests are length-prefixed binary frames (`ba be` header) with
//! wrapping 16-bit transaction IDs; the machine answers each with a
//! code/message/data triple and pushes notification frames (state,
//! working, finished) that update the device state.  The session
//! authenticates as the machine's guest user on connect and rejects
//! devices that do not identify as an M2.
//!
//! Everything is device-API-driven — no line protocol and no
//! `ok`/`error` acks; a parked request fails when the socket drops
//! (`fail_pending`) or at its per-request deadline.

use crate::driver::core::driver::{
    info::caps, ConnectionKind, DriverInfo, DriverMaturity,
};

use crate::driver::core::driver::{SetupField, SetupKind};

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use futures_util::{SinkExt, StreamExt};
use rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error, SignatureScheme};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::sync::{oneshot, Mutex as AsyncMutex};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{HeaderValue, ORIGIN};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};
use uuid::Uuid;

use crate::driver::core::driver::{
    AsyncEvent, CancelMode, Driver, DriverError, DriverEvents,
};
use crate::driver::core::types::{
    DeviceError, DeviceState, DeviceStatus, TransportStatus, UnitSystem,
};

const DEFAULT_PORT: u16 = 28900;
const FRAME_HEADER: [u8; 2] = [0xba, 0xbe];
const JSON_PROTOCOL: u8 = 4;
const PING_TRANSACTION_ID: u64 = 65510;
const TRANSACTION_ID_WRAP: u64 = 65500;
const FIRST_MESSAGE_KEY: &str = "bWFrZWJsb2NrLXh0b29s";
const GUEST_USER_ID: &str = "mk-guest";

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type SocketWriter = futures_util::stream::SplitSink<Socket, Message>;

#[derive(Clone)]
pub struct M2Config {
    pub host: String,
    pub port: u16,
    pub request_timeout: f64,
    pub connect_timeout: f64,
    pub reconnect_delay: f64,
    pub poll_interval: f64,
    pub tls_allow_invalid_certs: bool,
}

impl M2Config {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            port: DEFAULT_PORT,
            request_timeout: 10.0,
            connect_timeout: 15.0,
            reconnect_delay: 5.0,
            poll_interval: 3.0,
            tls_allow_invalid_certs: false,
        }
    }

    fn secs(v: f64) -> Duration {
        Duration::from_secs_f64(v.max(0.0))
    }
}

pub struct XtoolM2Driver {
    config: M2Config,
    events: Arc<dyn DriverEvents>,
    state: Arc<Mutex<DeviceState>>,
    keep_running: AtomicBool,
    connection: AsyncMutex<Option<Arc<M2Connection>>>,
    connection_task: Mutex<Option<JoinHandle<()>>>,
}

/// Typed identity and capabilities of this driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "XtoolM2Driver",
    connection: ConnectionKind::Network,
    maturity: DriverMaturity::Experimental,
    caps: caps(
        false, false, false, true, false, false, true, false, false, true,
        false,
    ),
    uses_gcode: false,
    machine_space_wcs: "G53",
    supported_wcs: &[],
    config_keys: &["host", "port", "tls_allow_invalid_certs"],
    setup: SETUP,
    fingerprint: None,
};

impl XtoolM2Driver {
    pub fn new(config: M2Config, events: Arc<dyn DriverEvents>) -> Arc<Self> {
        Arc::new(Self {
            config,
            events,
            state: Arc::new(Mutex::new(DeviceState::default())),
            keep_running: AtomicBool::new(false),
            connection: AsyncMutex::new(None),
            connection_task: Mutex::new(None),
        })
    }

    async fn disconnect_tasks(&self) {
        let task = { self.connection_task.lock().unwrap().take() };
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }
        let conn = self.connection.lock().await.take();
        if let Some(conn) = conn {
            conn.close().await;
        }
    }

    fn set_connection_status(
        &self,
        status: TransportStatus,
        message: Option<&str>,
    ) {
        self.events.connection_status(status, message);
    }

    async fn connection_loop(self: Arc<Self>) {
        while self.keep_running.load(Ordering::SeqCst) {
            self.set_connection_status(TransportStatus::Connecting, None);
            match M2Connection::connect(
                &self.config,
                self.events.clone(),
                self.state.clone(),
            )
            .await
            {
                Ok(conn) => {
                    *self.connection.lock().await = Some(conn.clone());
                    self.set_connection_status(
                        TransportStatus::Connected,
                        None,
                    );
                    if let Err(err) = self.refresh_state(&conn).await {
                        log_crate::warn!("M2 initial state poll failed: {err}");
                    }
                    self.run_connected(conn).await;
                }
                Err(err) => {
                    log_crate::error!("M2 connection failed: {err}");
                    self.set_connection_status(
                        TransportStatus::Error,
                        Some(&err.to_string()),
                    );
                }
            }
            if !self.keep_running.load(Ordering::SeqCst) {
                break;
            }
            *self.connection.lock().await = None;
            self.set_connection_status(TransportStatus::Sleeping, None);
            tokio::time::sleep(M2Config::secs(self.config.reconnect_delay))
                .await;
        }
    }

    async fn run_connected(self: &Arc<Self>, conn: Arc<M2Connection>) {
        let mut interval = tokio::time::interval(M2Config::secs(
            self.config.poll_interval.max(0.05),
        ));
        loop {
            tokio::select! {
                _ = conn.closed.wait() => break,
                _ = interval.tick() => {
                    if !self.keep_running.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Err(err) = conn.ping().await {
                        log_crate::warn!("M2 heartbeat failed: {err}");
                        break;
                    }
                    if let Err(err) = self.refresh_state(&conn).await {
                        log_crate::warn!("M2 state poll failed: {err}");
                        break;
                    }
                }
            }
        }
        conn.close().await;
        let mut current = self.connection.lock().await;
        if current
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(active, &conn))
        {
            *current = None;
        }
        if self.keep_running.load(Ordering::SeqCst) {
            self.set_connection_status(
                TransportStatus::Error,
                Some("M2 WebSocket connection lost"),
            );
        }
    }

    async fn refresh_state(
        &self,
        conn: &Arc<M2Connection>,
    ) -> Result<(), DriverError> {
        let snapshot = conn
            .request(
                "POST",
                "/v1/platform/device/state/sync",
                json!({}),
                json!({}),
                M2Config::secs(self.config.request_timeout),
            )
            .await?;
        if let Some(mode) = snapshot_mode(&snapshot) {
            apply_mode(&self.state, &self.events, &mode);
        }
        if let Ok(coordinate) = conn
            .request(
                "GET",
                "/v1/project/device/coordinate",
                json!({}),
                json!({}),
                M2Config::secs(self.config.request_timeout),
            )
            .await
        {
            self.apply_coordinate(&coordinate);
        }
        Ok(())
    }

    fn apply_coordinate(&self, coordinate: &Value) {
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            let values = ["x", "y", "z"]
                .map(|key| coordinate.get(key).and_then(Value::as_f64))
                .to_vec();
            if values.iter().all(Option::is_none) {
                return;
            }
            if state.machine_pos == values {
                return;
            }
            state.machine_pos = values;
            state.clone()
        };
        self.events.state_changed(&snapshot);
    }

    async fn current_connection(
        &self,
    ) -> Result<Arc<M2Connection>, DriverError> {
        self.connection
            .lock()
            .await
            .clone()
            .ok_or_else(|| DriverError::Connection("not connected".to_string()))
    }

    async fn control(&self, action: &str) -> Result<(), DriverError> {
        self.current_connection()
            .await?
            .request(
                "POST",
                "/v1/project/device/control",
                json!({"action": action}),
                json!({}),
                M2Config::secs(self.config.request_timeout),
            )
            .await?;
        Ok(())
    }

    pub async fn start(&self) -> Result<(), DriverError> {
        self.control("START").await
    }
}

struct M2Connection {
    writer: AsyncMutex<SocketWriter>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value, DriverError>>>>,
    reader_task: Mutex<Option<JoinHandle<()>>>,
    transaction: AtomicU64,
    closed: AsyncEvent,
    events: Arc<dyn DriverEvents>,
    state: Arc<Mutex<DeviceState>>,
}

impl M2Connection {
    async fn connect(
        config: &M2Config,
        events: Arc<dyn DriverEvents>,
        state: Arc<Mutex<DeviceState>>,
    ) -> Result<Arc<Self>, DriverError> {
        let client_id = Uuid::new_v4();
        let url = format!(
            "wss://{}:{}/websocket?id={client_id}&function=instruction",
            config.host, config.port
        );
        let mut request = url
            .into_client_request()
            .map_err(|err| DriverError::Connection(err.to_string()))?;
        request
            .headers_mut()
            .insert(ORIGIN, HeaderValue::from_static("atomm://renderer"));
        let connector = if config.tls_allow_invalid_certs {
            log_crate::warn!(
                "M2 TLS certificate validation is disabled for {}",
                config.host
            );
            Some(insecure_connector()?)
        } else {
            None
        };
        let (socket, _) = timeout(
            M2Config::secs(config.connect_timeout),
            connect_async_tls_with_config(request, None, false, connector),
        )
        .await
        .map_err(|_| DriverError::Timeout("operation timed out".to_string()))?
        .map_err(|err| DriverError::Connection(err.to_string()))?;
        let (writer, reader) = socket.split();
        let connection = Arc::new(Self {
            writer: AsyncMutex::new(writer),
            pending: Mutex::new(HashMap::new()),
            reader_task: Mutex::new(None),
            transaction: AtomicU64::new(0),
            closed: AsyncEvent::new(),
            events,
            state,
        });
        let reader_connection = connection.clone();
        let reader_task = tokio::spawn(async move {
            reader_connection.read_loop(reader).await;
        });
        *connection.reader_task.lock().unwrap() = Some(reader_task);
        let connect_result = connection
            .request(
                "GET",
                "/v1/user/parity",
                json!({}),
                json!({
                    "userID": GUEST_USER_ID,
                    "userKey": FIRST_MESSAGE_KEY,
                    "timezone": "UTC",
                }),
                Duration::from_secs(10),
            )
            .await;
        if let Err(err) = connect_result {
            connection.close().await;
            return Err(err);
        }
        let identity = match connection
            .request(
                "GET",
                "/v1/platform/device/machine-info",
                json!({}),
                json!({}),
                Duration::from_secs(10),
            )
            .await
        {
            Ok(identity) => identity,
            Err(err) => {
                connection.close().await;
                return Err(err);
            }
        };
        let model = ["deviceCode", "deviceName", "name", "model"]
            .iter()
            .filter_map(|key| identity.get(key).and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        if !model.contains("m2") && !model.contains("js002") {
            connection.close().await;
            return Err(DriverError::Device(format!(
                "device at {} did not identify itself as an xTool M2",
                config.host
            )));
        }
        Ok(connection)
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        params: Value,
        data: Value,
        request_timeout: Duration,
    ) -> Result<Value, DriverError> {
        let transaction_id = self.next_transaction_id();
        let payload = json!({
            "type": "request",
            "method": method,
            "url": path,
            "params": params,
            "data": data,
            "timestamp": now_millis(),
            "transactionId": transaction_id,
        });
        let bytes = serde_json::to_vec(&payload)
            .map_err(|err| DriverError::Other(err.to_string()))?;
        let frame = encode_frame(&bytes, JSON_PROTOCOL);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().unwrap().insert(transaction_id, sender);
        let send_result = self
            .writer
            .lock()
            .await
            .send(Message::Binary(frame.into()))
            .await;
        if let Err(err) = send_result {
            self.pending.lock().unwrap().remove(&transaction_id);
            return Err(DriverError::Connection(err.to_string()));
        }
        let response = timeout(request_timeout, receiver)
            .await
            .map_err(|_| {
                self.pending.lock().unwrap().remove(&transaction_id);
                DriverError::Timeout("operation timed out".to_string())
            })?
            .map_err(|_| {
                DriverError::Connection("connection closed".into())
            })??;
        let code = response.get("code").and_then(Value::as_i64).unwrap_or(0);
        if code != 0 {
            let message = response
                .get("msg")
                .or_else(|| response.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            return Err(DriverError::Device(format!(
                "M2 {method} {path} returned code {code}: {message}"
            )));
        }
        Ok(response.get("data").cloned().unwrap_or(Value::Null))
    }

    async fn ping(&self) -> Result<(), DriverError> {
        self.request_with_id(
            "GET",
            "/v1/user/ping",
            json!({}),
            json!({}),
            Duration::from_secs(5),
            PING_TRANSACTION_ID,
        )
        .await?;
        Ok(())
    }

    async fn request_with_id(
        &self,
        method: &str,
        path: &str,
        params: Value,
        data: Value,
        request_timeout: Duration,
        transaction_id: u64,
    ) -> Result<Value, DriverError> {
        let payload = json!({
            "type": "request",
            "method": method,
            "url": path,
            "transactionId": transaction_id,
            "data": data,
            "params": params,
            "timestamp": now_millis(),
        });
        let bytes = serde_json::to_vec(&payload)
            .map_err(|err| DriverError::Other(err.to_string()))?;
        let frame = encode_frame(&bytes, JSON_PROTOCOL);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().unwrap().insert(transaction_id, sender);
        if let Err(err) = self
            .writer
            .lock()
            .await
            .send(Message::Binary(frame.into()))
            .await
        {
            self.pending.lock().unwrap().remove(&transaction_id);
            return Err(DriverError::Connection(err.to_string()));
        }
        let response = timeout(request_timeout, receiver)
            .await
            .map_err(|_| {
                self.pending.lock().unwrap().remove(&transaction_id);
                DriverError::Timeout("operation timed out".to_string())
            })?
            .map_err(|_| {
                DriverError::Connection("connection closed".into())
            })??;
        let code = response.get("code").and_then(Value::as_i64).unwrap_or(0);
        if code != 0 {
            return Err(DriverError::Device(format!(
                "M2 heartbeat returned code {code}"
            )));
        }
        Ok(response.get("data").cloned().unwrap_or(Value::Null))
    }

    fn next_transaction_id(&self) -> u64 {
        let next = self.transaction.fetch_add(1, Ordering::SeqCst) + 1;
        if next >= TRANSACTION_ID_WRAP {
            self.transaction.store(1, Ordering::SeqCst);
            1
        } else {
            next
        }
    }

    async fn read_loop(
        self: Arc<Self>,
        mut reader: futures_util::stream::SplitStream<Socket>,
    ) {
        let mut buffer = Vec::new();
        while let Some(message) = reader.next().await {
            match message {
                Ok(Message::Binary(data)) => {
                    buffer.extend_from_slice(&data);
                    let (frames, remainder) = decode_frames(&buffer);
                    buffer = remainder;
                    for (protocol, payload) in frames {
                        if protocol != JSON_PROTOCOL {
                            continue;
                        }
                        if let Ok(value) =
                            serde_json::from_slice::<Value>(&payload)
                        {
                            self.handle_message(value);
                        }
                    }
                }
                Ok(Message::Text(text)) => {
                    if let Ok(value) = serde_json::from_str::<Value>(&text) {
                        self.handle_message(value);
                    }
                }
                Ok(Message::Ping(data)) => {
                    if let Err(err) =
                        self.writer.lock().await.send(Message::Pong(data)).await
                    {
                        log_crate::debug!("M2 pong failed: {err}");
                        break;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        self.closed.set();
        let pending = std::mem::take(&mut *self.pending.lock().unwrap());
        for (_, sender) in pending {
            let _ = sender.send(Err(DriverError::Connection(
                "M2 WebSocket closed".into(),
            )));
        }
    }

    fn handle_message(&self, value: Value) {
        if value.get("type").and_then(Value::as_str) == Some("response") {
            let transaction = value
                .get("transactionId")
                .and_then(Value::as_u64)
                .or_else(|| {
                    value
                        .get("data")
                        .and_then(|data| data.get("transactionId"))
                        .and_then(Value::as_u64)
                });
            if let Some(transaction) = transaction {
                if let Some(sender) =
                    self.pending.lock().unwrap().remove(&transaction)
                {
                    let _ = sender.send(Ok(value));
                }
            }
            return;
        }
        let method = value.get("method").and_then(Value::as_str).unwrap_or("");
        let data = value.get("data").cloned().unwrap_or(Value::Null);
        if method == "REPORT" {
            if let Some(entries) = data.as_object() {
                for (url, payload) in entries {
                    self.handle_report(url, payload);
                }
                return;
            }
        }
        if let Some(url) = value.get("url").and_then(Value::as_str) {
            self.handle_report(url, &data);
        }
    }

    fn handle_report(&self, url: &str, payload: &Value) {
        if url == "/v1/platform/device/state" {
            if let Some(mode) = snapshot_mode(payload) {
                apply_mode(&self.state, &self.events, &mode);
            }
        } else if url == "/v1/platform/device/alarm"
            && payload.as_array().is_some_and(|alarms| !alarms.is_empty())
        {
            let snapshot = {
                let mut state = self.state.lock().unwrap();
                if state.status == DeviceStatus::Alarm && state.error.is_some()
                {
                    return;
                }
                state.status = DeviceStatus::Alarm;
                state.error = Some(DeviceError::new(
                    -1,
                    "M2 alarm",
                    "Check the machine status in xTool Studio.",
                ));
                state.clone()
            };
            self.events.state_changed(&snapshot);
        }
    }

    async fn close(&self) {
        self.closed.set();
        let _ = self.writer.lock().await.close().await;
        self.fail_pending();
        let task = { self.reader_task.lock().unwrap().take() };
        if let Some(mut task) = task {
            if timeout(Duration::from_secs(1), &mut task).await.is_err() {
                task.abort();
                let _ = task.await;
            }
        }
    }

    fn close_now(&self) {
        self.closed.set();
        self.fail_pending();
        if let Ok(mut task) = self.reader_task.lock() {
            if let Some(task) = task.take() {
                task.abort();
            }
        }
    }

    fn fail_pending(&self) {
        let pending = std::mem::take(&mut *self.pending.lock().unwrap());
        for (_, sender) in pending {
            let _ = sender.send(Err(DriverError::Connection(
                "M2 WebSocket closed".into(),
            )));
        }
    }
}

fn insecure_connector() -> Result<Connector, DriverError> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = Arc::new(AllowInvalidCerts(provider.clone()));
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|err| DriverError::Other(err.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    Ok(Connector::Rustls(Arc::new(config)))
}

#[derive(Debug)]
struct AllowInvalidCerts(Arc<rustls::crypto::CryptoProvider>);

impl ServerCertVerifier for AllowInvalidCerts {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            signature,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            signature,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Guard against hostile peers sending unbounded frames.
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

fn encode_frame(payload: &[u8], protocol: u8) -> Vec<u8> {
    let length = payload.len();
    let mut header = [0u8; 10];
    header[..2].copy_from_slice(&FRAME_HEADER);
    header[2] = ((length >> 16) & 0xff) as u8;
    header[3] = ((length >> 8) & 0xff) as u8;
    header[4] = (length & 0xff) as u8;
    header[5] = protocol & 0x7f;
    let payload_crc = crc16(payload);
    header[6] = (payload_crc >> 8) as u8;
    header[7] = payload_crc as u8;
    let header_crc = crc16(&header[..8]);
    header[8] = (header_crc >> 8) as u8;
    header[9] = header_crc as u8;
    let mut frame = header.to_vec();
    frame.extend_from_slice(payload);
    frame
}

fn decode_frames(buffer: &[u8]) -> (Vec<(u8, Vec<u8>)>, Vec<u8>) {
    let mut frames = Vec::new();
    let mut position = 0;
    while position + 10 <= buffer.len() {
        if buffer[position..position + 2] != FRAME_HEADER {
            position += 1;
            continue;
        }
        let length = ((buffer[position + 2] as usize) << 16)
            | ((buffer[position + 3] as usize) << 8)
            | buffer[position + 4] as usize;
        if length > MAX_FRAME_BYTES {
            position += 1;
            continue;
        }
        let end = position + 10 + length;
        if end > buffer.len() {
            break;
        }
        let header = &buffer[position..position + 10];
        let header_crc = u16::from_be_bytes([header[8], header[9]]);
        if crc16(&header[..8]) != header_crc {
            position += 1;
            continue;
        }
        let payload = &buffer[position + 10..end];
        let crc_disabled = header[5] & 0x80 != 0;
        let payload_crc = u16::from_be_bytes([header[6], header[7]]);
        if !crc_disabled && crc16(payload) != payload_crc {
            position += 1;
            continue;
        }
        frames.push((header[5] & 0x7f, payload.to_vec()));
        position = end;
    }
    (frames, buffer[position..].to_vec())
}

fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for byte in data {
        crc ^= u16::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xa001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn snapshot_mode(snapshot: &Value) -> Option<String> {
    snapshot
        .get("curMode")
        .and_then(|mode| mode.get("mode"))
        .or_else(|| snapshot.get("mode"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn status_for_mode(mode: &str) -> DeviceStatus {
    match mode {
        "Idle"
        | "WorkReady"
        | "WorkDone"
        | "finish"
        | "P_IDLE"
        | "P_READY"
        | "P_FRAME_READY"
        | "P_PROCESSING_READY"
        | "P_FINISH"
        | "P_FINISH_PROCESSING" => DeviceStatus::Idle,
        "Framing" | "P_FRAMING" => DeviceStatus::Jog,
        "WorkPlay" | "WorkDrying" | "P_PROCESSING" | "P_WORKING" => {
            DeviceStatus::Run
        }
        "WorkPause" | "WorkCancel" | "P_PAUSE" | "P_EMERGENCY_STOP" => {
            DeviceStatus::Hold
        }
        "Error" | "P_ERROR" => DeviceStatus::Alarm,
        "Sleep" | "P_SLEEP" => DeviceStatus::Sleep,
        _ => DeviceStatus::Unknown,
    }
}

fn apply_mode(
    state: &Mutex<DeviceState>,
    events: &Arc<dyn DriverEvents>,
    mode: &str,
) {
    let status = status_for_mode(mode);
    if status == DeviceStatus::Unknown {
        return;
    }
    let state = {
        let mut state = state.lock().unwrap();
        if state.status == status {
            return;
        }
        let old_status = state.status;
        state.status = status;
        state.error = if status == DeviceStatus::Alarm {
            Some(DeviceError::new(
                -1,
                "M2 reported an error",
                "Check the machine status in xTool Studio.",
            ))
        } else {
            None
        };
        if status == DeviceStatus::Idle
            && matches!(old_status, DeviceStatus::Run | DeviceStatus::Hold)
        {
            events.job_finished();
        }
        state.clone()
    };
    events.state_changed(&state);
}

/// The session's [`Driver`] impl: the single, real implementation
/// of every operation the protocol supports.
#[async_trait]
impl Driver for XtoolM2Driver {
    fn info(&self) -> &'static crate::driver::core::driver::DriverInfo {
        &INFO
    }

    fn state(&self) -> DeviceState {
        self.state.lock().unwrap().clone()
    }
    fn resource_uri(&self) -> Option<String> {
        Some(format!(
            "wss://{}:{}/websocket",
            self.config.host, self.config.port
        ))
    }
    fn is_job_running(&self) -> bool {
        matches!(self.state().status, DeviceStatus::Run | DeviceStatus::Hold)
    }
    fn abort_background_tasks_now(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        if let Ok(mut task) = self.connection_task.lock() {
            if let Some(task) = task.take() {
                task.abort();
            }
        }
        if let Ok(mut conn) = self.connection.try_lock() {
            if let Some(conn) = conn.take() {
                conn.close_now();
            }
        }
    }
    async fn connect(self: Arc<Self>) -> Result<(), DriverError> {
        self.disconnect_tasks().await;
        self.keep_running.store(true, Ordering::SeqCst);
        let core = self.clone();
        let task = tokio::spawn(async move { core.connection_loop().await });
        *self.connection_task.lock().unwrap() = Some(task);

        Ok(())
    }

    async fn cleanup(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        self.disconnect_tasks().await;
        self.set_connection_status(TransportStatus::Disconnected, None);
    }

    async fn set_hold(&self, hold: bool) -> Result<(), DriverError> {
        self.control(if hold { "PAUSE" } else { "RESUME" }).await
    }

    async fn cancel(&self, _mode: CancelMode) -> Result<(), DriverError> {
        self.control("CANCEL").await
    }

    async fn home(
        &self,
        axes: Option<Vec<String>>,
        _active_wcs: &str,
    ) -> Result<(), DriverError> {
        if axes.is_some() {
            return Err(DriverError::Unsupported(
                "this device only supports homing all axes".to_string(),
            ));
        }
        self.current_connection()
            .await?
            .request(
                "POST",
                "/v1/project/control/home",
                json!({"axis": "ALL"}),
                json!({}),
                M2Config::secs(self.config.request_timeout),
            )
            .await?;
        Ok(())
    }

    async fn move_to(
        &self,
        speed: f64,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> Result<(), DriverError> {
        if z.is_some() {
            return Err(DriverError::Unsupported(
                "this device cannot move the Z axis".to_string(),
            ));
        }
        self.current_connection()
            .await?
            .request(
                "POST",
                "/v1/project/control/absolute-move",
                json!({}),
                json!({"coor": {"x": x, "y": y}, "speed": speed}),
                M2Config::secs(self.config.request_timeout),
            )
            .await?;
        Ok(())
    }

    async fn jog(
        &self,
        speed: f64,
        deltas: &[(String, f64)],
    ) -> Result<(), DriverError> {
        let mut dx = 0.0;
        let mut dy = 0.0;
        let mut dz = 0.0;
        for (axis, distance) in deltas {
            match axis.to_ascii_uppercase().as_str() {
                "X" => dx = *distance,
                "Y" => dy = *distance,
                "Z" => dz = *distance,
                other => {
                    return Err(DriverError::Other(format!(
                        "jog axis {other} is not supported"
                    )));
                }
            }
        }
        self.current_connection()
            .await?
            .request(
                "POST",
                "/v1/project/control/relative-move",
                json!({}),
                json!({"dx": dx, "dy": dy, "dz": dz, "speed": speed}),
                M2Config::secs(self.config.request_timeout),
            )
            .await?;
        Ok(())
    }

    async fn clear_alarm(&self) -> Result<(), DriverError> {
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            if state.error.is_some() {
                state.error = None;
                Some(state.clone())
            } else {
                None
            }
        };
        if let Some(snapshot) = snapshot {
            self.events.state_changed(&snapshot);
        }
        Ok(())
    }

    async fn detect_unit_system(
        &self,
    ) -> Result<Option<UnitSystem>, DriverError> {
        Ok(Some(UnitSystem::Metric))
    }

    async fn read_settings(
        &self,
    ) -> Result<Vec<(String, String)>, DriverError> {
        log_crate::warn!("xTool machines do not expose readable settings.");
        Ok(Vec::new())
    }

    async fn read_wcs_offsets(
        &self,
    ) -> Result<std::collections::HashMap<String, (f64, f64, f64)>, DriverError>
    {
        Ok(std::collections::HashMap::new())
    }
}

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("host", SetupKind::Hostname).required(),
    SetupField::new("port", SetupKind::Port)
        .with_default("28900")
        .with_range(1.0, 65535.0),
    SetupField::new("tls_allow_invalid_certs", SetupKind::Bool)
        .with_default("false"),
];
