//! Parsers for Grbl responses: status reports, build info, settings,
//! WCS offsets, probe results and raw-output classification.
//!
//! This is a faithful port of Rayforge's `grbl_util.py` parsing
//! helpers; behavior is byte-for-byte compatible.

use std::collections::HashMap;
use std::sync::LazyLock;

use super::errors::{alarm_code_to_device_error, error_code_to_device_error};
use crate::driver::core::types::{
    inches_to_mm, DeviceError, DeviceState, DeviceStatus, Pos, UnitSystem,
};

/// Grbl realtime command characters.  The firmware executes these
/// immediately upon reception: they bypass the RX buffer, must never
/// be queued behind buffered gcode, and never produce an 'ok' ack.
pub const GRBL_REALTIME_COMMANDS: [&str; 3] = ["?", "~", "!"];

type Regex = regex::Regex;

static POS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r":(-?\d+\.?\d*),(-?\d+\.?\d*)(?:,(-?\d+\.?\d*))?(?:,(-?\d+\.?\d*))?",
    )
    .unwrap()
});
static FS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"FS:(\d+),(\d+)").unwrap());
static BF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Bf:(\d+),(\d+)").unwrap());
static GRBL_SETTING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$(\d+)=([\d\.-]+)").unwrap());
static WCS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[(G5[4-9]):([\d\.-]+),([\d\.-]+)(?:,([\d\.-]+))?\]").unwrap()
});
static PRB_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[PRB:([\d\.-]+),([\d\.-]+),([\d\.-]+):(\d)\]").unwrap()
});
static GRBL_PARSER_STATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r".*(G5[4-9]).*").unwrap());
static GRBL_OPT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[OPT:([A-Z]+),(\d+),(\d+)\]").unwrap());
static GCODE_COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\([^)]*\)").unwrap());

type BytesRegex = regex::bytes::Regex;

static GRBL_DISCOVERY_RE: LazyLock<BytesRegex> = LazyLock::new(|| {
    BytesRegex::new(
        r"(?i)\bgrbl(?:hal)?\b|<(?:Idle|Run|Hold|Alarm|Home)[,|>]|\[(?:VER|OPT|MSG|GC):",
    )
    .unwrap()
});

/// True when raw serial output identifies a Grbl device.
pub fn is_grbl_output(data: &[u8]) -> bool {
    GRBL_DISCOVERY_RE.is_match(data)
}

/// Split command lines into realtime commands and regular gcode.
///
/// Returns `(gcode_lines, realtime_lines)`, preserving the order
/// within each group.
pub fn split_realtime_commands(lines: &[String]) -> (Vec<String>, Vec<String>) {
    let mut gcode = Vec::new();
    let mut realtime = Vec::new();
    for line in lines {
        if GRBL_REALTIME_COMMANDS.contains(&line.as_str()) {
            realtime.push(line.clone());
        } else {
            gcode.push(line.clone());
        }
    }
    (gcode, realtime)
}

/// Strip G-code comments from a line: everything after ';' and
/// content between '(' and ')'.
pub fn strip_gcode_comments(line: &str) -> String {
    let mut line = GCODE_COMMENT_RE.replace_all(line, "").to_string();
    if let Some(idx) = line.find(';') {
        line.truncate(idx);
    }
    line.trim().to_string()
}

/// Converts a G-code WCS name (e.g. "G54") to its P-number.
pub fn gcode_to_p_number(wcs_slot: &str) -> Option<i64> {
    let rest = wcs_slot.strip_prefix('G')?;
    let num: i64 = rest.parse().ok()?;
    let p_num = num - 53;
    if (1..=6).contains(&p_num) {
        Some(p_num)
    } else {
        None
    }
}

/// Parse a `[VER:...]` line into `(version, build_name)`.
///
/// Handles both standard Grbl format (`1.1h.ORTUR`) and
/// comma-separated format (`1.0.15,20240923`).  Returns `None` if
/// the line is not a VER line.
pub fn parse_ver(line: &str) -> Option<(String, Option<String>)> {
    let content = line.strip_prefix("[VER:")?;
    let content = content.trim_end_matches([':', ']']);
    if content.is_empty() {
        return None;
    }
    if let Some((version, _)) = content.split_once(',') {
        return Some((version.to_string(), None));
    }
    let parts: Vec<&str> = content.split('.').collect();
    match parts.len() {
        n if n >= 3 => Some((
            format!("{}.{}", parts[0], parts[1]),
            Some(parts[2].to_string()),
        )),
        2 => Some((format!("{}.{}", parts[0], parts[1]), None)),
        _ => Some((content.to_string(), None)),
    }
}

/// Parses '$I' output lines to extract the GRBL firmware version
/// string (e.g. `1.1h`), or `None` if no VER line is found.
pub fn parse_version(response_lines: &[String]) -> Option<String> {
    response_lines
        .iter()
        .find_map(|line| parse_ver(line).map(|(version, _)| version))
}

/// Parse `$$` response lines into `(key, value)` pairs.
pub fn parse_grbl_settings(lines: &[String]) -> HashMap<String, f64> {
    let mut settings = HashMap::new();
    for (key, value) in parse_setting_pairs(lines) {
        if let Ok(value) = value.parse::<f64>() {
            settings.insert(key, value);
        }
    }
    settings
}

/// Parse `$$` response lines into ordered `(key, raw_value_string)`
/// pairs, preserving the raw text for settings UIs.
pub fn parse_setting_pairs(lines: &[String]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for line in lines {
        if let Some(caps) = GRBL_SETTING_RE.captures(line) {
            pairs.push((caps[1].to_string(), caps[2].to_string()));
        }
    }
    pairs
}

/// Inspect GRBL `$$` response lines and infer the device's unit
/// system from the `$13` (Report in inches) setting.
pub fn detect_unit_system_from_settings(
    settings_lines: &[String],
) -> Option<UnitSystem> {
    let report_inches = parse_grbl_settings(settings_lines).get("13").copied();
    match report_inches {
        None => None,
        Some(v) if v as i64 != 0 => Some(UnitSystem::Imperial),
        Some(_) => Some(UnitSystem::Metric),
    }
}

/// True when GRBL's `$13` (Report in inches) flag is set.
pub fn is_report_in_inches(settings_lines: &[String]) -> bool {
    parse_grbl_settings(settings_lines)
        .get("13")
        .is_some_and(|v| *v as i64 != 0)
}

/// Parse a `[MSG:key:value]` line into `(key, value)`.
pub fn parse_msg(line: &str) -> Option<(String, String)> {
    let content = line.strip_prefix("[MSG:")?;
    let content = content.trim_end_matches(']');
    let (key, value) = content.split_once(':')?;
    Some((key.trim().to_string(), value.trim().to_string()))
}

/// Parse a grblHAL `[BOARD:...]` build-info line.
pub fn parse_board(line: &str) -> Option<String> {
    let content = line.strip_prefix("[BOARD:")?;
    let board = content.strip_suffix(']').unwrap_or(content).trim();
    if board.is_empty() {
        None
    } else {
        Some(board.to_string())
    }
}

/// Extract a human-readable device name from build info lines.
///
/// Checks `[MSG:machine:...]` lines first, then grblHAL's
/// `[BOARD:...]` line, and finally falls back to the VER line's
/// build-info field (e.g. `[VER:1.1h.ORTUR:]` → `ORTUR`).
pub fn extract_device_name(build_info: &[String]) -> String {
    for line in build_info {
        if let Some((key, value)) = parse_msg(line) {
            let key = key.to_ascii_lowercase();
            if key == "machine" || key == "mechine" {
                return value;
            }
        }
    }
    for line in build_info {
        if let Some(board) = parse_board(line) {
            return board;
        }
    }
    for line in build_info {
        if let Some((_, Some(build_name))) = parse_ver(line) {
            return build_name;
        }
    }
    "Unknown Grbl Device".to_string()
}

/// Extract a human-readable device name from raw serial output, as
/// captured during device discovery.
///
/// Returns the machine name when the output carries one, otherwise
/// the first informative banner line, and `None` when there is no
/// usable line at all.
pub fn extract_device_name_from_output(data: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let name = extract_device_name(&lines);
    if name != "Unknown Grbl Device" {
        return Some(name);
    }
    extract_grbl_banner(&lines)
}

fn extract_grbl_banner(lines: &[String]) -> Option<String> {
    for raw in lines {
        let line = raw.trim();
        if line.is_empty() || is_grbl_ack(line) {
            continue;
        }
        if line.starts_with('<') && line.ends_with('>') {
            continue;
        }
        return Some(line.chars().take(80).collect());
    }
    None
}

fn is_grbl_ack(line: &str) -> bool {
    let stripped = line.trim_start_matches('\u{fffd}').trim_start_matches('\0');
    stripped == "ok" || stripped.starts_with("error:")
}

/// Determines if a GRBL version supports single-axis homing.
///
/// Support is assumed for versions > 1.1 or for 1.1g and newer.
pub fn version_supports_single_axis_homing(
    version_num: f64,
    version_letter: &str,
) -> bool {
    if version_num > 1.1 {
        return true;
    }
    if (version_num - 1.1).abs() < f64::EPSILON {
        return !version_letter.is_empty()
            && version_letter.to_ascii_lowercase().as_str() >= "g";
    }
    false
}

/// Extract the RX buffer size from an
/// `[OPT:<flags>,<planner_buffer_blocks>,<rx_buffer_size>]` line.
pub fn parse_opt_info(line: &str) -> Option<i64> {
    GRBL_OPT_RE
        .captures(line)
        .and_then(|caps| caps[3].parse::<i64>().ok())
}

/// Extract the compile option flags from an
/// `[OPT:<flags>,<planner_buffer_blocks>,<rx_buffer_size>]` line.
pub fn parse_opt_flags(line: &str) -> Option<String> {
    GRBL_OPT_RE.captures(line).map(|caps| caps[1].to_string())
}

/// Parses the response from a '$G' command to find the active WCS.
/// Example response: `[G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]`
pub fn parse_grbl_parser_state(response_lines: &[String]) -> Option<String> {
    for line in response_lines {
        if let Some(caps) = GRBL_PARSER_STATE_RE.captures(line) {
            return Some(caps[1].to_string());
        }
    }
    None
}

/// Parse a `[G5x:x,y,z]` line from `$#` output into
/// `(slot, (x, y, z))`.
pub fn parse_wcs_line(line: &str) -> Option<(String, (f64, f64, f64))> {
    let caps = WCS_RE.captures(line)?;
    let x: f64 = caps[2].parse().ok()?;
    let y: f64 = caps[3].parse().ok()?;
    let z: f64 = match caps.get(4) {
        Some(m) => m.as_str().parse().ok()?,
        None => 0.0,
    };
    Some((caps[1].to_string(), (x, y, z)))
}

/// Parse a `[PRB:x,y,z:success]` line into `(pos, success)`.
pub fn parse_probe_line(line: &str) -> Option<((f64, f64, f64), bool)> {
    let caps = PRB_RE.captures(line)?;
    let x: f64 = caps[1].parse().ok()?;
    let y: f64 = caps[2].parse().ok()?;
    let z: f64 = caps[3].parse().ok()?;
    let success = &caps[4] == "1";
    Some(((x, y, z), success))
}

fn parse_status_part(status_part: &str) -> (DeviceStatus, Option<String>) {
    let mut parts = status_part.splitn(2, ':');
    let status_name = parts.next().unwrap_or("");
    let status = DeviceStatus::from_name(status_name);
    // Only `Alarm` uses the `:N` suffix for an actual error code.
    // For `Hold` and `Door` it is a sub-state indicator that must
    // NOT be treated as an error.
    let error_code = if status == DeviceStatus::Alarm {
        parts.next().map(str::to_string)
    } else {
        None
    };
    (status, error_code)
}

fn parse_position_attribute(attrib: &str, pos_type: &str) -> Option<Pos> {
    if !attrib.starts_with(&format!("{pos_type}:")) {
        return None;
    }
    POS_RE.captures(attrib).and_then(|caps| {
        let mut values: Vec<Option<f64>> = Vec::with_capacity(4);
        for group in 1..=4 {
            let value = match caps.get(group) {
                Some(m) => m.as_str().parse::<f64>().ok(),
                None => Some(0.0),
            };
            match value {
                Some(v) => values.push(Some(v)),
                None => return None,
            }
        }
        if caps.get(4).is_none() {
            values.truncate(3);
        }
        Some(values)
    })
}

fn parse_feed_rate(attrib: &str) -> Option<i64> {
    let caps = FS_RE.captures(attrib)?;
    caps[1].parse().ok()
}

fn parse_buffer_state(attrib: &str) -> Option<(i64, i64)> {
    let caps = BF_RE.captures(attrib)?;
    let available = caps[1].parse().ok()?;
    let rx_available = caps[2].parse().ok()?;
    Some((available, rx_available))
}

fn pos_from_inches(pos: &Pos) -> Pos {
    pos.iter().map(|v| v.map(inches_to_mm)).collect()
}

fn pad_pos(pos: &Pos, n: usize, default: f64) -> Vec<Option<f64>> {
    let mut result = pos.clone();
    while result.len() < n {
        result.push(Some(default));
    }
    result
}

fn all_known(pos: &[Option<f64>]) -> bool {
    pos.iter().all(|v| v.is_some())
}

/// Recalculate positions based on the GRBL equations for
/// consistency.  Also infers WCO if missing but both MPos and WPos
/// are present.
fn recalculate_positions(
    machine_pos: Pos,
    work_pos: Pos,
    wco: Pos,
    mpos_found: bool,
    wpos_found: bool,
    wco_found: bool,
) -> (Pos, Pos, Pos) {
    let n = machine_pos.len().max(work_pos.len()).max(wco.len()).max(3);
    let machine_pos = pad_pos(&machine_pos, n, 0.0);
    let mut work_pos = pad_pos(&work_pos, n, 0.0);
    let mut wco = pad_pos(&wco, n, 0.0);

    // 1. Infer WCO if explicitly missing but both MPos and WPos
    //    exist: WCO = MPos - WPos.
    if mpos_found
        && wpos_found
        && !wco_found
        && all_known(&machine_pos)
        && all_known(&work_pos)
    {
        wco = (0..n)
            .map(|i| Some(machine_pos[i].unwrap() - work_pos[i].unwrap()))
            .collect();
    }

    // 2. Recalculate missing positions based on what we have.
    if mpos_found && all_known(&machine_pos) && all_known(&wco) {
        work_pos = (0..n)
            .map(|i| Some(machine_pos[i].unwrap() - wco[i].unwrap()))
            .collect();
        return (machine_pos, work_pos, wco);
    }
    if wpos_found && all_known(&work_pos) && all_known(&wco) {
        let mpos = (0..n)
            .map(|i| Some(work_pos[i].unwrap() + wco[i].unwrap()))
            .collect();
        return (mpos, work_pos, wco);
    }
    (machine_pos, work_pos, wco)
}

/// Parse a GRBL status string like
/// `<Idle|MPos:10.0,20.0,30.0|WPos:0,0,0>` into a [`DeviceState`],
/// using `default` as the base.
///
/// When `report_in_inches` is true (GRBL `$13` set), positions are
/// converted back to mm.
pub fn parse_state(
    state_str: &str,
    default: &DeviceState,
    report_in_inches: bool,
) -> DeviceState {
    let mut state = default.clone();
    let inner: &str = match state_str
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
    {
        Some(inner) => inner,
        None => return state,
    };

    let mut status_part = "";
    let mut attribs: Vec<&str> = Vec::new();
    for part in inner.split('|') {
        if part.is_empty() {
            continue;
        }
        if status_part.is_empty() {
            status_part = part;
        } else {
            attribs.push(part);
        }
    }

    if !status_part.is_empty() {
        let (status, error_code) = parse_status_part(status_part);
        state.status = status;
        if let Some(code) = error_code {
            state.error = Some(if status == DeviceStatus::Alarm {
                alarm_code_to_device_error(&code)
            } else {
                error_code_to_device_error(&code)
            });
        }
    }

    let mut mpos_found = false;
    let mut wpos_found = false;
    let mut wco_found = false;
    for attrib in attribs {
        if let Some(parsed) = parse_position_attribute(attrib, "MPos") {
            mpos_found = parsed.first().is_some_and(|v| v.is_some());
            state.machine_pos = parsed;
        } else if let Some(parsed) = parse_position_attribute(attrib, "WPos") {
            wpos_found = parsed.first().is_some_and(|v| v.is_some());
            state.work_pos = parsed;
        } else if let Some(parsed) = parse_position_attribute(attrib, "WCO") {
            wco_found = true;
            state.wco = parsed;
        } else if attrib.starts_with("FS:") {
            if let Some(feed_rate) = parse_feed_rate(attrib) {
                state.feed_rate = Some(feed_rate);
            }
        } else if attrib.starts_with("Bf:") {
            if let Some((available, rx_available)) = parse_buffer_state(attrib)
            {
                state.buffer_available = Some(available);
                state.buffer_rx_available = Some(rx_available);
            }
        }
    }

    if report_in_inches {
        if mpos_found {
            state.machine_pos = pos_from_inches(&state.machine_pos);
        }
        if wpos_found {
            state.work_pos = pos_from_inches(&state.work_pos);
        }
        if wco_found {
            state.wco = pos_from_inches(&state.wco);
        }
    }

    let (mpos, wpos, wco) = recalculate_positions(
        state.machine_pos.clone(),
        state.work_pos.clone(),
        state.wco.clone(),
        mpos_found,
        wpos_found,
        wco_found,
    );
    state.machine_pos = mpos;
    state.work_pos = wpos;
    state.wco = wco;
    state
}

/// Returns the [`DeviceError`] for a given GRBL error code string.
pub fn get_error(error_code: &str) -> DeviceError {
    error_code_to_device_error(error_code)
}
