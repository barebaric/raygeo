//! Device census for GRBL controllers.
//!
//! Interprets the raw ``$I`` and ``$$`` responses into the
//! structured census dict: firmware identity, compile flags, flavor
//! classification (stock Grbl / grblHAL / FluidNC), the parsed
//! settings inventory, and the semantic values applications need
//! (work travel, motion limits, laser mode, unit reporting). Pure
//! functions over parser output — no I/O — so local sessions and
//! remote daemon proxies are probed identically.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::driver::grbl::parser::{
    extract_device_name, parse_grbl_settings, parse_opt_flags, parse_opt_info,
    parse_version,
};

/// Regexes matching grblHAL / FluidNC identifiers in a ``[VER:]``
/// line.
static HAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)grblhal").unwrap());
static FLUIDNC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("(?i)fluidnc").unwrap());

/// grblHAL exposes per-axis settings under the ``$4xx`` group
/// (e.g. ``$400`` for axis steps/mm). Stock Grbl uses ``$0``–
/// ``$255``.
static HAL_SETTING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$4\d{2}").unwrap());

/// The firmware family a device identifies itself as.
fn detect_flavor(
    build_info: &[String],
    settings_lines: &[String],
) -> &'static str {
    for line in build_info {
        if HAL_RE.is_match(line) {
            return "grblhal";
        }
        if FLUIDNC_RE.is_match(line) {
            return "fluidnc";
        }
    }
    if settings_lines
        .iter()
        .any(|line| HAL_SETTING_RE.is_match(line))
    {
        return "grblhal";
    }
    if parse_version(build_info).is_some() {
        return "stock";
    }
    "unknown"
}

/// The structured result of probing a GRBL device: firmware
/// identity, compile flags, flavor classification and the semantic
/// values applications need.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrblCensus {
    pub name: String,
    pub firmware_version: Option<String>,
    pub rx_buffer_size: Option<i64>,
    pub compile_flags: String,
    pub flavor: String,
    pub settings: HashMap<String, f64>,
    pub max_travel: Option<(f64, f64)>,
    pub max_speed: Option<i64>,
    pub acceleration: Option<i64>,
    pub max_spindle: Option<i64>,
    pub laser_mode: Option<bool>,
    pub homing_enabled: Option<bool>,
    pub report_inches: Option<bool>,
    pub arc_tolerance: Option<f64>,
    pub single_axis_homing: bool,
}

impl GrblCensus {
    /// The wire form served by the daemon protocol.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({}))
    }

    /// Rebuilds a census from its wire form.
    pub fn from_json(value: Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

/// Builds the census from the raw Grbl ``$I`` and ``$$`` response
/// lines.
///
/// Pure data transformation with no I/O; the caller is responsible
/// for communicating with the device and passing the collected
/// response lines.
pub fn describe(
    build_info: &[String],
    settings_lines: &[String],
) -> GrblCensus {
    let mut rx_buffer_size: Option<i64> = None;
    let mut compile_flags = String::new();
    for line in build_info {
        if let Some(rx) = parse_opt_info(line) {
            rx_buffer_size = Some(rx);
        }
        if let Some(flags) = parse_opt_flags(line) {
            compile_flags = flags;
        }
    }

    let settings: HashMap<String, f64> = parse_grbl_settings(settings_lines);
    let setting = |key: &str| settings.get(key).copied();

    let max_speed = match (setting("110"), setting("111")) {
        (Some(x), Some(y)) => Some((x.min(y)) as i64),
        _ => None,
    };
    let acceleration = match (setting("120"), setting("121")) {
        (Some(x), Some(y)) => Some((x.min(y)) as i64),
        _ => None,
    };
    let max_travel = match (setting("130"), setting("131")) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    };
    let max_spindle = setting("30").map(|value| value as i64);
    let laser_mode = setting("32").map(|value| value != 0.0);
    let homing_enabled = setting("22").map(|value| value != 0.0);
    let report_inches = setting("13").map(|value| value != 0.0);
    let arc_tolerance = setting("12");

    GrblCensus {
        name: extract_device_name(build_info),
        firmware_version: parse_version(build_info),
        rx_buffer_size,
        compile_flags: compile_flags.clone(),
        flavor: detect_flavor(build_info, settings_lines).to_string(),
        settings,
        max_travel,
        max_speed,
        acceleration,
        max_spindle,
        laser_mode,
        homing_enabled,
        report_inches,
        arc_tolerance,
        single_axis_homing: compile_flags.contains('H'),
    }
}
