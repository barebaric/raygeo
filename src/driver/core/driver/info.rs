//! Protocol-neutral driver metadata: identity, maturity, typed
//! capabilities and the auto-detection fingerprint.

use serde_json::{json, Value};

/// How battle-tested a driver is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverMaturity {
    Stable,
    Untested,
    Experimental,
    KnownBuggy,
}

impl DriverMaturity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Untested => "untested",
            Self::Experimental => "experimental",
            Self::KnownBuggy => "known_buggy",
        }
    }
}

/// The physical connection a driver dials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    Serial,
    Telnet,
    Network,
    Bluetooth,
    /// The driver owns no transport (simulator, remote proxy).
    None,
}

impl ConnectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Serial => "serial",
            Self::Telnet => "telnet",
            Self::Network => "network",
            Self::Bluetooth => "bluetooth",
            Self::None => "none",
        }
    }
}

/// The machine-agnostic capabilities a driver claims.
///
/// The booleans mirror the class attributes of the original pure
/// Python `Driver` base, so hosts can gate UI without probing the
/// driver by error.
#[derive(Debug, Clone, Copy, Default)]
pub struct Capabilities {
    /// Firmware settings can be read and written.
    pub settings: bool,
    /// A device census and profile probe are available.
    pub probing: bool,
    /// The driver can report metric vs imperial.
    pub unit_detection: bool,
    /// Pause and resume are supported.
    pub hold: bool,
    /// Alarms can be cleared.
    pub clear_alarm: bool,
    /// Per-command progress is reported during jobs.
    pub granular_progress: bool,
    /// Jogging is supported.
    pub jog: bool,
    /// Homing a single axis is supported (not only all axes).
    pub single_axis_homing: bool,
    /// Firmware performs its own overscan.
    pub native_overscan: bool,
    /// Multi-depth raster mode is supported.
    pub multi_depth_raster: bool,
    /// RX-buffer statistics are reported.
    pub buffer_stats: bool,
}

/// Build a [`Capabilities`] value in a `static` initializer.
#[allow(clippy::too_many_arguments)]
pub const fn caps(
    settings: bool,
    probing: bool,
    unit_detection: bool,
    hold: bool,
    clear_alarm: bool,
    granular_progress: bool,
    jog: bool,
    single_axis_homing: bool,
    native_overscan: bool,
    multi_depth_raster: bool,
    buffer_stats: bool,
) -> Capabilities {
    Capabilities {
        settings,
        probing,
        unit_detection,
        hold,
        clear_alarm,
        granular_progress,
        jog,
        single_axis_homing,
        native_overscan,
        multi_depth_raster,
        buffer_stats,
    }
}

/// The device fingerprint used by the daemon to auto-detect a
/// protocol on an unknown serial port.
#[derive(Debug, Clone, Copy)]
pub struct ProbeFingerprint {
    /// The interactive command sent to the device.
    pub command: &'static str,
    /// A marker expected in the response.
    pub signature: &'static str,
}

/// The kind of value a setup field takes, for host-side form
/// rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupKind {
    SerialPort,
    Baudrate,
    Hostname,
    Port,
    Text,
    Bool,
    Int,
    Float,
    Choice,
}

impl SetupKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SerialPort => "serial_port",
            Self::Baudrate => "baudrate",
            Self::Hostname => "hostname",
            Self::Port => "port",
            Self::Text => "text",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
            Self::Choice => "choice",
        }
    }
}

/// One connection-setup field a driver accepts.
///
/// This is the machine-readable half of the setup form: keys, types,
/// defaults, ranges and choices.  Labels and descriptions are the
/// host application's presentation data.
#[derive(Debug, Clone, Copy)]
pub struct SetupField {
    pub key: &'static str,
    pub kind: SetupKind,
    pub required: bool,
    pub default: Option<&'static str>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub choices: &'static [&'static str],
}

impl SetupField {
    pub const fn new(key: &'static str, kind: SetupKind) -> Self {
        Self {
            key,
            kind,
            required: false,
            default: None,
            min: None,
            max: None,
            choices: &[],
        }
    }

    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub const fn with_default(mut self, value: &'static str) -> Self {
        self.default = Some(value);
        self
    }

    pub const fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    pub const fn with_choices(
        mut self,
        choices: &'static [&'static str],
    ) -> Self {
        self.choices = choices;
        self
    }

    pub fn to_json(&self) -> Value {
        json!({
            "key": self.key,
            "kind": self.kind.as_str(),
            "required": self.required,
            "default": self.default,
            "min": self.min,
            "max": self.max,
            "choices": self.choices,
        })
    }
}

/// Typed metadata describing one driver class.
///
/// Every driver owns exactly one of these; it replaces the former
/// protocol registry and its method-name string sets.
#[derive(Debug, Clone)]
pub struct DriverInfo {
    /// The class name, which is also the persisted driver name.
    pub name: &'static str,
    pub connection: ConnectionKind,
    pub maturity: DriverMaturity,
    pub caps: Capabilities,
    /// The protocol speaks G-code (false for print-job and binary
    /// protocols).
    pub uses_gcode: bool,
    /// The WCS id that denotes machine space.
    pub machine_space_wcs: &'static str,
    /// The selectable work-coordinate systems.
    pub supported_wcs: &'static [&'static str],
    /// The connection config keys the driver accepts.
    pub config_keys: &'static [&'static str],
    /// The typed connection-setup form.
    pub setup: &'static [SetupField],
    /// Serial command/response signature for auto-detection.
    pub fingerprint: Option<ProbeFingerprint>,
}

impl DriverInfo {
    /// The capability flags as a JSON object.
    pub fn capabilities_json(&self) -> Value {
        let caps = self.caps;
        json!({
            "uses_gcode": self.uses_gcode,
            "supports_settings": caps.settings,
            "supports_probing": caps.probing,
            "supports_hold": caps.hold,
            "supports_clear_alarm": caps.clear_alarm,
            "supports_unit_detection": caps.unit_detection,
            "supports_jog": caps.jog,
            "supports_single_axis_homing": caps.single_axis_homing,
            "native_overscan": caps.native_overscan,
            "supports_multi_depth_raster": caps.multi_depth_raster,
            "reports_granular_progress": caps.granular_progress,
            "reports_buffer_stats": caps.buffer_stats,
        })
    }

    /// The wire form served by `driver/list`.
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "connection": self.connection.as_str(),
            "maturity": self.maturity.as_str(),
            "capabilities": self.capabilities_json(),
            "uses_gcode": self.uses_gcode,
            "machine_space_wcs": self.machine_space_wcs,
            "supported_wcs": self.supported_wcs,
            "config_keys": self.config_keys,
            "setup": self.setup.iter().map(SetupField::to_json).collect::<Vec<_>>(),
            "fingerprint": self.fingerprint.map(|fingerprint| json!({
                "command": fingerprint.command,
                "signature": fingerprint.signature,
            })),
        })
    }
}

/// The default WCS set every G-code driver offers.
pub const DEFAULT_SUPPORTED_WCS: &[&str] =
    &["G54", "G55", "G56", "G57", "G58", "G59"];
