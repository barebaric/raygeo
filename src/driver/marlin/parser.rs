//! Parsing utilities for the Marlin firmware, ported from
//! Rayforge's `marlin_util.py`.  All functions are side-effect free;
//! the discovery-oriented byte-level helpers exist so device
//! discovery and the session agree on what a Marlin boot banner
//! looks like.

use regex::Regex;
use std::sync::LazyLock;

use crate::driver::core::types::UnitSystem;

pub(crate) static M114_POS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"X:([+-]?\d+\.?\d*)\s+Y:([+-]?\d+\.?\d*)\s+Z:([+-]?\d+\.?\d*)")
        .expect("valid regex")
});

static MARLIN_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Marlin\s+([\d.]+)").expect("valid regex"));

static MARLIN_ERROR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Error:(.+)").expect("valid regex"));

static M115_FIRMWARE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"FIRMWARE_NAME:\s*(\S+)").expect("valid regex")
});

static M115_MACHINE_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"MACHINE_TYPE:(.+?)(?:\s+[A-Z_]+:|$)").expect("valid regex")
});

static M149_UNITS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)M149\s+Units\s+in\s+(inches|inch|in|mm|millimeters|millimetres)",
    )
    .expect("valid regex")
});

/// True when *line* is a Marlin `ok` acknowledgement.  Handles plain
/// `ok` as well as temperature-augmented forms such as
/// `ok T:19.5 /200.0 B:60.0 /60.0`.
pub fn is_ok(line: &str) -> bool {
    let stripped = line.trim();
    stripped.starts_with("ok")
        && (stripped.len() == 2
            || !stripped[2..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric()))
}

/// True when *line* is a Marlin error response (`Error:...`).
pub fn is_error(line: &str) -> bool {
    line.trim().starts_with("Error:")
}

/// True when *line* is a Marlin boot/startup message (`start`,
/// `Marlin`, `echo:`, `External Reset`).
pub fn is_boot_message(line: &str) -> bool {
    let stripped = line.trim();
    ["start", "Marlin", "echo:", "External Reset"]
        .iter()
        .any(|prefix| stripped.starts_with(prefix))
}

/// True when raw serial output identifies a Marlin device.
pub fn is_marlin_output(data: &[u8]) -> bool {
    let text = String::from_utf8_lossy(data);
    text.lines().any(is_boot_message)
}

/// Extract a human-readable identity hint from raw Marlin serial
/// output: the first boot message line, skipping bare `ok`/`Error:`
/// acknowledgements.  Returns None when the output carries no boot
/// message.
pub fn extract_marlin_banner_from_output(data: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || is_ok(line) || is_error(line) {
            continue;
        }
        if is_boot_message(line) {
            return Some(line.chars().take(80).collect());
        }
    }
    None
}

/// True when *line* carries an M114 position report.
pub fn is_m114_position(line: &str) -> bool {
    M114_POS_RE.is_match(line)
}

/// Parse a single M114 position line like
/// `X:10.00 Y:20.00 Z:0.00 E:0.00 Count ...`.
pub fn parse_m114_line(line: &str) -> Option<(f64, f64, f64)> {
    let caps = M114_POS_RE.captures(line)?;
    Some((
        caps[1].parse().ok()?,
        caps[2].parse().ok()?,
        caps[3].parse().ok()?,
    ))
}

/// Parse M114 output to extract the (x, y, z) position, or None
/// when no position line is present.
pub fn parse_m114_position(lines: &[String]) -> Option<(f64, f64, f64)> {
    for line in lines {
        if let Some(pos) = parse_m114_line(line) {
            return Some(pos);
        }
    }
    None
}

/// Extract the Marlin version string from a boot line like
/// `Marlin 2.1.2.7`.
pub fn parse_marlin_version(line: &str) -> Option<String> {
    MARLIN_VERSION_RE
        .captures(line)
        .map(|caps| caps[1].to_string())
}

/// Extract the message portion of an `Error:...` line.
pub fn parse_error_message(line: &str) -> String {
    MARLIN_ERROR_RE
        .captures(line)
        .map(|caps| caps[1].trim().to_string())
        .unwrap_or_default()
}

/// Parse M115 response lines into firmware name and machine type.
pub fn parse_m115_firmware_info(lines: &[String]) -> Vec<(String, String)> {
    let mut info = Vec::new();
    for line in lines {
        if let Some(caps) = M115_FIRMWARE_RE.captures(line) {
            info.push((
                "firmware_name".to_string(),
                caps[1].trim().to_string(),
            ));
        }
        if let Some(caps) = M115_MACHINE_TYPE_RE.captures(line) {
            info.push(("machine_type".to_string(), caps[1].trim().to_string()));
        }
    }
    info
}

static M203_FEEDRATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"echo:\s*M203\s+X([+-]?\d+\.?\d*)\s+Y([+-]?\d+\.?\d*)")
        .expect("valid regex")
});

static M204_ACCEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"echo:\s*M204\s*S([+-]?\d+\.?\d*)").expect("valid regex")
});

static M211_MAX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"X:?\s*([+-]?\d+\.?\d*)\s+Y:?\s*([+-]?\d+\.?\d*)")
        .expect("valid regex")
});

/// Parse M211 output to extract X/Y max travel from software
/// endstops.
pub fn parse_m211_endstops(lines: &[String]) -> Option<(f64, f64)> {
    for line in lines {
        if let Some(caps) = M211_MAX_RE.captures(line) {
            return Some((caps[1].parse().ok()?, caps[2].parse().ok()?));
        }
    }
    None
}

/// Parse M503 output into key motion settings: `max_feedrate_x`,
/// `max_feedrate_y` (mm/s from M203) and `acceleration` (mm/s^2
/// from M204).
pub fn parse_m503_settings(lines: &[String]) -> Vec<(String, f64)> {
    let mut settings = Vec::new();
    for line in lines {
        if let Some(caps) = M203_FEEDRATE_RE.captures(line) {
            if let (Ok(x), Ok(y)) = (caps[1].parse(), caps[2].parse()) {
                settings.push(("max_feedrate_x".to_string(), x));
                settings.push(("max_feedrate_y".to_string(), y));
            }
        }
        if let Some(caps) = M204_ACCEL_RE.captures(line) {
            if let Ok(accel) = caps[1].parse() {
                settings.push(("acceleration".to_string(), accel));
            }
        }
    }
    settings
}

/// Extract a human-readable device name from M115 and boot output:
/// `MACHINE_TYPE` first (unless it is a generic placeholder), then
/// the firmware name, then the version from the boot banner.
pub fn extract_marlin_device_name(
    m115_lines: &[String],
    boot_lines: &[String],
) -> String {
    let machine_type = parse_m115_firmware_info(m115_lines)
        .into_iter()
        .find(|(key, _)| key == "machine_type")
        .map(|(_, value)| value);
    if let Some(name) = machine_type {
        let generic = ["3d printer", "default"]
            .iter()
            .any(|g| name.to_lowercase() == *g);
        if !generic {
            return name;
        }
    }
    let firmware_name = parse_m115_firmware_info(m115_lines)
        .into_iter()
        .find(|(key, _)| key == "firmware_name")
        .map(|(_, value)| value);
    if let Some(name) = firmware_name {
        if !name.is_empty() {
            return name;
        }
    }
    for line in boot_lines {
        if let Some(version) = parse_marlin_version(line) {
            return format!("Marlin {version}");
        }
    }
    "Unknown Marlin Device".to_string()
}

/// Inspect Marlin `M149` response lines and infer the device's unit
/// system (`echo: M149 Units in inches` / `... in mm`).
pub fn detect_unit_system_from_m149(lines: &[String]) -> Option<UnitSystem> {
    for line in lines {
        if let Some(caps) = M149_UNITS_RE.captures(line) {
            let unit = caps[1].to_lowercase();
            return Some(
                if ["inches", "inch", "in"].contains(&unit.as_str()) {
                    UnitSystem::Imperial
                } else {
                    UnitSystem::Metric
                },
            );
        }
    }
    None
}
