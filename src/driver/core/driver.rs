//! Driver primitives shared by the protocol drivers: the typed
//! metadata, the event notification trait, the error type, the async
//! latch and small shared helpers.

pub mod info;

pub use info::{
    Capabilities, ConnectionKind, DriverInfo, DriverMaturity, ProbeFingerprint,
    SetupField, SetupKind,
};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use crate::driver::core::dialect::Dialect;
use crate::driver::core::transport::TransportError;
use crate::driver::core::types::{DeviceState, TransportStatus, UnitSystem};

/// WCS slot → (x, y, z) offsets in machine units.
pub type WcsOffsets = HashMap<String, (f64, f64, f64)>;

/// Converts a (possibly negative) seconds value into a non-negative
/// [`Duration`]; shared by every session's timeout configuration.
pub(crate) fn secs(v: f64) -> Duration {
    Duration::from_secs_f64(v.max(0.0))
}

/// One notification from a device session.
#[derive(Debug, Clone)]
pub enum DriverEvent {
    StateChanged(DeviceState),
    ConnectionStatus {
        status: TransportStatus,
        message: Option<String>,
    },
    CommandStatus {
        status: TransportStatus,
        message: Option<String>,
    },
    JobFinished,
    JobError {
        message: String,
    },
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
    CommandDone(i64),
    /// A complete line received from the device, including status
    /// reports, `ok`/`error:` acknowledgements and `$` replies.
    RawRx(String),
    /// A complete line sent to the device (interactive commands and
    /// realtime bytes; not bulk job streaming).
    RawTx(String),
    /// A chunk of raw bytes received from the device (binary
    /// protocols such as Ruida).
    RawRxBytes(Vec<u8>),
    /// A chunk of raw bytes sent to the device (binary protocols
    /// such as Ruida).
    RawTxBytes(Vec<u8>),
}

/// Notifications emitted by a device session, implemented by the
/// Python bindings (or by embedders).
///
/// [`emit`](Self::emit) is the canonical entry point; the named
/// helpers below are adapters so session code stays readable.
pub trait DriverEvents: Send + Sync {
    fn emit(&self, event: DriverEvent);

    fn state_changed(&self, state: &DeviceState) {
        self.emit(DriverEvent::StateChanged(state.clone()));
    }

    fn connection_status(
        &self,
        status: TransportStatus,
        message: Option<&str>,
    ) {
        self.emit(DriverEvent::ConnectionStatus {
            status,
            message: message.map(str::to_string),
        });
    }

    fn command_status(&self, status: TransportStatus, message: Option<&str>) {
        self.emit(DriverEvent::CommandStatus {
            status,
            message: message.map(str::to_string),
        });
    }

    fn job_finished(&self) {
        self.emit(DriverEvent::JobFinished);
    }

    fn job_error(&self, message: &str) {
        self.emit(DriverEvent::JobError {
            message: message.to_string(),
        });
    }

    fn probe_status(&self, message: &str) {
        self.emit(DriverEvent::ProbeStatus {
            message: message.to_string(),
        });
    }

    fn wcs_updated(&self, offsets: &HashMap<String, (f64, f64, f64)>) {
        self.emit(DriverEvent::WcsUpdated {
            offsets: offsets.clone(),
        });
    }

    fn config_changed(&self, key: &str, value: i64) {
        self.emit(DriverEvent::ConfigChanged {
            key: key.to_string(),
            value,
        });
    }

    fn command_done(&self, op_index: i64) {
        self.emit(DriverEvent::CommandDone(op_index));
    }

    fn raw_rx(&self, line: &str) {
        self.emit(DriverEvent::RawRx(line.to_string()));
    }

    fn raw_tx(&self, line: &str) {
        self.emit(DriverEvent::RawTx(line.to_string()));
    }

    fn raw_rx_bytes(&self, data: &[u8]) {
        self.emit(DriverEvent::RawRxBytes(data.to_vec()));
    }

    fn raw_tx_bytes(&self, data: &[u8]) {
        self.emit(DriverEvent::RawTxBytes(data.to_vec()));
    }
}

/// No-op event sink.
pub struct NoopEvents;

impl DriverEvents for NoopEvents {
    fn emit(&self, _event: DriverEvent) {}
}

#[derive(thiserror::Error, Debug)]
pub enum DriverError {
    #[error("{0}")]
    Connection(String),
    #[error("{0}")]
    Timeout(String),
    #[error("{0}")]
    Device(String),
    #[error("buffer stall: {0}")]
    BufferStall(String),
    /// The physical resource is already held by another driver.
    #[error("resource {resource} is already in use by {owner}")]
    ResourceBusy { resource: String, owner: String },
    /// The session cannot perform the operation at all (as opposed
    /// to failing at it): the protocol has no such capability, and
    /// the uniform session surface still exposes the method.
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    #[error("{0}")]
    Other(String),
}

impl From<TransportError> for DriverError {
    fn from(err: TransportError) -> Self {
        match err {
            TransportError::Connection(msg) => Self::Connection(msg),
            TransportError::NotConnected => {
                Self::Connection("not connected".to_string())
            }
        }
    }
}

/// The canonical error for a [`Driver`] operation the protocol
/// cannot perform at all.
pub fn unsupported(op: &str) -> DriverError {
    DriverError::Unsupported(format!("{op} is not supported by this protocol"))
}

/// Cancel urgency for [`Driver::cancel`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelMode {
    /// Cooperative stop; the device may finish the current move.
    Graceful,
    /// Immediate stop (quick stop, feed hold or soft reset).
    Immediate,
}

/// The uniform, typed session surface shared by every protocol
/// driver.
///
/// This is the canonical convention the protocol sessions used to
/// follow only by name — `run`/`cancel`/`home` signatures used to
/// drift per protocol.  The native session methods remain the
/// behavior source; these impls adapt them (and their historical
/// signature differences) losslessly.  Operations a protocol cannot
/// perform at all return [`DriverError::Unsupported`].
#[async_trait]
pub trait Driver: Send + Sync {
    /// The driver's typed identity and capabilities.
    fn info(&self) -> &'static DriverInfo;

    /// Open the connection.  Connection problems are reported
    /// through [`DriverEvents::connection_status`], not the return
    /// value.
    async fn connect(self: Arc<Self>) -> Result<(), DriverError>;

    /// Close the connection and stop all background tasks.
    async fn cleanup(&self);

    /// The last reported device state.
    fn state(&self) -> DeviceState;

    /// True while a job streams.
    fn is_job_running(&self) -> bool;

    /// The transport's resource identifier (e.g. `serial:///dev/ttyUSB0`).
    fn resource_uri(&self) -> Option<String>;

    /// Replace the command templates (a no-op for protocols that
    /// take no template data).
    fn update_dialect(&self, dialect: Dialect) {
        let _ = dialect;
    }

    /// Abort every background task without awaiting, for use from
    /// `Drop` where blocking is not allowed.
    fn abort_background_tasks_now(&self);

    /// Execute pre-rendered machine code (no comment stripping, no
    /// op mapping).  Protocols without such a path return
    /// [`DriverError::Unsupported`].
    async fn run_raw(
        self: Arc<Self>,
        machine_code: &str,
    ) -> Result<(), DriverError> {
        let _ = machine_code;
        Err(unsupported("run_raw"))
    }

    /// Stream (or upload) a complete G-code job and wait for it to
    /// finish.  *op_map* maps zero-based G-code line indices to op
    /// indices; *op_estimates* carries the per-op durations in
    /// seconds where the protocol can use them.
    async fn run(
        self: Arc<Self>,
        gcode: &str,
        op_map: &HashMap<usize, i64>,
        op_estimates: &[f64],
    ) -> Result<(), DriverError> {
        let _ = (gcode, op_map, op_estimates);
        Err(unsupported("run"))
    }

    /// Stop the running job.
    async fn cancel(&self, mode: CancelMode) -> Result<(), DriverError>;

    /// Send one interactive command and await its full response.
    async fn execute_interactive_command(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        let _ = command;
        Err(unsupported("execute_interactive_command"))
    }

    /// Send one command and return its response lines.  This is the
    /// non-raising path: protocols that report rejections as lines
    /// (`error:`) or swallow them keep doing so.  Defaults to the
    /// interactive path.
    async fn execute_command(
        &self,
        command: &str,
    ) -> Result<Vec<String>, DriverError> {
        self.execute_interactive_command(command).await
    }

    /// Bytes currently buffered in the device's RX buffer.  `0` when
    /// the protocol has no buffer accounting.
    fn buffer_count(&self) -> usize {
        0
    }

    /// The device's total RX buffer size.  `0` when the protocol has
    /// no buffer accounting.
    fn rx_buffer_size(&self) -> usize {
        0
    }

    /// Commands waiting in the device's planner, if the protocol can
    /// report them.
    fn pending_commands(&self) -> Vec<String> {
        Vec::new()
    }

    /// Home all axes or *axes*; *active_wcs* is the WCS the caller
    /// expects to stay active (some protocols re-assert it).
    async fn home(
        &self,
        axes: Option<Vec<String>>,
        active_wcs: &str,
    ) -> Result<(), DriverError> {
        let _ = (axes, active_wcs);
        Err(unsupported("home"))
    }

    /// Absolute move in the machine's current coordinate frame.
    async fn move_to(
        &self,
        speed: f64,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> Result<(), DriverError> {
        let _ = (speed, x, y, z);
        Err(unsupported("move_to"))
    }

    /// Relative jog by per-axis distances.
    async fn jog(
        &self,
        speed: f64,
        deltas: &[(String, f64)],
    ) -> Result<(), DriverError> {
        let _ = (speed, deltas);
        Err(unsupported("jog"))
    }

    async fn select_tool(&self, tool_number: i64) -> Result<(), DriverError> {
        let _ = tool_number;
        Err(unsupported("select_tool"))
    }

    /// Laser/spindle power; `None` disables the tool.
    async fn set_power(&self, power: Option<f64>) -> Result<(), DriverError> {
        let _ = power;
        Err(unsupported("set_power"))
    }

    /// Laser power used while focusing; `None` disables it.
    async fn set_focus_power(
        &self,
        power: Option<f64>,
    ) -> Result<(), DriverError> {
        let _ = power;
        Err(unsupported("set_focus_power"))
    }

    /// Pause or resume the running job.
    async fn set_hold(&self, hold: bool) -> Result<(), DriverError> {
        let _ = hold;
        Err(unsupported("set_hold"))
    }

    async fn clear_alarm(&self) -> Result<(), DriverError> {
        Err(unsupported("clear_alarm"))
    }

    async fn set_wcs_offset(
        &self,
        wcs_slot: &str,
        x: f64,
        y: f64,
        z: Option<f64>,
    ) -> Result<(), DriverError> {
        let _ = (wcs_slot, x, y, z);
        Err(unsupported("set_wcs_offset"))
    }

    async fn read_settings(
        &self,
    ) -> Result<Vec<(String, String)>, DriverError> {
        Err(unsupported("read_settings"))
    }

    async fn write_setting(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), DriverError> {
        let _ = (key, value);
        Err(unsupported("write_setting"))
    }

    async fn read_wcs_offsets(
        &self,
    ) -> Result<HashMap<String, (f64, f64, f64)>, DriverError> {
        Err(unsupported("read_wcs_offsets"))
    }

    async fn read_parser_state(&self) -> Result<Option<String>, DriverError> {
        Err(unsupported("read_parser_state"))
    }

    /// Probe along *axis_letter*; `Ok(None)` = no trigger reported.
    async fn run_probe_cycle(
        &self,
        axis_letter: &str,
        max_travel: f64,
        feed_rate: f64,
    ) -> Result<Option<(f64, f64, f64)>, DriverError> {
        let _ = (axis_letter, max_travel, feed_rate);
        Err(unsupported("run_probe_cycle"))
    }

    async fn detect_unit_system(
        &self,
    ) -> Result<Option<UnitSystem>, DriverError> {
        Err(unsupported("detect_unit_system"))
    }
}

/// Implements [`Driver`] for a driver struct that embeds an engine
/// and forwards every operation to it.
///
/// Several drivers share one protocol engine (the GRBL streaming
/// stack, for example); forwarding keeps each driver a distinct type
/// without duplicating the trait surface.
macro_rules! delegate_driver {
    ($ty:ty, $field:ident) => {
        #[async_trait::async_trait]
        impl $crate::driver::core::driver::Driver for $ty {
            fn info(&self) -> &'static $crate::driver::core::driver::DriverInfo {
                $crate::driver::core::driver::Driver::info(&*self.$field)
            }

            fn state(&self) -> $crate::driver::core::types::DeviceState {
                $crate::driver::core::driver::Driver::state(&*self.$field)
            }

            fn is_job_running(&self) -> bool {
                $crate::driver::core::driver::Driver::is_job_running(&*self.$field)
            }

            fn resource_uri(&self) -> Option<String> {
                $crate::driver::core::driver::Driver::resource_uri(&*self.$field)
            }

            fn update_dialect(
                &self,
                dialect: $crate::driver::core::dialect::Dialect,
            ) {
                $crate::driver::core::driver::Driver::update_dialect(
                    &*self.$field,
                    dialect,
                )
            }

            fn abort_background_tasks_now(&self) {
                $crate::driver::core::driver::Driver::abort_background_tasks_now(
                    &*self.$field,
                )
            }

            async fn connect(
                self: std::sync::Arc<Self>,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::connect(self.$field.clone()).await
            }

            async fn cleanup(&self) {
                $crate::driver::core::driver::Driver::cleanup(&*self.$field).await
            }

            async fn run(
                self: std::sync::Arc<Self>,
                gcode: &str,
                op_map: &std::collections::HashMap<usize, i64>,
                op_estimates: &[f64],
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::run(
                    self.$field.clone(),
                    gcode,
                    op_map,
                    op_estimates,
                )
                .await
            }

            async fn run_raw(
                self: std::sync::Arc<Self>,
                machine_code: &str,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::run_raw(
                    self.$field.clone(),
                    machine_code,
                )
                .await
            }

            async fn cancel(
                &self,
                mode: $crate::driver::core::driver::CancelMode,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::cancel(&*self.$field, mode).await
            }

            async fn execute_interactive_command(
                &self,
                command: &str,
            ) -> Result<Vec<String>, $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::execute_interactive_command(
                    &*self.$field,
                    command,
                )
                .await
            }

            async fn execute_command(
                &self,
                command: &str,
            ) -> Result<Vec<String>, $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::execute_command(
                    &*self.$field,
                    command,
                )
                .await
            }

            fn buffer_count(&self) -> usize {
                $crate::driver::core::driver::Driver::buffer_count(&*self.$field)
            }

            fn rx_buffer_size(&self) -> usize {
                $crate::driver::core::driver::Driver::rx_buffer_size(&*self.$field)
            }

            fn pending_commands(&self) -> Vec<String> {
                $crate::driver::core::driver::Driver::pending_commands(&*self.$field)
            }

            async fn home(
                &self,
                axes: Option<Vec<String>>,
                active_wcs: &str,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::home(
                    &*self.$field,
                    axes,
                    active_wcs,
                )
                .await
            }

            async fn move_to(
                &self,
                speed: f64,
                x: f64,
                y: f64,
                z: Option<f64>,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::move_to(
                    &*self.$field,
                    speed,
                    x,
                    y,
                    z,
                )
                .await
            }

            async fn jog(
                &self,
                speed: f64,
                deltas: &[(String, f64)],
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::jog(&*self.$field, speed, deltas)
                    .await
            }

            async fn select_tool(
                &self,
                tool_number: i64,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::select_tool(
                    &*self.$field,
                    tool_number,
                )
                .await
            }

            async fn set_power(
                &self,
                power: Option<f64>,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::set_power(&*self.$field, power)
                    .await
            }

            async fn set_focus_power(
                &self,
                power: Option<f64>,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::set_focus_power(
                    &*self.$field,
                    power,
                )
                .await
            }

            async fn set_hold(
                &self,
                hold: bool,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::set_hold(&*self.$field, hold)
                    .await
            }

            async fn clear_alarm(
                &self,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::clear_alarm(&*self.$field).await
            }

            async fn set_wcs_offset(
                &self,
                wcs_slot: &str,
                x: f64,
                y: f64,
                z: Option<f64>,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::set_wcs_offset(
                    &*self.$field,
                    wcs_slot,
                    x,
                    y,
                    z,
                )
                .await
            }

            async fn read_settings(
                &self,
            ) -> Result<Vec<(String, String)>, $crate::driver::core::driver::DriverError>
            {
                $crate::driver::core::driver::Driver::read_settings(&*self.$field).await
            }

            async fn write_setting(
                &self,
                key: &str,
                value: &str,
            ) -> Result<(), $crate::driver::core::driver::DriverError> {
                $crate::driver::core::driver::Driver::write_setting(
                    &*self.$field,
                    key,
                    value,
                )
                .await
            }

            async fn read_wcs_offsets(
                &self,
            ) -> Result<
                std::collections::HashMap<String, (f64, f64, f64)>,
                $crate::driver::core::driver::DriverError,
            > {
                $crate::driver::core::driver::Driver::read_wcs_offsets(&*self.$field)
                    .await
            }

            async fn read_parser_state(
                &self,
            ) -> Result<Option<String>, $crate::driver::core::driver::DriverError>
            {
                $crate::driver::core::driver::Driver::read_parser_state(&*self.$field)
                    .await
            }

            async fn run_probe_cycle(
                &self,
                axis_letter: &str,
                max_travel: f64,
                feed_rate: f64,
            ) -> Result<
                Option<(f64, f64, f64)>,
                $crate::driver::core::driver::DriverError,
            > {
                $crate::driver::core::driver::Driver::run_probe_cycle(
                    &*self.$field,
                    axis_letter,
                    max_travel,
                    feed_rate,
                )
                .await
            }

            async fn detect_unit_system(
                &self,
            ) -> Result<
                Option<$crate::driver::core::types::UnitSystem>,
                $crate::driver::core::driver::DriverError,
            > {
                $crate::driver::core::driver::Driver::detect_unit_system(&*self.$field)
                    .await
            }
        }
    };
}

pub(crate) use delegate_driver;

/// An asyncio.Event-style latch built on tokio primitives: the
/// `handshake` latch of the line sessions and generally useful for
/// session implementations.
pub struct AsyncEvent {
    set: Mutex<bool>,
    notify: tokio::sync::Notify,
}

impl Default for AsyncEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncEvent {
    pub fn new() -> Self {
        Self {
            set: Mutex::new(false),
            notify: tokio::sync::Notify::new(),
        }
    }

    pub fn set(&self) {
        *self.set.lock().unwrap() = true;
        self.notify.notify_waiters();
        self.notify.notify_one();
    }

    pub fn clear(&self) {
        *self.set.lock().unwrap() = false;
    }

    pub fn is_set(&self) -> bool {
        *self.set.lock().unwrap()
    }

    pub async fn wait(&self) {
        loop {
            if self.is_set() {
                return;
            }
            let notified = self.notify.notified();
            if self.is_set() {
                return;
            }
            notified.await;
        }
    }
}
