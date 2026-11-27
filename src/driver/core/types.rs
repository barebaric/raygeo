//! Shared device value types (status, state, errors, units).
//!
//! These mirror Rayforge's `machine.driver.driver` module so the
//! Python bindings can convert losslessly.

use std::collections::HashMap;
use std::fmt;

/// Machine state as reported in device status reports (GRBL-style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeviceStatus {
    #[default]
    Unknown,
    Idle,
    Run,
    Hold,
    Jog,
    Alarm,
    Door,
    Check,
    Home,
    Sleep,
    Tool,
    Queue,
    Lock,
    Unlock,
    Cycle,
    Test,
}

impl DeviceStatus {
    /// Parse the state name from a status report (e.g. `Idle`,
    /// `Hold:0`).  Unknown names map to [`DeviceStatus::Unknown`],
    /// mirroring the Python driver.
    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_uppercase().as_str() {
            "IDLE" => Self::Idle,
            "RUN" => Self::Run,
            "HOLD" => Self::Hold,
            "JOG" => Self::Jog,
            "ALARM" => Self::Alarm,
            "DOOR" => Self::Door,
            "CHECK" => Self::Check,
            "HOME" => Self::Home,
            "SLEEP" => Self::Sleep,
            "TOOL" => Self::Tool,
            "QUEUE" => Self::Queue,
            "LOCK" => Self::Lock,
            "UNLOCK" => Self::Unlock,
            "CYCLE" => Self::Cycle,
            "TEST" => Self::Test,
            _ => Self::Unknown,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Idle => "Idle",
            Self::Run => "Run",
            Self::Hold => "Hold",
            Self::Jog => "Jog",
            Self::Alarm => "Alarm",
            Self::Door => "Door",
            Self::Check => "Check",
            Self::Home => "Home",
            Self::Sleep => "Sleep",
            Self::Tool => "Tool",
            Self::Queue => "Queue",
            Self::Lock => "Lock",
            Self::Unlock => "Unlock",
            Self::Cycle => "Cycle",
            Self::Test => "Test",
        }
    }
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// An error or alarm with code, title and description.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceError {
    pub code: i32,
    pub title: String,
    pub description: String,
}

impl DeviceError {
    pub fn new(code: i32, title: &str, description: &str) -> Self {
        Self {
            code,
            title: title.to_string(),
            description: description.to_string(),
        }
    }
}

/// A position tuple in mm; entries may be unknown (`None`).
pub type Pos = Vec<Option<f64>>;

/// The complete state of a device at a moment in time.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceState {
    pub status: DeviceStatus,
    pub error: Option<DeviceError>,
    pub machine_pos: Pos,
    pub work_pos: Pos,
    pub wco: Pos,
    pub feed_rate: Option<i64>,
    pub spindle_speed: Option<i64>,
    pub buffer_available: Option<i64>,
    pub buffer_rx_available: Option<i64>,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            status: DeviceStatus::Unknown,
            error: None,
            machine_pos: vec![None, None, None],
            work_pos: vec![None, None, None],
            wco: vec![Some(0.0), Some(0.0), Some(0.0)],
            feed_rate: None,
            spindle_speed: None,
            buffer_available: None,
            buffer_rx_available: None,
        }
    }
}

/// Connection lifecycle status of a transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportStatus {
    #[default]
    Unknown,
    Idle,
    Connecting,
    Connected,
    Error,
    Closing,
    Disconnected,
    Sleeping,
}

impl TransportStatus {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Idle => "IDLE",
            Self::Connecting => "CONNECTING",
            Self::Connected => "CONNECTED",
            Self::Error => "ERROR",
            Self::Closing => "CLOSING",
            Self::Disconnected => "DISCONNECTED",
            Self::Sleeping => "SLEEPING",
        }
    }
}

impl fmt::Display for TransportStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Unit system inferred from the `$13` (report in inches) setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitSystem {
    Metric,
    Imperial,
}

/// Millimeters per inch.
pub const MM_PER_INCH: f64 = 25.4;

/// WCS slot offsets in mm: slot name → (x, y, z).
pub type WcsOffsets = HashMap<String, (f64, f64, f64)>;

pub fn inches_to_mm(v: f64) -> f64 {
    v * MM_PER_INCH
}
