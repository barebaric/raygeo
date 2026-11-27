//! The Smoothie session: a minimal ping-pong session for
//! Smoothieware over telnet or serial.
//!
//! Port of Rayforge's `SmoothieDriver` protocol behavior.  Smoothie
//! speaks Grbl-style `<...>` status reports in response to `?`, so
//! this session deliberately reuses the GRBL parser and error
//! tables — sanctioned cross-protocol imports; see `mod.rs` for
//! the justification.  Commands use the same strict ping-pong as the minimal
//! GRBL session: send one line, wait for its `ok`/`error:`
//! acknowledgement, then send the next.
//!
//! Differences from the GRBL sessions: the connection is considered
//! established as soon as the transport answers a status poll (the
//! Smoothie welcome banner is firmware-specific and unused), and
//! cancel sends Ctrl+C instead of a soft reset.
//!
//! The ping-pong mechanics (pending acknowledgements, connection
//! loop, job streaming, cancel, safety shutdown) are shared with the
//! other line sessions via [`core::line_driver`]; this module only
//! holds the Smoothie-specific dispatch and hooks.

use crate::driver::core::driver::{
    info::{caps, DEFAULT_SUPPORTED_WCS},
    ConnectionKind, DriverInfo, DriverMaturity,
};

use crate::driver::core::driver::{SetupField, SetupKind};

use std::borrow::Cow;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use std::collections::HashMap;

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::{
    secs, CancelMode, Driver, DriverError, DriverEvents,
};
use crate::driver::core::line_driver::{
    self, LineDriverBase, PendingStore, PingPongDriver, StopCommand,
};
use crate::driver::core::transport::TransportKind;
use crate::driver::core::types::{DeviceState, DeviceStatus, TransportStatus};

use crate::driver::grbl::parser;

/// Configuration for the Smoothie session.
#[derive(Clone)]
pub struct SmoothieConfig {
    pub transport: TransportKind,
    pub poll_status_while_running: bool,
    pub command_timeout: f64,
    pub handshake_timeout: f64,
    pub handshake_poll_interval: f64,
    pub status_poll_interval: f64,
    pub reconnect_delay: f64,
    pub safety_shutdown_delay: f64,
}

impl SmoothieConfig {
    pub fn new(transport: TransportKind) -> Self {
        Self {
            transport,
            poll_status_while_running: false,
            command_timeout: 30.0,
            handshake_timeout: 15.0,
            handshake_poll_interval: 0.5,
            status_poll_interval: 1.0,
            reconnect_delay: 5.0,
            safety_shutdown_delay: 0.2,
        }
    }
}

/// The Smoothie session: connection lifecycle, status polling and
/// ping-pong command exchange.
pub struct SmoothieDriver {
    base: LineDriverBase,
    config: SmoothieConfig,
}

/// Typed identity and capabilities of this driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "SmoothieDriver",
    connection: ConnectionKind::Telnet,
    maturity: DriverMaturity::Stable,
    caps: caps(
        false, true, false, true, true, true, true, true, false, true, false,
    ),
    uses_gcode: true,
    machine_space_wcs: "G53",
    supported_wcs: DEFAULT_SUPPORTED_WCS,
    config_keys: &["host", "tcp_port"],
    setup: SETUP,
    fingerprint: None,
};

impl SmoothieDriver {
    pub fn new(
        config: SmoothieConfig,
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        let transport = config.transport.build();
        Arc::new(Self {
            base: LineDriverBase::new(
                dialect,
                events,
                transport,
                PendingStore::slot(),
            ),
            config,
        })
    }

    // ---------------------------------------------------------------
    // Handshake / polling
    // ---------------------------------------------------------------

    /// Poll the device with `?` until a status report arrives —
    /// Smoothieware needs a moment after opening the link before it
    /// accepts commands, and silently drops earlier queries.
    async fn await_handshake(&self) -> bool {
        let deadline =
            tokio::time::Instant::now() + secs(self.config.handshake_timeout);
        loop {
            self.base.events.raw_tx("?");
            if let Err(err) = self.base.transport.write(b"?").await {
                log_crate::error!("Connection error during handshake: {err}");
                return false;
            }
            let poll_deadline = tokio::time::Instant::now()
                + secs(self.config.handshake_poll_interval);
            tokio::select! {
                _ = self.base.handshake_received.wait() => return true,
                _ = tokio::time::sleep_until(poll_deadline) => {
                    if tokio::time::Instant::now() >= deadline {
                        return false;
                    }
                }
            }
        }
    }

    async fn status_poll_loop(&self) {
        while self.base.transport.is_open()
            && self.base.keep_running.load(Ordering::SeqCst)
        {
            if self.config.poll_status_while_running
                || !self.base.job_running.load(Ordering::SeqCst)
            {
                let _guard = self.base.cmd_lock.lock().await;
                if self.base.transport.is_open() {
                    self.base.events.raw_tx("?");
                    if let Err(err) = self.base.transport.write(b"?").await {
                        log_crate::warn!(
                            "Connection lost while polling: {err}"
                        );
                        break;
                    }
                }
            }
            tokio::time::sleep(secs(self.config.status_poll_interval)).await;
        }
    }

    // ---------------------------------------------------------------
    // RX handling
    // ---------------------------------------------------------------

    fn handle_error_line(&self, line: &str) {
        let code = line.split(':').nth(1).unwrap_or("").trim();
        let error = parser::get_error(code);
        self.base.set_device_error(error);
        self.base.pending.finish_next(Some(line));

        if self.base.is_job_running() {
            log_crate::error!(
                "Device error during job: {line}. Halting stream."
            );
            *self.base.job_exception.lock().unwrap() =
                Some(DriverError::Device(format!("Device error: {line}")));
        }
    }

    fn handle_alarm(&self, code: &str, line: &str) {
        let error =
            crate::driver::grbl::errors::alarm_code_to_device_error(code);
        let snapshot = {
            let mut state = self.base.state.lock().unwrap();
            if state.error.is_none() {
                state.error = Some(error);
            }
            state.status = DeviceStatus::Alarm;
            Some(state.clone())
        };
        if let Some(snapshot) = snapshot {
            self.base.events.state_changed(&snapshot);
        }
        self.base.pending.finish_next(Some(line));

        if self.base.is_job_running() {
            log_crate::error!(
                "Device ALARM during job: {line}. Halting stream."
            );
            *self.base.job_exception.lock().unwrap() =
                Some(DriverError::Device(format!("Device ALARM: {line}")));
        }
    }

    /// Parse a Grbl-style status report (Smoothie emits them for
    /// `?`; see the module docs for why the GRBL parser is reused).
    fn handle_status_report(&self, report: &str) {
        self.base.handshake_received.set();
        let mut state = {
            let current = self.base.state.lock().unwrap();
            parser::parse_state(report, &current, false)
        };

        // While a job runs, 'Idle' between commands is reported as
        // 'Run'; a requested hold owns the pause state.
        if self.base.is_job_running() && state.status == DeviceStatus::Idle {
            state.status = DeviceStatus::Run;
        }
        if self.base.is_holding.load(Ordering::SeqCst)
            && state.status != DeviceStatus::Alarm
        {
            state.status = DeviceStatus::Hold;
        }

        self.base.emit_state_if_changed(state);
    }
}

/// The Smoothie session contract: the protocol hooks, plus the
/// [`Driver`] operations Smoothieware supports, with their real
/// bodies.  Everything shared streams through the
/// [`PingPongDriver`] supertrait.
#[async_trait]
impl PingPongDriver for SmoothieDriver {
    fn base(&self) -> &LineDriverBase {
        &self.base
    }

    fn handle_line(&self, line: &str) {
        if line == "ok" {
            self.base.events.raw_rx("ok");
            self.base.pending.finish_next(Some("ok"));
            return;
        }
        if line.starts_with("error:") {
            self.base.events.raw_rx(line);
            self.handle_error_line(line);
            return;
        }
        if line.starts_with('<') && line.ends_with('>') {
            self.base.events.raw_rx(line);
            self.handle_status_report(line);
            return;
        }
        if let Some(code) = line.strip_prefix("ALARM:") {
            self.base.events.raw_rx(line);
            self.handle_alarm(code.trim(), line);
            return;
        }
        self.base.events.raw_rx(line);
        self.base.pending.collect(line);
    }

    async fn handshake(&self) -> bool {
        self.await_handshake().await
    }

    async fn poll_loop(&self) {
        self.status_poll_loop().await;
    }

    fn on_handshake_failed(&self) {
        log_crate::warn!(
            "No response from device. Port may be a phantom connection."
        );
        self.base.connection_status(
            TransportStatus::Error,
            Some("No response from device"),
        );
    }

    async fn ping_pong(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        let _guard = self.base.cmd_lock.lock().await;
        if !self.base.transport.is_open() {
            return Err(DriverError::Connection(
                "Transport not connected".to_string(),
            ));
        }
        self.base
            .await_ack(command, secs(self.config.command_timeout))
            .await
    }

    fn line_timeout(&self, _estimate: Option<f64>) -> Duration {
        secs(self.config.command_timeout)
    }

    fn job_log_label(&self) -> &'static str {
        "Smoothie"
    }

    fn prep_job_line<'a>(&'a self, raw: &'a str) -> Cow<'a, str> {
        Cow::Owned(parser::strip_gcode_comments(raw))
    }

    fn safety_shutdown_delay(&self) -> Option<Duration> {
        Some(secs(self.config.safety_shutdown_delay))
    }

    fn reconnect_delay(&self) -> Duration {
        secs(self.config.reconnect_delay)
    }

    fn stop_command(&self) -> StopCommand {
        // Smoothieware aborts the job on Ctrl+C.
        StopCommand {
            bytes: b"\x03",
            log: "Sending Ctrl+C to device.",
        }
    }

    fn not_connected_message(&self) -> &'static str {
        "Transport not connected"
    }
}

/// The Smoothie [`Driver`]: the shared ping-pong operations call
/// the skeleton's free functions; the Smoothie-specific operations
/// carry their real bodies here.
#[async_trait]
impl Driver for SmoothieDriver {
    fn info(&self) -> &'static crate::driver::core::driver::DriverInfo {
        &INFO
    }

    fn state(&self) -> DeviceState {
        self.base.state()
    }

    fn is_job_running(&self) -> bool {
        self.base.is_job_running()
    }

    fn resource_uri(&self) -> Option<String> {
        self.base.resource_uri()
    }

    fn update_dialect(&self, dialect: Dialect) {
        self.base.update_dialect(dialect);
    }

    fn abort_background_tasks_now(&self) {
        self.base.abort_background_tasks_now();
    }

    async fn connect(self: Arc<Self>) -> Result<(), DriverError> {
        line_driver::connect(&self).await;
        Ok(())
    }

    async fn cleanup(&self) {
        self.base.cleanup().await;
    }

    async fn run(
        self: Arc<Self>,
        gcode: &str,
        op_map: &HashMap<usize, i64>,
        op_estimates: &[f64],
    ) -> Result<(), DriverError> {
        line_driver::stream_job(self.as_ref(), gcode, op_map, op_estimates)
            .await;
        Ok(())
    }

    async fn run_raw(
        self: Arc<Self>,
        machine_code: &str,
    ) -> Result<(), DriverError> {
        line_driver::run_raw(self.as_ref(), machine_code).await;
        Ok(())
    }

    async fn cancel(&self, mode: CancelMode) -> Result<(), DriverError> {
        line_driver::cancel(self, mode == CancelMode::Immediate).await
    }

    async fn execute_interactive_command(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        // The acknowledgement and status reports are not reply
        // lines; callers parse `version` and `config-get` values out
        // of what remains.
        let lines = line_driver::execute_command(self, command).await?;
        Ok(lines
            .into_iter()
            .filter(|line| {
                line != "ok" && !(line.starts_with('<') && line.ends_with('>'))
            })
            .collect())
    }

    async fn home(
        &self,
        axes: Option<Vec<String>>,
        _active_wcs: &str,
    ) -> Result<(), DriverError> {
        line_driver::home(self, axes).await
    }

    async fn move_to(
        &self,
        speed: f64,
        x: f64,
        y: f64,
        _z: Option<f64>,
    ) -> Result<(), DriverError> {
        line_driver::move_to(self, speed, x, y).await
    }

    async fn jog(
        &self,
        speed: f64,
        deltas: &[(String, f64)],
    ) -> Result<(), DriverError> {
        line_driver::jog(self, speed, deltas).await
    }

    async fn select_tool(&self, tool_number: i64) -> Result<(), DriverError> {
        line_driver::select_tool(self, tool_number).await
    }

    async fn set_power(&self, power: Option<f64>) -> Result<(), DriverError> {
        line_driver::set_power(self, power).await
    }

    async fn set_focus_power(
        &self,
        power: Option<f64>,
    ) -> Result<(), DriverError> {
        line_driver::set_focus_power(self, power).await
    }

    async fn set_hold(&self, hold: bool) -> Result<(), DriverError> {
        self.base.is_holding.store(hold, Ordering::SeqCst);
        if !self.base.transport.is_open() {
            return Err(DriverError::Connection(
                "Transport not connected".to_string(),
            ));
        }
        self.base
            .transport
            .write(if hold { b"!" } else { b"~" })
            .await?;
        let desired = if hold {
            DeviceStatus::Hold
        } else if self.base.is_job_running() {
            DeviceStatus::Run
        } else {
            DeviceStatus::Idle
        };
        let snapshot = {
            let mut state = self.base.state.lock().unwrap();
            if state.status != desired {
                state.status = desired;
                Some(state.clone())
            } else {
                None
            }
        };
        if let Some(snapshot) = snapshot {
            self.base.events.state_changed(&snapshot);
        }
        Ok(())
    }

    async fn clear_alarm(&self) -> Result<(), DriverError> {
        let dialect = self.base.dialect();
        let response =
            line_driver::execute_command(self, &dialect.clear_alarm).await?;
        let has_error = response.iter().any(|line| line.starts_with("error:"));
        if !has_error {
            self.base.clear_device_error();
        }
        Ok(())
    }

    async fn set_wcs_offset(
        &self,
        wcs_slot: &str,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> Result<(), DriverError> {
        let p_num = parser::gcode_to_p_number(wcs_slot).ok_or_else(|| {
            DriverError::Other(format!("Invalid WCS slot: {wcs_slot}"))
        })?;
        let dialect = self.base.dialect();
        let cmd = dialect
            .format_wcs_offset(p_num, x, y, z)
            .map_err(DriverError::Other)?;
        line_driver::execute_command(self, &cmd).await?;
        Ok(())
    }
}

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("host", SetupKind::Hostname).required(),
    SetupField::new("tcp_port", SetupKind::Port)
        .with_default("23")
        .with_range(1.0, 65535.0),
];
