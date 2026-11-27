//! The OctoPrint session: job submission and live printer state
//! over the server's REST API and SockJS WebSocket.
//!
//! OctoPrint is a print-server API, not a device protocol: commands
//! and jobs are HTTP requests authenticated with an API key, and
//! state arrives as push messages over a SockJS WebSocket (with a
//! REST polling fallback for servers where the socket is not
//! reachable). The session maps OctoPrint's printer-state flags
//! onto the generic device state and completes jobs from the
//! server's print events.

use crate::driver::core::driver::{
    info::{caps, DEFAULT_SUPPORTED_WCS},
    ConnectionKind, DriverInfo, DriverMaturity,
};

use crate::driver::core::driver::{SetupField, SetupKind};

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use serde_json::{json, Value};
use tokio::task::JoinHandle;

use crate::driver::core::driver::{
    CancelMode, Driver, DriverError, DriverEvents,
};
use crate::driver::core::http;
use crate::driver::core::types::{
    DeviceError, DeviceState, DeviceStatus, TransportStatus, UnitSystem,
};
use crate::driver::core::ws::WsClient;

/// Default HTTP (API) port of an OctoPrint server.
pub const DEFAULT_HTTP_PORT: u16 = 80;

/// Interval between REST state polls used when the SockJS
/// WebSocket is not reachable.
const POLL_INTERVAL: f64 = 2.0;
/// Delay between connection attempts after a lost connection.
const RECONNECT_DELAY: f64 = 5.0;
/// WebSocket ping interval offered in the handshake.
const WS_PING_INTERVAL: f64 = 30.0;

/// Filename jobs are uploaded under.
const JOB_FILENAME: &str = "rayforge.gcode";

fn secs(seconds: f64) -> Duration {
    Duration::from_secs_f64(seconds.max(0.01))
}

/// Connection settings for an OctoPrint server.
pub struct OctoPrintConfig {
    pub host: String,
    pub port: u16,
    /// URL prefix the server is mounted under ("" for the root,
    /// no trailing slash; e.g. "/octoprint").
    pub path_prefix: String,
    pub api_key: String,
    pub request_timeout: f64,
    pub connect_timeout: f64,
    pub poll_interval: f64,
    pub reconnect_delay: f64,
    pub ws_ping_interval: f64,
}

impl OctoPrintConfig {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            port: DEFAULT_HTTP_PORT,
            path_prefix: String::new(),
            api_key: String::new(),
            request_timeout: 30.0,
            connect_timeout: 10.0,
            poll_interval: POLL_INTERVAL,
            reconnect_delay: RECONNECT_DELAY,
            ws_ping_interval: WS_PING_INTERVAL,
        }
    }

    fn secs(&self, seconds: f64) -> Duration {
        secs(seconds)
    }

    /// The `http://host:port/prefix` base for API paths.
    fn base(&self) -> String {
        format!("http://{}:{}{}", self.host, self.port, self.path_prefix)
    }
}

/// Normalizes a URL path prefix: ``/`` yields an empty prefix so
/// URLs stay clean (``http://host:port/api/…``); a non-root prefix
/// like ``/octoprint`` is kept as-is (no trailing slash; request
/// paths already start with ``/``).
pub fn normalize_path_prefix(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut prefix = trimmed.to_string();
    while prefix.len() > 1 && prefix.ends_with('/') {
        prefix.pop();
    }
    if prefix == "/" {
        String::new()
    } else {
        prefix
    }
}

struct JobState {
    active: bool,
    done: Option<Result<(), String>>,
}

/// The OctoPrint session core.
pub struct OctoPrintDriver {
    config: OctoPrintConfig,
    events: Arc<dyn DriverEvents>,
    state: Mutex<DeviceState>,
    keep_running: AtomicBool,
    job_running: AtomicBool,
    /// Set when the current job reaches a terminal event.
    job_done: Mutex<JobState>,
    /// The session key + user name from the passive login, used to
    /// authenticate the WebSocket.
    session_auth: Mutex<Option<(String, String)>>,
    connection_task: Mutex<Option<JoinHandle<()>>>,
    background_tasks: Mutex<Vec<JoinHandle<()>>>,
}

/// Typed identity and capabilities of this driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "OctoPrintDriver",
    connection: ConnectionKind::Network,
    maturity: DriverMaturity::Untested,
    caps: caps(
        false, true, false, true, true, false, true, true, false, true, false,
    ),
    uses_gcode: true,
    machine_space_wcs: "G53",
    supported_wcs: DEFAULT_SUPPORTED_WCS,
    config_keys: &["host", "port", "path", "api_key"],
    setup: SETUP,
    fingerprint: None,
};

impl OctoPrintDriver {
    pub async fn jog_absolute(
        &self,
        x: f64,
        y: f64,
        z: Option<f64>,
        speed: Option<f64>,
    ) -> Result<(), DriverError> {
        let mut params = json!({
            "command": "jog",
            "x": x,
            "y": y,
            "absolute": true,
        });
        if let Some(z) = z {
            params["z"] = json!(z);
        }
        if let Some(speed) = speed {
            params["speed"] = json!(speed);
        }
        self.api_request("POST", "/api/printer/printhead", Some(params))
            .await?;
        Ok(())
    }
    pub fn new(
        config: OctoPrintConfig,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            events,
            state: Mutex::new(DeviceState::default()),
            keep_running: AtomicBool::new(false),
            job_running: AtomicBool::new(false),
            job_done: Mutex::new(JobState {
                active: false,
                done: None,
            }),
            session_auth: Mutex::new(None),
            connection_task: Mutex::new(None),
            background_tasks: Mutex::new(Vec::new()),
        })
    }

    // ---------------------------------------------------------------
    // Accessors
    // ---------------------------------------------------------------

    fn api_key(&self) -> String {
        self.config.api_key.clone()
    }

    fn update_connection_status(
        &self,
        status: TransportStatus,
        message: Option<&str>,
    ) {
        self.events.connection_status(status, message);
    }

    fn set_status(&self, status: DeviceStatus) {
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            if state.status == status {
                return;
            }
            state.status = status;
            state.clone()
        };
        // Emit outside the lock: event delivery attaches to the
        // Python GIL and must never run while holding a lock that
        // Python-side code could take.
        self.events.state_changed(&snapshot);
    }

    fn set_error(&self, error: Option<DeviceError>) {
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            state.error = error;
            state.clone()
        };
        self.events.state_changed(&snapshot);
    }

    // ---------------------------------------------------------------
    // HTTP API
    // ---------------------------------------------------------------

    fn api_path(&self, path: &str) -> String {
        format!("{}{}", self.config.path_prefix, path)
    }

    async fn api_request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Option<Value>, DriverError> {
        self.api_request_allowing(method, path, body, &[]).await
    }

    async fn api_request_allowing(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        allowed_statuses: &[u16],
    ) -> Result<Option<Value>, DriverError> {
        let timeout = self.config.secs(self.config.request_timeout);
        let headers = [("X-Api-Key", self.api_key())];
        let headers: Vec<(&str, &str)> =
            headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let response = match (method, body) {
            ("GET", _) => {
                http::get_with_headers(
                    &self.config.host,
                    self.config.port,
                    &self.api_path(path),
                    &headers,
                    timeout,
                )
                .await?
            }
            (_, Some(payload)) => {
                http::post_with_headers(
                    &self.config.host,
                    self.config.port,
                    &self.api_path(path),
                    "application/json",
                    payload.to_string().as_bytes(),
                    &headers,
                    timeout,
                )
                .await?
            }
            _ => {
                return Err(DriverError::Device(format!(
                    "unsupported request: {method} {path}"
                )))
            }
        };
        if response.status == 403 {
            return Err(DriverError::Device(
                "authentication failed: the API key is invalid or \
                 expired"
                    .to_string(),
            ));
        }
        if !(200..300).contains(&response.status)
            && !allowed_statuses.contains(&response.status)
        {
            return Err(DriverError::Device(format!(
                "unexpected HTTP status {} from {path}",
                response.status
            )));
        }
        if response.body.is_empty() {
            return Ok(None);
        }
        let text = response.text();
        match serde_json::from_str::<Value>(&text) {
            Ok(value) => Ok(Some(value)),
            Err(_) => {
                // Non-JSON bodies (204-style replies with padding)
                // carry no data.
                Ok(None)
            }
        }
    }

    /// Fetches the census payloads: the parsed ``/api/version``
    /// and ``/api/printer`` responses, as JSON-serializable dicts.
    pub async fn probe_info(&self) -> Result<(Value, Value), DriverError> {
        let version = self
            .api_request("GET", "/api/version", None)
            .await?
            .unwrap_or(Value::Null);
        let printer = self
            .api_request("GET", "/api/printer?exclude=temperature,sd", None)
            .await?
            .unwrap_or(Value::Null);
        Ok((version, printer))
    }

    async fn verify_connection(&self) -> Result<(), DriverError> {
        let login = self
            .api_request("POST", "/api/login", Some(json!({"passive": true})))
            .await?
            .ok_or_else(|| {
                DriverError::Device(
                    "OctoPrint returned no login data".to_string(),
                )
            })?;
        let session_key = login
            .get("session")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let user_name = login
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        *self.session_auth.lock().unwrap() = Some((session_key, user_name));
        Ok(())
    }

    // ---------------------------------------------------------------
    // Connect / disconnect
    // ---------------------------------------------------------------

    async fn abort_background_tasks(&self) {
        let task = self.connection_task.lock().unwrap().take();
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }
        let tasks: Vec<JoinHandle<()>> =
            std::mem::take(&mut *self.background_tasks.lock().unwrap());
        for task in tasks {
            task.abort();
            let _ = task.await;
        }
    }

    async fn connection_loop(self: Arc<Self>) {
        loop {
            if !self.keep_running.load(Ordering::SeqCst) {
                break;
            }
            self.update_connection_status(TransportStatus::Connecting, None);
            match self.verify_connection().await {
                Ok(()) => {
                    self.update_connection_status(
                        TransportStatus::Connected,
                        None,
                    );
                    // The WebSocket carries push updates; when it
                    // cannot be established (proxy, path, older
                    // server), fall back to REST polling.
                    let core = self.clone();
                    match core.run_websocket().await {
                        Ok(()) => {}
                        Err(err) => {
                            log_crate::info!(
                                "WebSocket unavailable, using polling: \
                                 {err}"
                            );
                            let core = self.clone();
                            if let Err(err) = core.run_polling().await {
                                log_crate::error!("Polling error: {err}");
                            }
                        }
                    }
                }
                Err(err) => {
                    self.update_connection_status(
                        TransportStatus::Error,
                        Some(&err.to_string()),
                    );
                }
            }
            if !self.keep_running.load(Ordering::SeqCst) {
                break;
            }
            self.update_connection_status(TransportStatus::Sleeping, None);
            tokio::time::sleep(self.config.secs(self.config.reconnect_delay))
                .await;
        }
    }

    async fn run_websocket(self: Arc<Self>) -> Result<(), DriverError> {
        let ws_path = format!("{}/sockjs/websocket", self.config.path_prefix);
        let mut ws = WsClient::connect_path(
            &self.config.host,
            self.config.port,
            &ws_path,
            Some(&self.config.base()),
            self.config.secs(self.config.connect_timeout),
        )
        .await?;

        // ``/sockjs/websocket`` is SockJS's raw WebSocket
        // transport: it sends no ``o`` open frame and uses no
        // framing, so the auth message goes out right after
        // connecting. Without it OctoPrint withholds status updates
        // and the driver stays at UNKNOWN.
        let auth = self.session_auth.lock().unwrap().clone();
        if let Some((session_key, user_name)) = auth {
            if !session_key.is_empty() && !user_name.is_empty() {
                let message = json!({
                    "auth": format!("{user_name}:{session_key}")
                });
                ws.send_text(&message.to_string()).await?;
            }
        }

        loop {
            if !self.keep_running.load(Ordering::SeqCst) {
                return Ok(());
            }
            let frame = ws.receive().await?;
            if frame.starts_with("c[") {
                return Err(DriverError::Connection(
                    "server closed the WebSocket connection".to_string(),
                ));
            }
            for message in sockjs_payloads(&frame) {
                self.process_push_message(&message);
            }
        }
    }

    async fn run_polling(self: Arc<Self>) -> Result<(), DriverError> {
        while self.keep_running.load(Ordering::SeqCst) {
            match self
                .api_request("GET", "/api/printer?exclude=temperature,sd", None)
                .await
            {
                Ok(Some(data)) => {
                    if let Some(state) = data.get("state") {
                        self.update_state_from_octoprint(state);
                    }
                }
                Ok(None) => {}
                Err(err) => match err {
                    DriverError::Device(message)
                    | DriverError::Connection(message) => {
                        self.update_connection_status(
                            TransportStatus::Error,
                            Some(&message),
                        );
                    }
                    _ => {}
                },
            }
            if self.job_running.load(Ordering::SeqCst) {
                self.poll_job_state().await;
            }
            tokio::time::sleep(self.config.secs(self.config.poll_interval))
                .await;
        }
        Ok(())
    }

    async fn poll_job_state(&self) {
        let Ok(Some(job)) = self.api_request("GET", "/api/job", None).await
        else {
            return;
        };
        if let Some(progress) = job.get("progress") {
            if let Some(completion) = progress.get("completion") {
                self.report_progress(completion);
            }
        }
        let job_state =
            job.get("state").and_then(Value::as_str).unwrap_or_default();
        if !matches!(
            job_state,
            "Printing" | "Pausing" | "Paused" | "Cancelling" | "Starting"
        ) {
            self.finish_job(job_state.contains("Error"));
        }
    }

    // ---------------------------------------------------------------
    // Push message processing
    // ---------------------------------------------------------------

    fn process_push_message(&self, payload: &Value) {
        if let Some(current) = payload.get("current") {
            self.handle_current_update(current);
        } else if let Some(history) = payload.get("history") {
            self.handle_current_update(history);
        } else if let Some(event) = payload.get("event") {
            self.handle_event(event);
        }
    }

    fn handle_current_update(&self, data: &Value) {
        if let Some(state) = data.get("state") {
            self.update_state_from_octoprint(state);
        }
        if self.job_running.load(Ordering::SeqCst) {
            if let Some(progress) = data.get("progress") {
                if let Some(completion) = progress.get("completion") {
                    self.report_progress(completion);
                }
            }
        }
    }

    fn handle_event(&self, event: &Value) {
        let event_type =
            event.get("type").and_then(Value::as_str).unwrap_or("");
        match event_type {
            "PrintDone" => self.finish_job(true),
            "PrintFailed" => self.finish_job(false),
            "PrintCancelled" => self.finish_job(true),
            "Connected" => {
                log_crate::info!("OctoPrint printer connected");
            }
            "Disconnected" => {
                self.set_status(DeviceStatus::Unknown);
            }
            _ => {}
        }
    }

    fn update_state_from_octoprint(&self, state_data: &Value) {
        let flags = state_data.get("flags").cloned().unwrap_or(json!({}));
        let text = state_data
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("Unknown");

        let flag = |name: &str| {
            flags.get(name).and_then(Value::as_bool).unwrap_or(false)
        };
        let new_status = if flag("error") {
            DeviceStatus::Alarm
        } else if flag("closedOrError") && !flag("operational") {
            DeviceStatus::Unknown
        } else if flag("printing") {
            DeviceStatus::Run
        } else if flag("paused") || flag("pausing") {
            DeviceStatus::Hold
        } else if flag("cancelling") {
            DeviceStatus::Run
        } else if flag("operational") {
            DeviceStatus::Idle
        } else {
            match text {
                "Operational" => DeviceStatus::Idle,
                "Printing" | "Starting" | "Cancelling" => DeviceStatus::Run,
                "Pausing" | "Paused" => DeviceStatus::Hold,
                "Error" | "Offline after error" => DeviceStatus::Alarm,
                _ => DeviceStatus::Unknown,
            }
        };
        self.set_status(new_status);
    }

    fn report_progress(&self, completion: &Value) {
        if let Some(percent) = completion.as_f64() {
            log_crate::debug!("Job progress: {percent:.1}%");
        }
    }

    fn finish_job(&self, success: bool) {
        let was_active = {
            let mut job = self.job_done.lock().unwrap();
            let was_active = job.active;
            job.active = false;
            job.done = Some(if success {
                Ok(())
            } else {
                Err("OctoPrint reported that the print job failed".into())
            });
            was_active
        };
        if !was_active {
            return;
        }
        self.job_running.store(false, Ordering::SeqCst);
        if success {
            self.set_status(DeviceStatus::Idle);
            self.set_error(None);
        } else {
            self.set_status(DeviceStatus::Alarm);
            self.set_error(Some(DeviceError {
                code: 1,
                title: "Print Failed".to_string(),
                description: "OctoPrint reported that the print job \
                              failed. Check OctoPrint for details."
                    .to_string(),
            }));
        }
        self.events.job_finished();
    }

    // ---------------------------------------------------------------
    // Jobs
    // ---------------------------------------------------------------

    fn upload_and_print_body(gcode: &str) -> Vec<u8> {
        gcode.as_bytes().to_vec()
    }

    async fn upload_and_print(&self, gcode: &str) -> Result<(), DriverError> {
        let timeout = self.config.secs(self.config.request_timeout);
        let headers = [("X-Api-Key", self.api_key())];
        let headers: Vec<(&str, &str)> =
            headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let response = http::post_multipart_fields(
            &self.config.host,
            self.config.port,
            &self.api_path("/api/files/local"),
            "file",
            JOB_FILENAME,
            Self::upload_and_print_body(gcode).as_slice(),
            &[("select", "true"), ("print", "true")],
            &headers,
            timeout,
        )
        .await?;
        if response.status == 403 {
            return Err(DriverError::Device(
                "authentication failed during upload".to_string(),
            ));
        }
        if response.status == 409 {
            return Err(DriverError::Device(
                "printer is busy or not operational; cannot start a \
                 new job"
                    .to_string(),
            ));
        }
        if !(200..300).contains(&response.status) {
            return Err(DriverError::Device(format!(
                "unexpected HTTP status {} from the upload endpoint",
                response.status
            )));
        }
        let data = serde_json::from_str::<Value>(&response.text())
            .unwrap_or(json!({}));
        if data.get("effectivePrint").and_then(Value::as_bool) != Some(true) {
            return Err(DriverError::Device(
                "OctoPrint accepted the file but could not start \
                 printing; the printer may not be operational or is \
                 already busy"
                    .to_string(),
            ));
        }
        Ok(())
    }

    // ---------------------------------------------------------------
    // Motion and control
    // ---------------------------------------------------------------

    async fn printer_command(&self, command: Value) -> Result<(), DriverError> {
        self.api_request("POST", "/api/printer/command", Some(command))
            .await?;
        Ok(())
    }
}

/// Extracts the JSON payloads of a push frame: "o"/"h" are protocol
/// frames (no payload), "a[...]" wraps an array of JSON-string
/// messages, and the raw transport delivers message objects as plain
/// JSON text.
fn sockjs_payloads(frame: &str) -> Vec<Value> {
    if frame == "o" || frame == "h" {
        return Vec::new();
    }
    if let Some(messages_json) = frame.strip_prefix("a[") {
        let Ok(messages) =
            serde_json::from_str::<Vec<Value>>(&format!("[{messages_json}"))
        else {
            return Vec::new();
        };
        return messages
            .into_iter()
            .filter_map(|raw_msg| match raw_msg {
                Value::String(text) => serde_json::from_str(&text).ok(),
                value => Some(value),
            })
            .collect();
    }
    serde_json::from_str::<Value>(frame)
        .ok()
        .map(|value| vec![value])
        .unwrap_or_default()
}

/// The session's [`Driver`] impl: the single, real implementation
/// of every operation the protocol supports.
#[async_trait]
impl Driver for OctoPrintDriver {
    fn info(&self) -> &'static crate::driver::core::driver::DriverInfo {
        &INFO
    }

    fn state(&self) -> DeviceState {
        self.state.lock().unwrap().clone()
    }
    fn resource_uri(&self) -> Option<String> {
        Some(format!("tcp://{}:{}", self.config.host, self.config.port))
    }
    fn is_job_running(&self) -> bool {
        self.job_running.load(Ordering::SeqCst)
    }
    fn abort_background_tasks_now(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        let task = self.connection_task.lock().unwrap().take();
        if let Some(task) = task {
            task.abort();
        }
        let tasks: Vec<JoinHandle<()>> =
            std::mem::take(&mut *self.background_tasks.lock().unwrap());
        for task in tasks {
            task.abort();
        }
    }
    async fn cleanup(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        self.abort_background_tasks().await;
        self.update_connection_status(TransportStatus::Disconnected, None);
    }
    async fn set_hold(&self, hold: bool) -> Result<(), DriverError> {
        let action = if hold { "pause" } else { "resume" };
        self.api_request(
            "POST",
            "/api/job",
            Some(json!({"command": "pause", "action": action})),
        )
        .await?;
        Ok(())
    }
    async fn clear_alarm(&self) -> Result<(), DriverError> {
        self.execute_command("M999").await?;
        self.set_error(None);
        self.set_status(DeviceStatus::Idle);
        Ok(())
    }
    async fn set_wcs_offset(
        &self,
        wcs_slot: &str,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> Result<(), DriverError> {
        let p_map: HashMap<&str, i64> = [
            ("G54", 1),
            ("G55", 2),
            ("G56", 3),
            ("G57", 4),
            ("G58", 5),
            ("G59", 6),
        ]
        .into_iter()
        .collect();
        let p_num = p_map.get(wcs_slot).copied().ok_or_else(|| {
            DriverError::Device(format!("invalid WCS slot: {wcs_slot}"))
        })?;
        let mut command = format!("G10 L2 P{p_num} X{x:.3} Y{y:.3}");
        if let Some(z) = z {
            command.push_str(&format!(" Z{z:.3}"));
        }
        self.execute_command(&command).await.map(|_| ())
    }
    async fn read_wcs_offsets(
        &self,
    ) -> Result<HashMap<String, (f64, f64, f64)>, DriverError> {
        // OctoPrint exposes no work coordinate offsets.
        Ok(HashMap::new())
    }
    async fn read_parser_state(&self) -> Result<Option<String>, DriverError> {
        // The server API exposes no G-code parser state.
        Ok(None)
    }
    async fn read_settings(
        &self,
    ) -> Result<Vec<(String, String)>, DriverError> {
        // OctoPrint does not expose firmware settings.
        Ok(Vec::new())
    }
    async fn write_setting(
        &self,
        _key: &str,
        _value: &str,
    ) -> Result<(), DriverError> {
        Err(DriverError::Unsupported(
        "OctoPrint does not support writing device firmware              settings through its API"
            .to_string(),
    ))
    }
    async fn connect(self: Arc<Self>) -> Result<(), DriverError> {
        self.abort_background_tasks().await;
        self.keep_running.store(true, Ordering::SeqCst);
        self.job_running.store(false, Ordering::SeqCst);
        let core = self.clone();
        let task = tokio::spawn(async move { core.connection_loop().await });
        *self.connection_task.lock().unwrap() = Some(task);

        Ok(())
    }

    async fn run(
        self: Arc<Self>,
        gcode: &str,
        op_map: &HashMap<usize, i64>,
        _op_estimates: &[f64],
    ) -> Result<(), DriverError> {
        // No granular progress exists on this protocol; report
        // every op in the line map as done once the job starts.
        let op_indices: Vec<i64> = op_map.values().copied().collect();
        self.upload_and_print(gcode).await?;
        for op_index in &op_indices {
            self.events.command_done(*op_index);
        }
        {
            let mut job = self.job_done.lock().unwrap();
            job.active = true;
            job.done = None;
        }
        self.job_running.store(true, Ordering::SeqCst);

        // Wait for the server's terminal print event (or the
        // polling fallback noticing the job left the printing
        // states).
        loop {
            if !self.keep_running.load(Ordering::SeqCst) {
                return Err(DriverError::Connection(
                    "connection closed while the job was running".to_string(),
                ));
            }
            {
                let job = self.job_done.lock().unwrap();
                if let Some(done) = &job.done {
                    return done.clone().map_err(DriverError::Device);
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn run_raw(
        self: Arc<Self>,
        machine_code: &str,
    ) -> Result<(), DriverError> {
        let commands: Vec<String> = machine_code
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        if commands.is_empty() {
            return Ok(());
        }
        let payload = if commands.len() == 1 {
            json!({"command": commands[0]})
        } else {
            json!({"commands": commands})
        };
        self.api_request("POST", "/api/printer/command", Some(payload))
            .await?;
        self.events.job_finished();
        Ok(())
    }

    async fn cancel(&self, _mode: CancelMode) -> Result<(), DriverError> {
        // OctoPrint answers 409 when there is no job to cancel; that
        // is not an error.
        self.api_request_allowing(
            "POST",
            "/api/job",
            Some(json!({"command": "cancel"})),
            &[409],
        )
        .await?;
        self.finish_job(true);
        Ok(())
    }

    async fn execute_interactive_command(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        self.execute_command(command).await?;
        // OctoPrint reports no per-line response text.
        Ok(Vec::new())
    }

    async fn jog(
        &self,
        speed: f64,
        deltas: &[(String, f64)],
    ) -> Result<(), DriverError> {
        let speed = Some(speed);
        let mut params = json!({"command": "jog"});
        if let Some(speed) = speed {
            params["speed"] = json!(speed);
        }
        for (axis, distance) in deltas {
            params[axis.as_str()] = json!(distance);
        }
        self.api_request("POST", "/api/printer/printhead", Some(params))
            .await?;
        Ok(())
    }

    async fn home(
        &self,
        axes: Option<Vec<String>>,
        _active_wcs: &str,
    ) -> Result<(), DriverError> {
        let axis_list: Vec<String> = axes
            .map(|names| names.iter().map(|name| name.to_lowercase()).collect())
            .unwrap_or_else(|| {
                vec!["x".to_string(), "y".to_string(), "z".to_string()]
            });
        self.api_request(
            "POST",
            "/api/printer/printhead",
            Some(json!({"command": "home", "axes": axis_list})),
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
        let mut params =
            json!({"command": "jog", "x": x, "y": y, "absolute": true});
        if let Some(z) = z {
            params["z"] = json!(z);
        }
        params["speed"] = json!(speed);
        self.api_request("POST", "/api/printer/printhead", Some(params))
            .await?;
        Ok(())
    }

    async fn select_tool(&self, tool_number: i64) -> Result<(), DriverError> {
        self.execute_command(&format!("T{tool_number}"))
            .await
            .map(|_| ())
    }
    async fn set_power(&self, power: Option<f64>) -> Result<(), DriverError> {
        let command = match power {
            Some(p) if p <= 0.0 => "M5".to_string(),
            Some(p) => format!("M3 S{}", p.round()),
            None => "M5".to_string(),
        };
        self.execute_command(&command).await.map(|_| ())
    }
    async fn set_focus_power(
        &self,
        power: Option<f64>,
    ) -> Result<(), DriverError> {
        self.set_power(power).await
    }
    async fn detect_unit_system(
        &self,
    ) -> Result<Option<UnitSystem>, DriverError> {
        // OctoPrint exposes no unit configuration; the dialect works
        // in millimeters.
        Ok(None)
    }

    async fn run_probe_cycle(
        &self,
        axis_letter: &str,
        max_travel: f64,
        feed_rate: f64,
    ) -> Result<Option<(f64, f64, f64)>, DriverError> {
        self.events
            .probe_status(&format!("Probing {axis_letter}..."));
        self.execute_command(&format!(
            "G38.2 {axis_letter}{max_travel:.3} F{}",
            feed_rate as i64
        ))
        .await?;
        self.events.probe_status(
            "Probe command sent. OctoPrint does not report probe \
             results via its API.",
        );
        Ok(None)
    }

    async fn execute_command(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        self.printer_command(json!({"command": command})).await?;
        Ok(Vec::new())
    }
}

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("host", SetupKind::Hostname).required(),
    SetupField::new("port", SetupKind::Port)
        .with_default("80")
        .with_range(1.0, 65535.0),
    SetupField::new("path", SetupKind::Text),
    SetupField::new("api_key", SetupKind::Text),
];
