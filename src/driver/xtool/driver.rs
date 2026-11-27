//! The xTool session: connection lifecycle, command exchange over
//! the proprietary HTTP API, and job execution with its WebSocket
//! job-state events.

use crate::driver::core::driver::{
    info::caps, ConnectionKind, DriverInfo, DriverMaturity,
};

use crate::driver::core::driver::{SetupField, SetupKind};

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::{
    AsyncEvent, CancelMode, Driver, DriverError, DriverEvents,
};
use crate::driver::core::http;
use crate::driver::core::types::{
    DeviceError, DeviceState, DeviceStatus, TransportStatus, UnitSystem,
};
use crate::driver::core::ws::WsClient;
use async_trait::async_trait;

/// Default HTTP (API) port of xTool machines.
pub const DEFAULT_HTTP_PORT: u16 = 8080;
/// Default WebSocket (notification) port of xTool machines.
pub const DEFAULT_WS_PORT: u16 = 8081;

/// Fixed speed (mm/min) for absolute moves without an explicit one.
/// The stop flush block xTool's own software sends after
/// `/cnc/data?action=stop`: terminate the program and force the
/// laser output off.
const STOP_FLUSH_BLOCK: &str = "M108\nM112 N0\nM9 S0 N0\n";

/// Driver configuration.
#[derive(Clone)]
pub struct XtoolConfig {
    pub host: String,
    pub http_port: u16,
    pub ws_port: u16,
    /// Budget for a single HTTP request, including connect.
    pub request_timeout: f64,
    /// Budget for the WebSocket handshake.
    pub connect_timeout: f64,
    /// Delay between connection attempts.
    pub reconnect_delay: f64,
    /// Budget for the homing cycle (M28) to report idle again.
    pub homing_timeout: f64,
    /// Budget for the first job-state event after starting a job.
    pub job_start_timeout: f64,
    /// Interval of the HTTP liveness poll.
    pub poll_interval: f64,
}

impl XtoolConfig {
    pub fn new(host: &str) -> Self {
        Self {
            host: host.to_string(),
            http_port: DEFAULT_HTTP_PORT,
            ws_port: DEFAULT_WS_PORT,
            request_timeout: 5.0,
            connect_timeout: 6.0,
            reconnect_delay: 5.0,
            homing_timeout: 120.0,
            job_start_timeout: 15.0,
            poll_interval: 0.5,
        }
    }

    fn secs(v: f64) -> Duration {
        Duration::from_secs_f64(v.max(0.0))
    }

    fn base(&self) -> String {
        format!("http://{}:{}", self.host, self.http_port)
    }
}

/// The xTool session: connection lifecycle, interactive commands,
/// homing/jogging sequences and job execution.
pub struct XtoolNetworkDriver {
    config: XtoolConfig,
    dialect: RwLock<Dialect>,
    events: Arc<dyn DriverEvents>,
    state: Mutex<DeviceState>,
    keep_running: AtomicBool,
    job_running: AtomicBool,
    is_cancelled: AtomicBool,
    /// Set when the machine reports IDLE or an error state.
    settled: AsyncEvent,
    /// Set when the machine reports a WORKING_* event.
    started: AsyncEvent,
    /// Set when the WebSocket drops; wakes pending waiters.
    ws_lost: AsyncEvent,
    cmd_lock: tokio::sync::Mutex<()>,
    connection_task: Mutex<Option<JoinHandle<()>>>,
    background_tasks: Mutex<Vec<JoinHandle<()>>>,
}

/// Typed identity and capabilities of this driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "XtoolNetworkDriver",
    connection: ConnectionKind::Network,
    maturity: DriverMaturity::Experimental,
    caps: caps(
        false, true, true, true, true, false, true, false, false, true, false,
    ),
    uses_gcode: true,
    machine_space_wcs: "G53",
    supported_wcs: &[],
    config_keys: &["host", "http_port", "ws_port"],
    setup: SETUP,
    fingerprint: None,
};

impl XtoolNetworkDriver {
    pub fn new(
        config: XtoolConfig,
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            dialect: RwLock::new(dialect),
            events,
            state: Mutex::new(DeviceState::default()),
            keep_running: AtomicBool::new(false),
            job_running: AtomicBool::new(false),
            is_cancelled: AtomicBool::new(false),
            settled: AsyncEvent::new(),
            started: AsyncEvent::new(),
            ws_lost: AsyncEvent::new(),
            cmd_lock: tokio::sync::Mutex::new(()),
            connection_task: Mutex::new(None),
            background_tasks: Mutex::new(Vec::new()),
        })
    }

    // ---------------------------------------------------------------
    // Accessors
    // ---------------------------------------------------------------

    fn dialect(&self) -> Dialect {
        self.dialect.read().unwrap().clone()
    }

    fn update_connection_status(
        &self,
        status: TransportStatus,
        message: Option<&str>,
    ) {
        self.events.connection_status(status, message);
    }

    fn update_command_status(
        &self,
        status: TransportStatus,
        message: Option<&str>,
    ) {
        self.events.command_status(status, message);
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

    fn abort_background_tasks_now_inner(&self) {
        let tasks: Vec<JoinHandle<()>> =
            std::mem::take(&mut *self.background_tasks.lock().unwrap());
        for task in tasks {
            task.abort();
        }
    }

    /// The connection loop: verify the endpoint really is an xTool
    /// machine, then keep the liveness poll and the WebSocket
    /// notification channel running until disconnect.
    async fn connection_loop(self: Arc<Self>) {
        while self.keep_running.load(Ordering::SeqCst) {
            self.update_connection_status(TransportStatus::Connecting, None);
            match self.verify_machine().await {
                Ok(machine_type) => {
                    log_crate::info!(
                        "xTool machine identified as: {machine_type}"
                    );
                }
                Err(err) => {
                    log_crate::error!("Connection error: {err}");
                    self.update_connection_status(
                        TransportStatus::Error,
                        Some(&err.to_string()),
                    );
                    self.sleep_reconnect().await;
                    continue;
                }
            }

            match self.clone().event_loop().await {
                Ok(()) => log_crate::debug!("Event loop finished cleanly."),
                Err(err) => {
                    log_crate::error!("Connection error: {err}");
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
            self.sleep_reconnect().await;
        }
    }

    async fn sleep_reconnect(&self) {
        if self.keep_running.load(Ordering::SeqCst) {
            tokio::time::sleep(XtoolConfig::secs(self.config.reconnect_delay))
                .await;
        }
    }

    /// GET /getmachinetype and confirm the device identifies itself
    /// as an xTool machine.
    ///
    /// Any device answering an HTTP request can be pointed at here;
    /// without this check a misconfigured endpoint (or an ESP3D
    /// firmware on another machine) shows up as "connected" while
    /// every command silently fails.
    async fn verify_machine(&self) -> Result<String, DriverError> {
        let timeout = XtoolConfig::secs(self.config.request_timeout);
        let machine_type = probe_machine_type(
            &self.config.host,
            self.config.http_port,
            timeout,
        )
        .await?;
        if !machine_type.to_ascii_lowercase().contains("xtool") {
            return Err(DriverError::Device(format!(
                "the device at {}:{} did not identify itself as an \
                 xTool machine (reported type: {:?})",
                self.config.host, self.config.http_port, machine_type,
            )));
        }
        Ok(machine_type)
    }

    /// The event loop: establish the WebSocket notification
    /// channel, then supervise it and the HTTP liveness poll; the
    /// first failure of either ends the connected phase.
    async fn event_loop(self: Arc<Self>) -> Result<(), DriverError> {
        self.settled.clear();
        self.started.clear();
        self.ws_lost.clear();

        let origin = self.config.base();
        let ws = WsClient::connect(
            &self.config.host,
            self.config.ws_port,
            Some(&origin),
            XtoolConfig::secs(self.config.connect_timeout),
        )
        .await?;
        self.update_connection_status(TransportStatus::Connected, None);

        let (tx, mut rx) = mpsc::unbounded_channel();
        let poller_tx = tx.clone();
        let poller = tokio::spawn({
            let core = self.clone();
            async move { core.poll_loop(poller_tx).await }
        });
        let receiver = tokio::spawn({
            let core = self.clone();
            async move { core.ws_receive_loop(ws, tx).await }
        });
        {
            let mut background = self.background_tasks.lock().unwrap();
            background.push(poller);
            background.push(receiver);
        }

        let result = match rx.recv().await {
            Some(err) => Err(err),
            None => Ok(()),
        };
        self.abort_background_tasks_now_inner();
        if result.is_err() {
            self.ws_lost.set();
        }
        result
    }

    /// Periodically GET /ping; the first failure reports the device
    /// as unreachable.
    async fn poll_loop(
        self: Arc<Self>,
        tx: mpsc::UnboundedSender<DriverError>,
    ) {
        loop {
            let result = match http::get(
                &self.config.host,
                self.config.http_port,
                "/ping",
                XtoolConfig::secs(self.config.request_timeout),
            )
            .await
            {
                Ok(response) if response.status == 200 => None,
                Ok(response) => Some(DriverError::Connection(format!(
                    "liveness poll returned HTTP {}",
                    response.status
                ))),
                Err(err) => Some(err),
            };
            if let Some(err) = result {
                log_crate::warn!("liveness poll failed: {err}");
                let _ = tx.send(err);
                return;
            }
            if !self.keep_running.load(Ordering::SeqCst) {
                return;
            }
            tokio::time::sleep(XtoolConfig::secs(self.config.poll_interval))
                .await;
        }
    }

    /// Receive and dispatch WebSocket messages until the channel
    /// drops.
    async fn ws_receive_loop(
        self: Arc<Self>,
        mut ws: WsClient,
        tx: mpsc::UnboundedSender<DriverError>,
    ) {
        loop {
            match ws.receive().await {
                Ok(message) => {
                    for line in message.lines() {
                        self.handle_event(line);
                    }
                }
                Err(err) => {
                    log_crate::warn!("WebSocket channel failed: {err}");
                    let _ = tx.send(err);
                    return;
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // WebSocket event handling
    // ---------------------------------------------------------------

    fn handle_event(&self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        log_crate::debug!("xTool event: {line}");
        self.events.raw_rx(line);
        let (status, error) = parse_event(line);
        if status == DeviceStatus::Unknown {
            return;
        }
        let (old_status, old_error) = {
            let state = self.state.lock().unwrap();
            (state.status, state.error.clone())
        };
        if status != old_status || error != old_error {
            {
                let mut state = self.state.lock().unwrap();
                state.status = status;
                state.error = error;
            }
            self.events.state_changed(&self.state());
        }
        if line.starts_with("ok:WORKING_") {
            self.started.set();
        }
        if matches!(status, DeviceStatus::Idle | DeviceStatus::Alarm) {
            self.settled.set();
        }
    }

    /// Wait until the machine reports IDLE, raising on error states.
    /// An infinite *timeout* waits unbounded (job length is
    /// unknowable; cancellation flows through `cancel`).
    async fn wait_settled(&self, timeout: f64) -> Result<(), DriverError> {
        let deadline = if timeout.is_finite() {
            Some(tokio::time::Instant::now() + XtoolConfig::secs(timeout))
        } else {
            None
        };
        let timer = async {
            match deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending::<()>().await,
            }
        };
        tokio::pin!(timer);
        loop {
            let settled = self.settled.wait();
            tokio::pin!(settled);
            tokio::select! {
                _ = &mut settled => {
                    self.settled.clear();
                }
                _ = self.ws_lost.wait() => {
                    return Err(DriverError::Connection(
                        "connection lost while waiting for the machine"
                            .to_string(),
                    ));
                }
                _ = &mut timer => {
                    return Err(DriverError::Timeout("operation timed out".to_string()));
                }
            }
            let state = self.state();
            match state.status {
                DeviceStatus::Idle => return Ok(()),
                DeviceStatus::Alarm => {
                    let message = match state.error {
                        Some(error) => {
                            format!("{}: {}", error.title, error.description)
                        }
                        None => {
                            "the machine reported an error state".to_string()
                        }
                    };
                    return Err(DriverError::Device(message));
                }
                _ => {}
            }
        }
    }

    // ---------------------------------------------------------------
    // HTTP command exchange
    // ---------------------------------------------------------------

    async fn request_get(&self, path: &str) -> Result<String, DriverError> {
        let response = http::get(
            &self.config.host,
            self.config.http_port,
            path,
            XtoolConfig::secs(self.config.request_timeout),
        )
        .await
        .and_then(|response| {
            if response.status == 200 {
                Ok(response)
            } else {
                Err(DriverError::Connection(format!(
                    "unexpected HTTP status {} for {path}",
                    response.status
                )))
            }
        })?;
        Ok(response.text())
    }

    async fn request_post(
        &self,
        path: &str,
        content_type: &str,
        body: &[u8],
    ) -> Result<String, DriverError> {
        let response = http::post(
            &self.config.host,
            self.config.http_port,
            path,
            content_type,
            body,
            XtoolConfig::secs(self.config.request_timeout),
        )
        .await
        .and_then(|response| {
            if response.status == 200 {
                Ok(response)
            } else {
                Err(DriverError::Connection(format!(
                    "unexpected HTTP status {} for {path}",
                    response.status
                )))
            }
        })?;
        Ok(response.text())
    }

    /// Validate the JSON acknowledgement of an HTTP API request: a
    /// JSON object whose `result` member is `"ok"`.  Anything else
    /// (e.g. an HTML error page from a foreign device) is a failure.
    fn check_result(&self, text: &str, what: &str) -> Result<(), DriverError> {
        let payload: HashMap<String, Value> = match serde_json::from_str(text) {
            Ok(payload) => payload,
            Err(_) => {
                return Err(DriverError::Device(format!(
                    "unexpected response from the device (response: {})",
                    truncate(text.trim(), 100),
                )));
            }
        };
        if payload.get("result").and_then(Value::as_str) == Some("ok") {
            return Ok(());
        }
        Err(DriverError::Device(format!(
            "the device rejected {what} (response: {})",
            truncate(text.trim(), 100),
        )))
    }

    fn raw_tx_line(&self, line: &str) {
        self.events.raw_tx(line);
    }

    /// Execute a single G-code line without failing the caller on
    /// Execute a multi-line block of G-code via `POST /cmd`.
    pub async fn execute_block(
        &self,
        block: String,
    ) -> Result<(), DriverError> {
        let _guard = self.cmd_lock.lock().await;
        for line in block.lines() {
            if !line.trim().is_empty() {
                self.raw_tx_line(line);
            }
        }
        let text = self
            .request_post("/cmd", "text/plain", block.as_bytes())
            .await?;
        self.check_result(&text, "the command block")
    }

    // ---------------------------------------------------------------
    // Machine sequences
    // ---------------------------------------------------------------

    async fn send_safety_off(&self, emergency: bool) {
        let dialect = self.dialect();
        let mut commands = dialect.safety_off_commands.clone();
        if emergency {
            if let Some(stop) = &dialect.emergency_stop {
                commands.push(stop.clone());
            }
        }
        for command in commands {
            if command.is_empty() {
                continue;
            }
            if let Err(err) = self.execute_interactive_command(&command).await {
                log_crate::warn!("Safety command '{command}' failed: {err}");
            }
        }
    }

    // ---------------------------------------------------------------
    // Jobs
    // ---------------------------------------------------------------

    async fn run_inner(&self, gcode: &str) -> Result<(), DriverError> {
        let text = http::post_multipart(
            &self.config.host,
            self.config.http_port,
            "/upload?filetype=1",
            "file",
            "rayforge.gcode",
            gcode.as_bytes(),
            XtoolConfig::secs(self.config.request_timeout),
        )
        .await
        .and_then(|response| {
            if response.status == 200 {
                Ok(response.text())
            } else {
                Err(DriverError::Connection(format!(
                    "unexpected HTTP status {} for /upload",
                    response.status
                )))
            }
        })?;
        self.check_result(&text, "the upload")?;

        self.settled.clear();
        self.started.clear();
        let text = self.request_get("/read?file=%2Ftmp.gcode").await?;
        self.check_result(&text, "the job start")?;

        let started = self.started.wait();
        tokio::pin!(started);
        let settled = self.settled.wait();
        tokio::pin!(settled);
        let deadline = tokio::time::Instant::now()
            + XtoolConfig::secs(self.config.job_start_timeout);
        tokio::select! {
            _ = &mut started => {}
            _ = &mut settled => {}
            _ = tokio::time::sleep_until(deadline) => {
                return Err(DriverError::Device(
                    "the machine did not start the job".to_string(),
                ));
            }
        }
        self.wait_settled(f64::INFINITY).await
    }
}

/// Map a WebSocket event line to ``(status, error)``.  Returns
/// ``Unknown`` for lines that carry no state information.
/// GETs the machine-type endpoint and returns the reported type.
///
/// Fails on transport errors, unexpected HTTP statuses and
/// malformed JSON bodies; a missing type field yields an empty
/// string. The xTool identification itself is the caller's check.
pub async fn probe_machine_type(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<String, DriverError> {
    let response = http::get(host, port, "/getmachinetype", timeout)
        .await
        .and_then(|response| {
            if response.status == 200 {
                Ok(response)
            } else {
                Err(DriverError::Connection(format!(
                    "unexpected HTTP status {} from /getmachinetype",
                    response.status
                )))
            }
        })?;
    let payload: HashMap<String, Value> =
        serde_json::from_str(&response.text()).map_err(|err| {
            DriverError::Device(format!(
                "unexpected response from /getmachinetype: {err}"
            ))
        })?;
    Ok(payload
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
}

fn parse_event(line: &str) -> (DeviceStatus, Option<DeviceError>) {
    let (status, error) = match line {
        "ok:IDLE" | "WORK_STOPPED" => (DeviceStatus::Idle, None),
        "ok:PAUSING" => (DeviceStatus::Hold, None),
        "ok:ERROR" => (
            DeviceStatus::Alarm,
            Some(DeviceError::new(
                -1,
                "Machine Error",
                "The machine reported an error state.",
            )),
        ),
        "err:tiltCheck" => (
            DeviceStatus::Alarm,
            Some(DeviceError::new(
                -1,
                "Tilt Detected",
                "The machine detected that it was lifted or tilted \
                 during operation.",
            )),
        ),
        "err:movingCheck" => (
            DeviceStatus::Alarm,
            Some(DeviceError::new(
                -1,
                "Movement Detected",
                "The machine detected unexpected movement of its base.",
            )),
        ),
        "err:limitCheck" => (
            DeviceStatus::Alarm,
            Some(DeviceError::new(
                -1,
                "Limit Switch Triggered",
                "A limit switch was activated during operation.",
            )),
        ),
        "err:flameCheck" => (
            DeviceStatus::Alarm,
            Some(DeviceError::new(
                -1,
                "Flame Detected",
                "The machine detected smoke or an open flame.",
            )),
        ),
        other => {
            if other.starts_with("ok:WORKING_") {
                (DeviceStatus::Run, None)
            } else if other.starts_with("err:") {
                (
                    DeviceStatus::Alarm,
                    Some(DeviceError::new(
                        -1,
                        "Machine Error",
                        "The machine reported an error state.",
                    )),
                )
            } else {
                (DeviceStatus::Unknown, None)
            }
        }
    };
    (status, error)
}

fn truncate(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// URL-encode for a query parameter.
fn url_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Render a feed rate the way the GRBL dialect does: integral
/// values without a decimal point.
fn format_speed(speed: f64) -> String {
    if speed.fract() == 0.0 {
        format!("{}", speed as i64)
    } else {
        format!("{speed}")
    }
}

/// Format a dialect power template with `{power:.0f}`-style
/// placeholders via the shared dialect formatter.
fn format_power(template: &str, power: f64) -> String {
    let args = [("power", crate::driver::core::dialect::Arg::Length(power))];
    crate::driver::core::dialect::format_template(template, &args)
        .unwrap_or_else(|_| template.to_string())
}

/// The session's [`Driver`] impl: the single, real implementation
/// of every operation the protocol supports.
#[async_trait]
impl Driver for XtoolNetworkDriver {
    fn info(&self) -> &'static crate::driver::core::driver::DriverInfo {
        &INFO
    }

    fn state(&self) -> DeviceState {
        self.state.lock().unwrap().clone()
    }
    fn resource_uri(&self) -> Option<String> {
        Some(format!(
            "tcp://{}:{}",
            self.config.host, self.config.http_port
        ))
    }
    fn is_job_running(&self) -> bool {
        self.job_running.load(Ordering::SeqCst)
    }
    fn update_dialect(&self, dialect: Dialect) {
        *self.dialect.write().unwrap() = dialect;
    }
    async fn cleanup(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        self.abort_background_tasks().await;
        self.update_connection_status(TransportStatus::Disconnected, None);
    }
    fn abort_background_tasks_now(&self) {
        self.keep_running.store(false, Ordering::SeqCst);
        let task = self.connection_task.lock().unwrap().take();
        if let Some(task) = task {
            task.abort();
        }
        self.abort_background_tasks_now_inner();
    }
    async fn set_hold(&self, hold: bool) -> Result<(), DriverError> {
        let action = if hold { "pause" } else { "resume" };
        let path = format!("/cnc/data?action={action}");
        let text = self.request_get(&path).await?;
        self.check_result(&text, &format!("'{action}'"))
    }
    async fn connect(self: Arc<Self>) -> Result<(), DriverError> {
        self.abort_background_tasks().await;
        self.keep_running.store(true, Ordering::SeqCst);
        self.is_cancelled.store(false, Ordering::SeqCst);
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
        self.job_running.store(true, Ordering::SeqCst);
        self.is_cancelled.store(false, Ordering::SeqCst);
        // The protocol has no granular progress: report every op as
        // done once the job is running on the device.
        let mut ops: Vec<i64> = op_map.values().copied().collect();
        ops.sort_unstable();
        ops.dedup();
        for op in ops {
            self.events.command_done(op);
        }
        let result = self.run_inner(gcode).await;
        if !self.is_cancelled.load(Ordering::SeqCst) {
            self.job_running.store(false, Ordering::SeqCst);
            self.events.job_finished();
        }
        result
    }

    async fn run_raw(
        self: Arc<Self>,
        machine_code: &str,
    ) -> Result<(), DriverError> {
        Driver::run(self, machine_code, &HashMap::new(), &[]).await
    }

    async fn cancel(&self, mode: CancelMode) -> Result<(), DriverError> {
        let emergency = mode == CancelMode::Immediate;
        self.is_cancelled.store(true, Ordering::SeqCst);
        let stop = async {
            let text = self.request_get("/cnc/data?action=stop").await?;
            self.check_result(&text, "the stop command")
        };
        if let Err(err) = stop.await {
            log_crate::warn!("Stopping the xTool job failed: {err}");
        }
        if let Err(err) = self.execute_block(STOP_FLUSH_BLOCK.to_string()).await
        {
            log_crate::warn!("Sending the stop flush block failed: {err}");
        }
        self.send_safety_off(emergency).await;
        self.settled.set();
        self.job_running.store(false, Ordering::SeqCst);
        self.events.job_finished();

        Ok(())
    }

    async fn execute_interactive_command(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        let command = command.to_string();
        let _guard = self.cmd_lock.lock().await;
        self.raw_tx_line(&command);
        let path = format!("/cmd?cmd={}", url_encode(&command));
        let text = match self.request_get(&path).await {
            Ok(text) => text,
            Err(err) => {
                self.update_command_status(
                    TransportStatus::Error,
                    Some(&err.to_string()),
                );
                return Err(err);
            }
        };
        if let Err(err) = self.check_result(&text, &format!("'{command}'")) {
            self.update_command_status(
                TransportStatus::Error,
                Some(&err.to_string()),
            );
            return Err(err);
        }
        self.update_command_status(TransportStatus::Idle, None);
        Ok(Vec::new())
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
        self.settled.clear();
        self.execute_interactive_command("M17 S1").await?;
        self.execute_interactive_command("M207 S0").await?;
        self.execute_interactive_command("M28").await?;
        self.wait_settled(self.config.homing_timeout).await?;
        self.execute_interactive_command("M18").await?;
        Ok(())
    }

    async fn move_to(
        &self,
        speed: f64,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> Result<(), DriverError> {
        let mut axes = vec![format!("X{x}"), format!("Y{y}")];
        if let Some(z) = z {
            axes.push(format!("Z{z}"));
        }
        let block = format!(
            "M17 S1\nM207 S0\nM101\nG90\n\
             G1 {} F{} S0\nM18\n",
            axes.join(" "),
            format_speed(speed),
        );
        self.execute_block(block).await
    }

    async fn jog(
        &self,
        speed: f64,
        deltas: &[(String, f64)],
    ) -> Result<(), DriverError> {
        let deltas = deltas.to_vec();
        if deltas.is_empty() {
            return Ok(());
        }
        let axes = deltas
            .iter()
            .map(|(name, distance)| {
                format!("{}{}", name.to_ascii_uppercase(), distance)
            })
            .collect::<Vec<_>>()
            .join(" ");
        let block = format!(
            "M17 S1\nM207 S0\nM101\nG92 X0 Y0\nG90\n\
             G1 {axes} F{} S0\nM18\n",
            format_speed(speed),
        );
        self.execute_block(block).await
    }

    async fn set_power(&self, power: Option<f64>) -> Result<(), DriverError> {
        let dialect = self.dialect();
        let command = match power {
            Some(power) => format_power(&dialect.laser_on, power),
            None => dialect.laser_off.clone(),
        };
        if let Err(err) = self.execute_interactive_command(&command).await {
            log_crate::warn!("set_power failed: {err}");
        }

        Ok(())
    }

    async fn set_focus_power(
        &self,
        power: Option<f64>,
    ) -> Result<(), DriverError> {
        let dialect = self.dialect();
        let command = match power {
            Some(power) => format_power(&dialect.focus_laser_on, power),
            None => dialect.laser_off.clone(),
        };
        if let Err(err) = self.execute_interactive_command(&command).await {
            log_crate::warn!("set_focus_power failed: {err}");
        }

        Ok(())
    }

    async fn select_tool(&self, tool_number: i64) -> Result<(), DriverError> {
        let template = self.dialect().tool_change;
        let command = crate::driver::core::dialect::format_template(
            &template,
            &[(
                "tool_number",
                crate::driver::core::dialect::Arg::Int(tool_number),
            )],
        )
        .map_err(DriverError::Other)?;
        self.execute_interactive_command(&command).await?;
        Ok(())
    }

    async fn clear_alarm(&self) -> Result<(), DriverError> {
        // The firmware clears its safety alarms itself; there is no
        // remote reset, so only the locally mirrored error clears.
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
    ) -> Result<HashMap<String, (f64, f64, f64)>, DriverError> {
        Ok(HashMap::new())
    }

    async fn execute_command(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        let command = command.to_string();
        if let Err(err) = self.execute_interactive_command(&command).await {
            self.update_command_status(
                TransportStatus::Error,
                Some(&err.to_string()),
            );
        } else {
            self.update_command_status(TransportStatus::Idle, None);
        }

        Ok(Vec::new())
    }
}

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("host", SetupKind::Hostname).required(),
    SetupField::new("http_port", SetupKind::Port)
        .with_default("8080")
        .with_range(1.0, 65535.0),
    SetupField::new("ws_port", SetupKind::Port)
        .with_default("8081")
        .with_range(1.0, 65535.0),
];
