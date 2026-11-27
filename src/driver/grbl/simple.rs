//! A deliberately minimal GRBL session: ping-pong only.
//!
//! Port of Rayforge's `GrblSerialSimpleDriver`: send one line, wait
//! for its `ok`/`error:` acknowledgement, then send the next.  There
//! is no character-counting flow control, no stall detection, no
//! deadlock recovery and no buffer management — the simplicity is
//! the point: this session is for hardware where the full driver
//! misbehaves, and it must not inherit the full driver's complexity.
//!
//! It shares the ping-pong mechanics (pending acknowledgements,
//! connection loop, job streaming, cancel, safety shutdown) with the
//! other line sessions via [`core::line_driver`], and the leaf
//! building blocks with the full session: transports, the parser,
//! the dialect templates, the value types and the [`DriverEvents`]
//! notification trait.  Behavioural contract difference: interactive
//! commands return an `error:` line as their last response line
//! instead of raising (the driver contract), while an error
//! response during a job halts the stream and triggers the safety
//! shutdown.

use crate::driver::core::driver::{
    info::{caps, DEFAULT_SUPPORTED_WCS},
    ConnectionKind, DriverInfo, DriverMaturity,
};

use crate::driver::core::driver::{SetupField, SetupKind};

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use super::parser;
use crate::driver::core::dialect::{format_template, Arg, Dialect};
use crate::driver::core::driver::{
    secs, CancelMode, Driver, DriverError, DriverEvents,
};
use crate::driver::core::line_driver::{
    self, LineDriverBase, PendingStore, PingPongDriver, StopCommand,
};
use crate::driver::core::transport::TransportKind;
use crate::driver::core::types::{
    DeviceState, DeviceStatus, TransportStatus, UnitSystem,
};

/// Configuration for the simple ping-pong session.
#[derive(Clone)]
pub struct GrblSimpleConfig {
    pub poll_status_while_running: bool,
    pub command_timeout: f64,
    pub homing_timeout: f64,
    pub stall_timeout_min: f64,
    pub stall_timeout_max: f64,
    pub stall_timeout_safety_factor: f64,
    pub stall_timeout_default: f64,
    pub handshake_timeout: f64,
    pub handshake_poll_interval: f64,
    pub status_poll_interval: f64,
    pub reconnect_delay: f64,
    pub safety_shutdown_delay: f64,
}

impl GrblSimpleConfig {
    pub fn new() -> Self {
        Self {
            poll_status_while_running: false,
            command_timeout: 30.0,
            homing_timeout: 0.0,
            stall_timeout_min: 5.0,
            stall_timeout_max: 120.0,
            stall_timeout_safety_factor: 3.0,
            stall_timeout_default: 30.0,
            handshake_timeout: 3.0,
            handshake_poll_interval: 0.5,
            status_poll_interval: 1.0,
            reconnect_delay: 5.0,
            safety_shutdown_delay: 0.2,
        }
    }
}

impl Default for GrblSimpleConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// The simple session: connection lifecycle, status polling and
/// ping-pong command exchange, all in one small state machine.
pub struct GrblSerialSimpleDriver {
    base: LineDriverBase,
    config: GrblSimpleConfig,
    report_in_inches: AtomicBool,
}

/// Typed identity and capabilities of this driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "GrblSerialSimpleDriver",
    connection: ConnectionKind::Serial,
    maturity: DriverMaturity::Stable,
    caps: caps(
        true, true, true, true, true, true, true, true, false, true, false,
    ),
    uses_gcode: true,
    machine_space_wcs: "G53",
    supported_wcs: DEFAULT_SUPPORTED_WCS,
    config_keys: &["port", "baudrate"],
    setup: SETUP,
    fingerprint: None,
};

impl GrblSerialSimpleDriver {
    pub fn new(
        transport: TransportKind,
        config: GrblSimpleConfig,
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        let transport = transport.build();
        Arc::new(Self {
            base: LineDriverBase::new(
                dialect,
                events,
                transport,
                PendingStore::slot(),
            ),
            config,
            report_in_inches: AtomicBool::new(false),
        })
    }

    // ---------------------------------------------------------------
    // Handshake / polling
    // ---------------------------------------------------------------

    /// Poll the device with `?` until it answers, bounded by the
    /// handshake timeout.  Devices that are still booting silently
    /// drop queries sent before their serial stream is ready.
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

    fn handle_status_report(&self, report: &str) {
        self.base.handshake_received.set();
        let report_in_inches = self.report_in_inches.load(Ordering::SeqCst);
        let mut state = {
            let current = self.base.state.lock().unwrap();
            parser::parse_state(report, &current, report_in_inches)
        };

        // If a job is active, 'Idle' between commands is reported as
        // 'Run'; while a hold was requested, the session owns the
        // pause state and forces 'Hold'.
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

    fn handle_error_line(&self, line: &str) {
        let code = line.split(':').nth(1).unwrap_or("").trim();
        self.base.set_device_error(parser::get_error(code));
        self.base.pending.finish_next(Some(line));

        if self.base.is_job_running() {
            log_crate::error!("GRBL error during job: {line}. Halting stream.");
            *self.base.job_exception.lock().unwrap() =
                Some(DriverError::Device(format!("GRBL error: {line}")));
        }
    }

    fn handle_alarm(&self, code: &str, line: &str) {
        let error = super::errors::alarm_code_to_device_error(code);
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
            log_crate::error!("GRBL ALARM during job: {line}. Halting stream.");
            *self.base.job_exception.lock().unwrap() =
                Some(DriverError::Device(format!("GRBL ALARM: {line}")));
        }
    }

    // ---------------------------------------------------------------
    // Command exchange
    // ---------------------------------------------------------------

    /// Send a realtime command (?, ~, !) directly: the firmware
    /// executes these on receipt and never acknowledges them.
    async fn send_realtime(&self, command: &str) -> Result<(), DriverError> {
        if !self.base.transport.is_open() {
            return Err(DriverError::Connection(
                "Serial transport disconnected".to_string(),
            ));
        }
        self.base.events.raw_tx(command);
        self.base.transport.write(command.as_bytes()).await?;
        Ok(())
    }
}

#[async_trait]
impl PingPongDriver for GrblSerialSimpleDriver {
    fn base(&self) -> &LineDriverBase {
        &self.base
    }

    fn handle_line(&self, line: &str) {
        if line == "ok" {
            self.base.events.raw_rx("ok");
            self.base.pending.finish_next(None);
            return;
        }
        if line.starts_with("error:") {
            self.base.events.raw_rx(line);
            self.handle_error_line(line);
            return;
        }
        if line.starts_with('<') && line.ends_with('>') {
            self.handle_status_report(line);
            return;
        }
        if let Some(code) = line.strip_prefix("ALARM:") {
            self.base.events.raw_rx(line);
            self.handle_alarm(code.trim(), line);
            return;
        }
        if line.starts_with("Grbl ") || line.starts_with("GrblHAL ") {
            self.base.events.raw_rx(line);
            self.base.handshake_received.set();
            return;
        }
        self.base.events.raw_rx(line);
        self.base.pending.collect(line);
    }

    async fn on_opened(&self) {
        self.base
            .connection_status(TransportStatus::Connected, None);
    }

    async fn on_established(&self) {
        // "Connected" was already reported when the transport
        // opened — before the handshake.
    }

    async fn handshake(&self) -> bool {
        self.await_handshake().await
    }

    fn on_handshake_failed(&self) {
        log_crate::warn!("Handshake timeout. Retrying connection.");
    }

    async fn poll_loop(&self) {
        self.status_poll_loop().await;
    }

    async fn ping_pong(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        let timeout = secs(self.config.command_timeout);
        self.ping_pong_with_timeout(command, timeout).await
    }

    /// Send one command and wait up to *timeout* for its
    /// acknowledgement.  Homing needs a budget measured in minutes on
    /// large machines, while normal commands keep the ping-pong
    /// timeout.
    async fn ping_pong_with_timeout(
        &self,
        command: &str,
        timeout: Duration,
    ) -> Result<Vec<String>, DriverError> {
        let _guard = self.base.cmd_lock.lock().await;
        if !self.base.transport.is_open() {
            return Err(DriverError::Connection(
                "Serial transport not connected".to_string(),
            ));
        }
        self.base.await_ack(command, timeout).await
    }

    fn line_timeout(&self, estimate: Option<f64>) -> Duration {
        match estimate {
            Some(seconds) => {
                secs((seconds * self.config.stall_timeout_safety_factor).clamp(
                    self.config.stall_timeout_min,
                    self.config.stall_timeout_max,
                ))
            }
            None => secs(self.config.stall_timeout_default),
        }
    }

    fn job_log_label(&self) -> &'static str {
        "ping-pong"
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
        StopCommand {
            bytes: b"\x18",
            log: "Sending Soft Reset (Ctrl-X) to device.",
        }
    }

    fn not_connected_message(&self) -> &'static str {
        "Serial transport not connected"
    }
}

/// The simple session's [`Driver`]: the shared ping-pong
/// operations call the skeleton's free functions; the GRBL-specific
/// operations carry their real bodies here.
#[async_trait]
impl Driver for GrblSerialSimpleDriver {
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
        let lines: Vec<String> = machine_code
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        let (gcode, realtime) = parser::split_realtime_commands(&lines);
        for line in &realtime {
            if let Err(err) = self.send_realtime(line).await {
                log_crate::error!("Realtime command failed: {err}");
                return Ok(());
            }
        }
        if gcode.is_empty() {
            return Ok(());
        }
        line_driver::stream_job(
            self.as_ref(),
            &gcode.join("\n"),
            &HashMap::new(),
            &[],
        )
        .await;
        Ok(())
    }

    async fn cancel(&self, mode: CancelMode) -> Result<(), DriverError> {
        line_driver::cancel(self, mode == CancelMode::Immediate).await
    }

    async fn execute_interactive_command(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        line_driver::execute_command(self, command).await
    }

    async fn home(
        &self,
        axes: Option<Vec<String>>,
        active_wcs: &str,
    ) -> Result<(), DriverError> {
        let dialect = self.base.dialect();
        let timeout = secs(if self.config.homing_timeout > 0.0 {
            self.config.homing_timeout
        } else {
            self.config.command_timeout
        });
        match axes {
            None => {
                line_driver::execute_command_with_timeout(
                    self,
                    &dialect.home_all,
                    timeout,
                )
                .await?;
            }
            Some(axes) => {
                for axis in axes {
                    let cmd = format_template(
                        &dialect.home_axis,
                        &[("axis_letter", Arg::Str(axis))],
                    )
                    .map_err(DriverError::Other)?;
                    line_driver::execute_command_with_timeout(
                        self, &cmd, timeout,
                    )
                    .await?;
                }
            }
        }

        // Some Grbl versions forget the G54 offset after homing;
        // toggling to another WCS and back re-activates it.
        let temp_wcs = if active_wcs == "G54" { "G55" } else { "G54" };
        line_driver::execute_command(self, "G4 P0.01").await?;
        line_driver::execute_command(self, temp_wcs).await?;
        line_driver::execute_command(self, active_wcs).await?;
        self.base.clear_device_error();
        Ok(())
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
        if !self.base.is_job_running() {
            self.base.is_cancelled.store(false, Ordering::SeqCst);
        }
        self.send_realtime(if hold { "!" } else { "~" }).await?;
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

    async fn clear_alarm(&self) -> Result<(), DriverError> {
        let dialect = self.base.dialect();
        let response =
            line_driver::execute_command(self, &dialect.clear_alarm).await?;
        let has_error = response.iter().any(|line| {
            line.starts_with("error:") || line.starts_with("Error:")
        });
        if !has_error {
            self.base.clear_device_error();
        }
        Ok(())
    }

    async fn read_settings(
        &self,
    ) -> Result<Vec<(String, String)>, DriverError> {
        let lines = line_driver::execute_command(self, "$$").await?;
        self.report_in_inches
            .store(parser::is_report_in_inches(&lines), Ordering::SeqCst);
        Ok(parser::parse_setting_pairs(&lines))
    }

    async fn write_setting(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), DriverError> {
        line_driver::execute_command(self, &format!("${key}={value}")).await?;
        Ok(())
    }

    async fn read_wcs_offsets(
        &self,
    ) -> Result<HashMap<String, (f64, f64, f64)>, DriverError> {
        let lines = line_driver::execute_command(self, "$#").await?;
        let report_in_inches = self.report_in_inches.load(Ordering::SeqCst);
        let mut offsets = HashMap::new();
        for line in &lines {
            if let Some((slot, (x, y, z))) = parser::parse_wcs_line(line) {
                let pos = if report_in_inches {
                    (
                        crate::driver::core::types::inches_to_mm(x),
                        crate::driver::core::types::inches_to_mm(y),
                        crate::driver::core::types::inches_to_mm(z),
                    )
                } else {
                    (x, y, z)
                };
                offsets.insert(slot, pos);
            }
        }
        self.base.events.wcs_updated(&offsets);
        Ok(offsets)
    }

    async fn read_parser_state(&self) -> Result<Option<String>, DriverError> {
        let lines = line_driver::execute_command(self, "$G").await?;
        Ok(parser::parse_grbl_parser_state(&lines))
    }

    async fn run_probe_cycle(
        &self,
        axis_letter: &str,
        max_travel: f64,
        feed_rate: f64,
    ) -> Result<Option<(f64, f64, f64)>, DriverError> {
        let axis_letter = axis_letter.to_uppercase();
        let dialect = self.base.dialect();
        let cmd = format_template(
            &dialect.probe_cycle,
            &[
                ("axis_letter", Arg::Str(axis_letter.clone())),
                ("max_travel", Arg::Length(max_travel)),
                ("feed_rate", Arg::Speed(feed_rate)),
            ],
        )
        .map_err(DriverError::Other)?;

        self.base
            .events
            .probe_status(&format!("Probing {axis_letter}..."));
        let lines = match line_driver::execute_command(self, &cmd).await {
            Ok(lines) => lines,
            Err(_) => {
                self.base.events.probe_status("Probe failed: Timed out");
                return Ok(None);
            }
        };

        for line in &lines {
            if let Some(((x, y, z), success)) = parser::parse_probe_line(line) {
                if success {
                    let pos = if self.report_in_inches.load(Ordering::SeqCst) {
                        (
                            crate::driver::core::types::inches_to_mm(x),
                            crate::driver::core::types::inches_to_mm(y),
                            crate::driver::core::types::inches_to_mm(z),
                        )
                    } else {
                        (x, y, z)
                    };
                    self.base.events.probe_status(&format!(
                        "Probe triggered at ({}, {}, {})",
                        pos.0, pos.1, pos.2
                    ));
                    return Ok(Some(pos));
                }
            }
        }
        self.base.events.probe_status("Probe failed");
        Ok(None)
    }

    async fn detect_unit_system(
        &self,
    ) -> Result<Option<UnitSystem>, DriverError> {
        let lines = match line_driver::execute_command(self, "$$").await {
            Ok(lines) => lines,
            Err(err) => {
                log_crate::warn!("Unit system detection failed: {err}");
                return Ok(None);
            }
        };
        self.report_in_inches
            .store(parser::is_report_in_inches(&lines), Ordering::SeqCst);
        Ok(parser::detect_unit_system_from_settings(&lines))
    }
}

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("port", SetupKind::SerialPort).required(),
    SetupField::new("baudrate", SetupKind::Baudrate).with_default("115200"),
];
