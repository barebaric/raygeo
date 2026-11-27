//! Device census for Smoothieware controllers.
//!
//! Turns the raw ``version`` and ``config-get`` responses into the
//! structured census dict: firmware identity and the motion limits
//! applications need. Pure functions over parser output — no I/O —
//! so local sessions and remote daemon proxies are probed
//! identically.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The structured result of probing a Smoothieware device: firmware
/// identity and the motion limits applications need.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmoothieCensus {
    pub firmware_version: Option<String>,
    pub alpha_max: Option<f64>,
    pub beta_max: Option<f64>,
    pub alpha_rate: Option<f64>,
    pub beta_rate: Option<f64>,
    pub acceleration: Option<f64>,
    pub max_travel: Option<(f64, f64)>,
    pub max_speed: Option<i64>,
    pub acceleration_steps: Option<i64>,
}

impl SmoothieCensus {
    /// The wire form served by the daemon protocol.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({}))
    }

    /// Rebuilds a census from its wire form.
    pub fn from_json(value: Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

/// Commands the census queries, in order.
pub const CENSUS_COMMANDS: [&str; 6] = [
    "version",
    "config-get alpha_max",
    "config-get beta_max",
    "config-get alpha_max_rate",
    "config-get beta_max_rate",
    "config-get acceleration",
];

/// The first non-empty reply line, if any.
fn first_value(lines: &[String]) -> Option<String> {
    lines
        .iter()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())
        .map(|line| line.to_string())
}

/// Drops ``ok`` acknowledgements and ``<...>`` status reports:
/// ``config-get`` replies arrive with a trailing ``ok``, and an
/// idle status poll can interleave; neither carries census data.
fn filter_reply(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed != "ok"
                && !(trimmed.starts_with('<') && trimmed.ends_with('>'))
        })
        .cloned()
        .collect()
}

/// Parses a ``config-get`` numeric reply into a float. Smoothieware
/// replies with the bare value (e.g. ``200.000``); non-numeric
/// replies yield ``None``.
fn parse_float(lines: &[String]) -> Option<f64> {
    first_value(&filter_reply(lines)).and_then(|raw| raw.parse::<f64>().ok())
}

/// Builds the census dict from the raw Smoothieware census-command
/// responses. Pure data transformation with no I/O; the caller is
/// responsible for communicating with the device and passing the
/// collected response lines.
pub fn describe(
    version_lines: &[String],
    alpha_max_lines: &[String],
    beta_max_lines: &[String],
    alpha_rate_lines: &[String],
    beta_rate_lines: &[String],
    acceleration_lines: &[String],
) -> SmoothieCensus {
    let alpha_max = parse_float(alpha_max_lines);
    let beta_max = parse_float(beta_max_lines);
    let alpha_rate = parse_float(alpha_rate_lines);
    let beta_rate = parse_float(beta_rate_lines);
    let acceleration = parse_float(acceleration_lines);

    let max_travel = match (alpha_max, beta_max) {
        (Some(alpha_max), Some(beta_max))
            if alpha_max > 0.0 && beta_max > 0.0 =>
        {
            Some((alpha_max, beta_max))
        }
        _ => None,
    };

    let max_speed = match (alpha_rate, beta_rate) {
        // config-get rates are in mm/s; machine configs are mm/min.
        (Some(alpha_rate), Some(beta_rate)) => {
            Some((alpha_rate.min(beta_rate) * 60.0) as i64)
        }
        _ => None,
    };

    SmoothieCensus {
        firmware_version: first_value(&filter_reply(version_lines)),
        alpha_max,
        beta_max,
        alpha_rate,
        beta_rate,
        acceleration,
        max_travel,
        max_speed,
        acceleration_steps: acceleration.map(|value| value as i64),
    }
}
