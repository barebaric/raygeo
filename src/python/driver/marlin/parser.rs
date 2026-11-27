//! Python bindings for the Marlin response parsers.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::driver::marlin::parser;
use std::collections::HashMap;

pub(crate) const MODULE_DOC: &str = "\
Pure parsers for Marlin response data: ok/error classification, \
boot banners, M114 positions, M115 firmware info and M149 units. \
All functions are side-effect free.
";

pyo3_stub_gen::module_doc!("raygeo.driver.marlin.parser", "{}", MODULE_DOC);

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.setattr("__doc__", MODULE_DOC)?;
    m.add(
        "__all__",
        vec![
            "is_ok_response",
            "is_error_response",
            "is_boot_message",
            "is_marlin_output",
            "extract_marlin_banner_from_output",
            "parse_m114_position",
            "parse_m114_line",
            "is_m114_position",
            "parse_marlin_version",
            "parse_error_message",
            "parse_m115_firmware_info",
            "parse_m211_endstops",
            "parse_m503_settings",
            "extract_marlin_device_name",
            "detect_unit_system_from_m149",
        ],
    )?;

    m.add_function(wrap_pyfunction!(is_ok_response, m)?)?;
    m.add_function(wrap_pyfunction!(is_error_response, m)?)?;
    m.add_function(wrap_pyfunction!(is_boot_message, m)?)?;
    m.add_function(wrap_pyfunction!(is_marlin_output, m)?)?;
    m.add_function(wrap_pyfunction!(extract_marlin_banner_from_output, m)?)?;
    m.add_function(wrap_pyfunction!(parse_m114_position, m)?)?;
    m.add_function(wrap_pyfunction!(parse_m114_line, m)?)?;
    m.add_function(wrap_pyfunction!(is_m114_position, m)?)?;
    m.add_function(wrap_pyfunction!(parse_marlin_version, m)?)?;
    m.add_function(wrap_pyfunction!(parse_error_message, m)?)?;
    m.add_function(wrap_pyfunction!(parse_m115_firmware_info, m)?)?;
    m.add_function(wrap_pyfunction!(parse_m211_endstops, m)?)?;
    m.add_function(wrap_pyfunction!(parse_m503_settings, m)?)?;
    m.add_function(wrap_pyfunction!(extract_marlin_device_name, m)?)?;
    m.add_function(wrap_pyfunction!(detect_unit_system_from_m149, m)?)?;
    Ok(())
}

/// True when *line* is a Marlin `ok` acknowledgement (plain or with
/// temperature payload).
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn is_ok_response(line: &str) -> bool {
    parser::is_ok(line)
}

/// True when *line* is a Marlin error response (`Error:...`).
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn is_error_response(line: &str) -> bool {
    parser::is_error(line)
}

/// True when *line* is a Marlin boot/startup message.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn is_boot_message(line: &str) -> bool {
    parser::is_boot_message(line)
}

/// True when raw serial output identifies a Marlin device.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn is_marlin_output(data: Vec<u8>) -> bool {
    parser::is_marlin_output(&data)
}

/// Extract the first boot message line from raw serial output, or
/// None.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn extract_marlin_banner_from_output(data: Vec<u8>) -> Option<String> {
    parser::extract_marlin_banner_from_output(&data)
}

/// Parse M114 output lines into an (x, y, z) position, or None.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn parse_m114_position(lines: Vec<String>) -> Option<(f64, f64, f64)> {
    parser::parse_m114_position(&lines)
}

/// Parse a single M114 position line, or None.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn parse_m114_line(line: &str) -> Option<(f64, f64, f64)> {
    parser::parse_m114_line(line)
}

/// True when *line* carries an M114 position report.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn is_m114_position(line: &str) -> bool {
    parser::is_m114_position(line)
}

/// Extract the Marlin version from a boot line like
/// `Marlin 2.1.2.7`, or None.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn parse_marlin_version(line: &str) -> Option<String> {
    parser::parse_marlin_version(line)
}

/// Extract the message portion of an `Error:...` line.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn parse_error_message(line: &str) -> String {
    parser::parse_error_message(line)
}

/// Parse M115 response lines into firmware name and machine type.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn parse_m115_firmware_info(lines: Vec<String>) -> HashMap<String, String> {
    parser::parse_m115_firmware_info(&lines)
        .into_iter()
        .collect()
}

/// Parse M211 output into (x_max, y_max) software endstop
/// travel, or None.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn parse_m211_endstops(lines: Vec<String>) -> Option<(f64, f64)> {
    parser::parse_m211_endstops(&lines)
}

/// Parse M503 output into key motion settings.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn parse_m503_settings(lines: Vec<String>) -> HashMap<String, f64> {
    parser::parse_m503_settings(&lines).into_iter().collect()
}

/// Extract a human-readable device name from M115 and boot output.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn extract_marlin_device_name(
    m115_lines: Vec<String>,
    boot_lines: Option<Vec<String>>,
) -> String {
    parser::extract_marlin_device_name(
        &m115_lines,
        &boot_lines.unwrap_or_default(),
    )
}

/// Infer the device unit system from M149 response lines
/// ("metric"/"imperial"), or None.
#[gen_stub_pyfunction(module = "raygeo.driver.marlin.parser")]
#[pyfunction]
fn detect_unit_system_from_m149(lines: Vec<String>) -> Option<&'static str> {
    match parser::detect_unit_system_from_m149(&lines) {
        Some(crate::driver::core::types::UnitSystem::Metric) => Some("metric"),
        Some(crate::driver::core::types::UnitSystem::Imperial) => {
            Some("imperial")
        }
        None => None,
    }
}
