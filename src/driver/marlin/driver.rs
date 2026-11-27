//! The Marlin session: a minimal ping-pong session for Marlin
//! firmware over serial.
//!
//! Port of Rayforge's `MarlinSerialDriver` protocol behavior.  The
//! device announces itself by sending output unprompted — stock
//! Marlin prints a `start` line, but custom forks may emit only a
//! boot banner or ack with bare `ok`; a port that stays silent
//! marks a phantom.  Positions are polled with `M114` instead of `?`
//! status reports, and there is no feed-hold support — pausing is
//! the caller's concern.  Commands use the same strict ping-pong as
//! the other minimal sessions: send one line, wait for its
//! `ok`/`error:` acknowledgement, then send the next.
//!
//! Unlike the Python driver this was ported from, boot and `echo:`
//! lines are collected as response lines of the pending command, so
//! M115/M149 probes actually receive them.
//!
//! Positions are reported in device units, unconverted; unit
//! conversion stays with the caller (see
//! [`parser::detect_unit_system_from_m149`]).
//!
//! The ping-pong mechanics (pending acknowledgements, connection
//! loop, job streaming, cancel, safety shutdown) are shared with the
//! other line sessions via [`core::line_driver`]; this module only
//! holds the Marlin-specific dispatch, the wire-ordered FIFO
//! pending queue and the M114 position polling.

use crate::driver::core::driver::{
    info::{caps, DEFAULT_SUPPORTED_WCS},
    ConnectionKind, DriverInfo, DriverMaturity, ProbeFingerprint,
};

use crate::driver::core::driver::{SetupField, SetupKind};

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use crate::driver::core::dialect::Dialect;
use crate::driver::core::driver::{
    secs, AsyncEvent, CancelMode, Driver, DriverError, DriverEvents,
};
use crate::driver::core::line_driver::{
    self, LineDriverBase, PendingStore, PingPongDriver, StopCommand,
};
use crate::driver::core::transport::TransportKind;
use crate::driver::core::types::{
    DeviceState, DeviceStatus, TransportStatus, UnitSystem,
};

use super::parser;

/// Configuration for the Marlin session.
#[derive(Clone)]
pub struct MarlinConfig {
    pub transport: TransportKind,
    pub poll_status_while_running: bool,
    pub command_timeout: f64,
    pub handshake_timeout: f64,
    pub handshake_poll_interval: f64,
    pub status_poll_interval: f64,
    pub reconnect_delay: f64,
    /// Marlin needs a moment after `start` before it accepts
    /// commands reliably.
    pub connect_settle: f64,
}

impl MarlinConfig {
    pub fn new(transport: TransportKind) -> Self {
        Self {
            transport,
            poll_status_while_running: false,
            command_timeout: 30.0,
            handshake_timeout: 5.0,
            handshake_poll_interval: 0.5,
            status_poll_interval: 2.0,
            reconnect_delay: 5.0,
            connect_settle: 1.0,
        }
    }
}

/// The Marlin session: connection lifecycle, M114 position polling
/// and ping-pong command exchange.
pub struct MarlinSerialDriver {
    base: LineDriverBase,
    config: MarlinConfig,
    /// Set once the device announced itself; commands must not be
    /// sent before that: the boot banner's trailing ok would
    /// otherwise resolve a pending command.
    connected: AsyncEvent,
    /// Startup banner lines, used to refine the firmware version in
    /// the device census.
    boot_lines: Mutex<Vec<String>>,
}

/// Typed identity and capabilities of this driver.
pub static INFO: DriverInfo = DriverInfo {
    name: "MarlinSerialDriver",
    connection: ConnectionKind::Serial,
    maturity: DriverMaturity::Stable,
    caps: caps(
        false, true, true, false, true, true, true, true, false, true, false,
    ),
    uses_gcode: true,
    machine_space_wcs: "MACHINE",
    supported_wcs: DEFAULT_SUPPORTED_WCS,
    config_keys: &["port", "baudrate"],
    setup: SETUP,
    fingerprint: Some(ProbeFingerprint {
        command: "M115",
        signature: "FIRMWARE",
    }),
};

impl MarlinSerialDriver {
    pub fn new(
        config: MarlinConfig,
        dialect: Dialect,
        events: Arc<dyn DriverEvents>,
    ) -> Arc<Self> {
        let transport = config.transport.build();
        Arc::new(Self {
            base: LineDriverBase::new(
                dialect,
                events,
                transport,
                PendingStore::queue(),
            ),
            config,
            connected: AsyncEvent::new(),
            boot_lines: Mutex::new(Vec::new()),
        })
    }

    // ---------------------------------------------------------------
    // Handshake / polling
    // ---------------------------------------------------------------

    /// Wait for the device to announce itself.  Booting devices
    /// speak unprompted — with `start`, a banner or an ack — so
    /// this simply waits for the first output line.
    async fn await_handshake(&self) -> bool {
        let deadline =
            tokio::time::Instant::now() + secs(self.config.handshake_timeout);
        tokio::select! {
            _ = self.base.handshake_received.wait() => true,
            _ = tokio::time::sleep_until(deadline) => false,
        }
    }

    async fn position_poll_loop(&self) {
        loop {
            // Poll on an interval, counting from the connection: a
            // freshly booted device gets a moment before the first
            // M114.
            tokio::time::sleep(secs(self.config.status_poll_interval)).await;
            if !self.base.transport.is_open()
                || !self.base.keep_running.load(Ordering::SeqCst)
            {
                break;
            }
            if self.config.poll_status_while_running
                || !self.base.job_running.load(Ordering::SeqCst)
            {
                // A tracked exchange, not fire-and-forget: M114's ok
                // must not resolve a later command's pending
                // acknowledgement.  The FIFO pending queue orders the
                // responses; no cmd_lock is taken so a slow device
                // cannot stall interactive commands.
                if let Err(err) = self.enqueue_and_wait("M114", false).await {
                    log_crate::warn!("Connection lost while polling: {err}");
                    break;
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // RX handling
    // ---------------------------------------------------------------

    /// Accepts the first device output as the handshake.  The
    /// session transmits nothing until the handshake resolves, so
    /// any complete line here is unprompted device output: proof of
    /// a live device.  Stock Marlin announces itself with `start`,
    /// but custom firmware forks may only print a banner or ack
    /// with bare `ok`; requiring the exact line locked those out.
    /// Only a fully silent port is a phantom.
    fn maybe_announce_handshake(&self, line: &str) {
        if self.base.handshake_received.is_set() {
            return;
        }
        if line.trim() != "start" {
            log_crate::debug!(
                "Handshake satisfied by unsolicited output: {line:?}"
            );
        }
        self.base.handshake_received.set();
    }

    fn handle_error_line(&self, line: &str) {
        let message = parser::parse_error_message(line);
        let snapshot = {
            let mut state = self.base.state.lock().unwrap();
            if state.error.is_none() {
                state.error =
                    Some(crate::driver::core::types::DeviceError::new(
                        -1,
                        "Device Error",
                        &message,
                    ));
                Some(state.clone())
            } else {
                None
            }
        };
        if let Some(snapshot) = snapshot {
            self.base.events.state_changed(&snapshot);
        }
        self.base.pending.finish_next(Some(line));

        if self.base.is_job_running() {
            log_crate::error!(
                "Device error during job: {line}. Halting stream."
            );
            *self.base.job_exception.lock().unwrap() =
                Some(DriverError::Device(format!("Device error: {line}")));
        }
    }

    fn handle_position_report(&self, line: &str) {
        let Some((x, y, z)) = parser::parse_m114_line(line) else {
            return;
        };
        let snapshot = {
            let mut state = self.base.state.lock().unwrap();
            state.machine_pos = vec![Some(x), Some(y), Some(z)];
            state.work_pos = vec![Some(x), Some(y), Some(z)];
            if state.status == DeviceStatus::Unknown {
                state.status = DeviceStatus::Idle;
            }
            Some(state.clone())
        };
        if let Some(snapshot) = snapshot {
            self.base.events.state_changed(&snapshot);
        }
    }

    // ---------------------------------------------------------------
    // Command exchange
    // ---------------------------------------------------------------

    /// Enqueue *command* and await its acknowledgement.  Outstanding
    /// commands form a FIFO queue matching the device's wire-ordered
    /// responses, so a status poll may be in flight while a user
    /// command waits without either stealing the other's
    /// acknowledgement.
    ///
    /// With `serialize` the write is excluded against other user
    /// commands; the status poll passes `false` so a slow device
    /// cannot stall interactive commands while its own response is
    /// pending.
    async fn enqueue_and_wait(
        &self,
        command: &str,
        serialize: bool,
    ) -> Result<Vec<String>, DriverError> {
        let _guard = if serialize {
            Some(self.base.cmd_lock.lock().await)
        } else {
            None
        };
        if !self.base.transport.is_open() {
            return Err(DriverError::Connection(
                "Serial transport not connected".to_string(),
            ));
        }
        self.base
            .await_ack(command, secs(self.config.command_timeout))
            .await
    }
}

impl MarlinSerialDriver {
    /// Boot banner lines received from the device.
    pub fn boot_lines(&self) -> Vec<String> {
        self.boot_lines.lock().unwrap().clone()
    }
}

#[async_trait]
impl PingPongDriver for MarlinSerialDriver {
    fn base(&self) -> &LineDriverBase {
        &self.base
    }

    fn handle_line(&self, line: &str) {
        if parser::is_boot_message(line) {
            self.boot_lines.lock().unwrap().push(line.to_string());
        }
        self.maybe_announce_handshake(line);
        if parser::is_ok(line) {
            self.base.events.raw_rx(line);
            self.base.pending.finish_next(Some(line));
            return;
        }
        if parser::is_error(line) {
            self.base.events.raw_rx(line);
            self.handle_error_line(line);
            return;
        }
        if parser::is_m114_position(line) {
            self.base.events.raw_rx(line);
            self.handle_position_report(line);
            self.base.pending.collect(line);
            return;
        }
        self.base.events.raw_rx(line);
        self.base.pending.collect(line);
    }

    async fn handshake(&self) -> bool {
        self.await_handshake().await
    }

    async fn on_established(&self) {
        // The device needs a moment after announcing itself.
        tokio::time::sleep(secs(self.config.connect_settle)).await;
        // Commands must not be sent before the device announced
        // itself: the boot banner's trailing ok would otherwise
        // resolve a pending command.
        self.connected.set();
        self.base
            .connection_status(TransportStatus::Connected, None);
    }

    fn on_handshake_failed(&self) {
        log_crate::warn!(
            "Device sent no output. Port may be a phantom COM port \
             without a connected device."
        );
        self.base.connection_status(
            TransportStatus::Error,
            Some("No response from device"),
        );
    }

    async fn poll_loop(&self) {
        self.position_poll_loop().await;
    }

    async fn ping_pong(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        if !self.connected.is_set() {
            // Wait out the handshake: sending earlier would let the
            // boot banner's trailing ok resolve this command.
            let wait_deadline = tokio::time::Instant::now()
                + secs(
                    self.config.handshake_timeout
                        + self.config.connect_settle
                        + 2.0,
                );
            if tokio::time::timeout_at(wait_deadline, self.connected.wait())
                .await
                .is_err()
            {
                return Err(DriverError::Connection(
                    "device did not announce itself".to_string(),
                ));
            }
        }
        if !self.base.transport.is_open() {
            return Err(DriverError::Connection(
                "Serial transport not connected".to_string(),
            ));
        }
        self.enqueue_and_wait(command, true).await
    }

    fn line_timeout(&self, _estimate: Option<f64>) -> Duration {
        secs(self.config.command_timeout)
    }

    fn job_log_label(&self) -> &'static str {
        "Marlin"
    }

    fn stop_job_on_alarm(&self) -> bool {
        false
    }

    fn safety_shutdown_delay(&self) -> Option<Duration> {
        None
    }

    fn reconnect_delay(&self) -> Duration {
        secs(self.config.reconnect_delay)
    }

    fn stop_command(&self) -> StopCommand {
        StopCommand {
            bytes: b"M410\n",
            log: "Sending M410 (Quick Stop) to device.",
        }
    }

    fn not_connected_message(&self) -> &'static str {
        "Serial transport not connected"
    }
}

/// The Marlin [`Driver`]: the shared ping-pong operations call the
/// skeleton's free functions; the Marlin-specific operations carry
/// their real bodies here.
#[async_trait]
impl Driver for MarlinSerialDriver {
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
        // The terminating `ok` is the acknowledgement, not part of
        // the reply callers parse (M115/M211/M503/M149).
        let lines = line_driver::execute_command(self, command).await?;
        Ok(lines.into_iter().filter(|line| line != "ok").collect())
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
        line_driver::jog(self, speed, deltas).await?;
        // The jog template selects relative mode; Marlin's job
        // streaming expects absolute moves, so restore it.
        line_driver::execute_command(self, "G90").await?;
        Ok(())
    }

    async fn set_hold(&self, _hold: bool) -> Result<(), DriverError> {
        log_crate::warn!(
            "Marlin does not support reliable pause/resume on 8-bit              boards; set_hold is a best-effort no-op."
        );
        Ok(())
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

    async fn clear_alarm(&self) -> Result<(), DriverError> {
        let dialect = self.base.dialect();
        let response =
            line_driver::execute_command(self, &dialect.clear_alarm).await?;
        let has_error = response.iter().any(|line| parser::is_error(line));
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
        let p_num = crate::driver::grbl::parser::gcode_to_p_number(wcs_slot)
            .ok_or_else(|| {
                DriverError::Other(format!("Invalid WCS slot: {wcs_slot}"))
            })?;
        let dialect = self.base.dialect();
        let cmd = dialect
            .format_wcs_offset(p_num, x, y, z)
            .map_err(DriverError::Other)?;
        line_driver::execute_command(self, &cmd).await?;
        Ok(())
    }

    async fn detect_unit_system(
        &self,
    ) -> Result<Option<UnitSystem>, DriverError> {
        let lines = match line_driver::execute_command(self, "M149").await {
            Ok(lines) => lines,
            Err(err) => {
                log_crate::warn!("Unit system detection failed: {err}");
                return Ok(None);
            }
        };
        Ok(parser::detect_unit_system_from_m149(&lines))
    }
}

/// Connection-setup form for this driver.
pub static SETUP: &[SetupField] = &[
    SetupField::new("port", SetupKind::SerialPort).required(),
    SetupField::new("baudrate", SetupKind::Baudrate).with_default("115200"),
];
