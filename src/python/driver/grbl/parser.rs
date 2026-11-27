//! Python bindings for the pure response parsers.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::driver::core::types::UnitSystem;
use crate::driver::grbl::errors::{
    alarm_code_to_device_error as alarm_lookup,
    error_code_to_device_error as error_lookup,
};
use crate::driver::grbl::parser;

use crate::python::driver::core::types::{DeviceError, DeviceState};

pub(crate) const MODULE_DOC: &str = "\
Pure parsers for GRBL response data: status reports, build info, \
settings, WCS offsets, probe results and raw-output classification. \
All functions are side-effect free.
";

pyo3_stub_gen::module_doc!("raygeo.driver.grbl.parser", "{}", MODULE_DOC);

fn to_state(state: crate::driver::core::types::DeviceState) -> DeviceState {
    DeviceState::new(state)
}

/// Parse a GRBL status string like `<Idle|MPos:10,20,30>` into a
/// DeviceState, using *default* as the base.  When
/// *report_in_inches* is true, positions are converted back to mm.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
#[pyo3(signature = (state_str, default=None, report_in_inches=false))]
fn parse_state(
    state_str: &str,
    default: Option<DeviceState>,
    report_in_inches: bool,
) -> DeviceState {
    let default = default.map(|d| d.inner).unwrap_or_default();
    to_state(parser::parse_state(state_str, &default, report_in_inches))
}

/// Parse `$I` response lines to extract the firmware version.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_version(response_lines: Vec<String>) -> Option<String> {
    parser::parse_version(&response_lines)
}

/// Parse a `[VER:...]` line into `(version, build_name)`.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_ver(line: &str) -> Option<(String, Option<String>)> {
    parser::parse_ver(line)
}

/// Extract the RX buffer size from an `[OPT:...]` line.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_opt_info(line: &str) -> Option<i64> {
    parser::parse_opt_info(line)
}

/// Extract the compile option flags from an `[OPT:...]` line.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_opt_flags(line: &str) -> Option<String> {
    parser::parse_opt_flags(line)
}

/// Parse a `[MSG:key:value]` line into `(key, value)`.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_msg(line: &str) -> Option<(String, String)> {
    parser::parse_msg(line)
}

/// Parse a grblHAL `[BOARD:...]` build-info line.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_board(line: &str) -> Option<String> {
    parser::parse_board(line)
}

/// Extract a human-readable device name from build info lines.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn extract_device_name(build_info: Vec<String>) -> String {
    parser::extract_device_name(&build_info)
}

/// Extract a device name from raw serial output, falling back to
/// the first informative banner line.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn extract_device_name_from_output(data: Vec<u8>) -> Option<String> {
    parser::extract_device_name_from_output(&data)
}

/// True when raw serial output identifies a Grbl device.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn is_grbl_output(data: Vec<u8>) -> bool {
    parser::is_grbl_output(&data)
}

/// Strip G-code comments from a line.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn strip_gcode_comments(line: &str) -> String {
    parser::strip_gcode_comments(line)
}

/// Split command lines into `(gcode, realtime)` groups.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn split_realtime_commands(lines: Vec<String>) -> (Vec<String>, Vec<String>) {
    parser::split_realtime_commands(&lines)
}

/// Convert a G-code WCS name (e.g. "G54") to its P-number.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn gcode_to_p_number(wcs_slot: &str) -> Option<i64> {
    parser::gcode_to_p_number(wcs_slot)
}

/// True if a GRBL version supports single-axis homing (> 1.1 or
/// 1.1g and newer).
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
#[pyo3(signature = (version_num, version_letter=""))]
fn version_supports_single_axis_homing(
    version_num: f64,
    version_letter: &str,
) -> bool {
    parser::version_supports_single_axis_homing(version_num, version_letter)
}

/// Parse `$$` lines into `{key: value}` floats.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_grbl_settings(
    lines: Vec<String>,
) -> std::collections::HashMap<String, f64> {
    parser::parse_grbl_settings(&lines)
}

/// Parse `$$` lines into ordered `(key, raw_value)` pairs.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_setting_pairs(lines: Vec<String>) -> Vec<(String, String)> {
    parser::parse_setting_pairs(&lines)
}

/// Parse a `[G5x:x,y,z]` line from `$#` output.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_wcs_line(line: &str) -> Option<(String, (f64, f64, f64))> {
    parser::parse_wcs_line(line)
}

/// Parse a `[PRB:x,y,z:success]` line into `(pos, success)`.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_probe_line(line: &str) -> Option<((f64, f64, f64), bool)> {
    parser::parse_probe_line(line)
}

/// Parse `$G` response lines to find the active WCS (G54-G59).
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn parse_grbl_parser_state(response_lines: Vec<String>) -> Option<String> {
    parser::parse_grbl_parser_state(&response_lines)
}

/// Infer the unit system ("metric"/"imperial") from `$$` lines.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn detect_unit_system_from_settings(
    settings_lines: Vec<String>,
) -> Option<&'static str> {
    match parser::detect_unit_system_from_settings(&settings_lines) {
        Some(UnitSystem::Metric) => Some("metric"),
        Some(UnitSystem::Imperial) => Some("imperial"),
        None => None,
    }
}

/// True when the `$13` (Report in inches) flag is set.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn is_report_in_inches(settings_lines: Vec<String>) -> bool {
    parser::is_report_in_inches(&settings_lines)
}

/// Look up an error code in the GRBL error table.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn error_code_to_device_error(error_code: &str) -> DeviceError {
    DeviceError::new(error_lookup(error_code))
}

/// Look up an alarm code in the GRBL alarm table.
#[gen_stub_pyfunction(module = "raygeo.driver.grbl.parser")]
#[pyfunction]
fn alarm_code_to_device_error(alarm_code: &str) -> DeviceError {
    DeviceError::new(alarm_lookup(alarm_code))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__doc__", MODULE_DOC)?;
    m.add(
        "__all__",
        vec![
            "parse_state",
            "parse_version",
            "parse_ver",
            "parse_opt_info",
            "parse_opt_flags",
            "parse_msg",
            "parse_board",
            "extract_device_name",
            "extract_device_name_from_output",
            "is_grbl_output",
            "strip_gcode_comments",
            "split_realtime_commands",
            "gcode_to_p_number",
            "version_supports_single_axis_homing",
            "parse_grbl_settings",
            "parse_setting_pairs",
            "parse_wcs_line",
            "parse_probe_line",
            "parse_grbl_parser_state",
            "detect_unit_system_from_settings",
            "is_report_in_inches",
            "error_code_to_device_error",
            "alarm_code_to_device_error",
        ],
    )?;
    m.add_function(pyo3::wrap_pyfunction!(parse_state, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_version, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_ver, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_opt_info, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_opt_flags, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_msg, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_board, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(extract_device_name, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        extract_device_name_from_output,
        m.clone()
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(is_grbl_output, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(strip_gcode_comments, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        split_realtime_commands,
        m.clone()
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(gcode_to_p_number, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        version_supports_single_axis_homing,
        m.clone()
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_grbl_settings, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_setting_pairs, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_wcs_line, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(parse_probe_line, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        parse_grbl_parser_state,
        m.clone()
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        detect_unit_system_from_settings,
        m.clone()
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(is_report_in_inches, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        error_code_to_device_error,
        m.clone()
    )?)?;
    m.add_function(pyo3::wrap_pyfunction!(
        alarm_code_to_device_error,
        m.clone()
    )?)?;
    Ok(())
}
