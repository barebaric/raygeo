use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::convert::gcode_types::GcodeDialectSpec;

/// Typed Python wrapper around the Rust ``GcodeDialectSpec``.
///
/// Constructed in Python with keyword arguments; the inner Rust
/// struct is passed directly to the encoder without a serde round-trip.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.ops.convert",
    name = "GcodeDialectSpec",
    frozen,
    skip_from_py_object
)]
pub struct PyGcodeDialectSpec(pub GcodeDialectSpec);

#[gen_stub_pymethods]
#[pymethods]
impl PyGcodeDialectSpec {
    #[new]
    #[pyo3(signature = (
        laser_on = "M4 S{power:.0f}".to_string(),
        laser_off = "M5".to_string(),
        tool_change = "T{tool_number}".to_string(),
        set_speed = "G1 F{speed:.0f}".to_string(),
        travel_move = "G0{x_cmd}{y_cmd}{z_cmd}{extra_cmd}{f_command}{s_command}".to_string(),
        linear_move = "G1{x_cmd}{y_cmd}{z_cmd}{extra_cmd}{f_command}{s_command}".to_string(),
        arc_cw = "G2{x_cmd}{y_cmd}{z_cmd} I{i} J{j}{extra_cmd}{f_command}{s_command}".to_string(),
        arc_ccw = "G3{x_cmd}{y_cmd}{z_cmd} I{i} J{j}{extra_cmd}{f_command}{s_command}".to_string(),
        air_assist_on = "M8".to_string(),
        air_assist_off = "M9".to_string(),
        bezier_cubic = String::new(),
        spindle_on_cw = "M3 S{rpm}".to_string(),
        spindle_on_ccw = "M4 S{rpm}".to_string(),
        spindle_off = "M5".to_string(),
        coolant_flood = "M8".to_string(),
        coolant_mist = "M7".to_string(),
        coolant_off = "M9".to_string(),
        dwell = "G4 P{seconds:.3f}".to_string(),
        preamble = vec!["G90".to_string()],
        postscript = vec!["M30".to_string()],
        inject_wcs_after_preamble = false,
        can_g0_with_speed = false,
        omit_unchanged_coords = true,
        continuous_laser_mode = false,
        modal_feedrate = false,
        gcode_precision = 3u8,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        laser_on: String,
        laser_off: String,
        tool_change: String,
        set_speed: String,
        travel_move: String,
        linear_move: String,
        arc_cw: String,
        arc_ccw: String,
        air_assist_on: String,
        air_assist_off: String,
        bezier_cubic: String,
        spindle_on_cw: String,
        spindle_on_ccw: String,
        spindle_off: String,
        coolant_flood: String,
        coolant_mist: String,
        coolant_off: String,
        dwell: String,
        preamble: Vec<String>,
        postscript: Vec<String>,
        inject_wcs_after_preamble: bool,
        can_g0_with_speed: bool,
        omit_unchanged_coords: bool,
        continuous_laser_mode: bool,
        modal_feedrate: bool,
        gcode_precision: u8,
    ) -> Self {
        PyGcodeDialectSpec(GcodeDialectSpec {
            laser_on,
            laser_off,
            tool_change,
            set_speed,
            travel_move,
            linear_move,
            arc_cw,
            arc_ccw,
            bezier_cubic,
            air_assist_on,
            air_assist_off,
            spindle_on_cw,
            spindle_on_ccw,
            spindle_off,
            coolant_flood,
            coolant_mist,
            coolant_off,
            dwell,
            preamble,
            postscript,
            inject_wcs_after_preamble,
            can_g0_with_speed,
            omit_unchanged_coords,
            continuous_laser_mode,
            modal_feedrate,
            gcode_precision,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "GcodeDialectSpec(laser_on={:?}, laser_off={:?}, ...)",
            self.0.laser_on, self.0.laser_off,
        )
    }
}
