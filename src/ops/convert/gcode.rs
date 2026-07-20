//! G-code encoder: converts an ``Ops`` sequence into a G-code string.
//!
//! Handles modal speed/power tracking, coordinate omission, laser
//! safety, macro emission, and op-map building.  Operates on the
//! ``Ops`` commands vector directly without crossing the PyO3
//! boundary per command.

use std::collections::{HashMap, HashSet};

use serde::de::DeserializeOwned;

use crate::fstring::{
    parse_include_directive, render_named, resolve_path_vars, NamedVars,
};
use crate::ops::axis::Axis;
use crate::ops::cache::Cacheable;
use crate::ops::container::Ops;
use crate::ops::convert::gcode_types::{
    EncodeContext, EncodeResult, GcodeDialectSpec, Macro, MacroTable,
};
use crate::ops::convert::{EncodeCtx, EncodeOutput, Encoder};
use crate::ops::enums::CommandType;
use crate::ops::state::{AirAssistMode, CoolantMode};
use crate::ops::types::MoveCmd;
use crate::types::Point3D;

const COORD_TOLERANCE: f64 = 1e-6;

/// Format a float with the given number of decimal places.
fn format_with_precision(value: f64, precision: u8) -> String {
    format!("{:.*}", precision as usize, value)
}

/// Small fixed-size map keyed by the single-axis bit position.
///
/// The `Axis` bitflags type is not `Hash`, so we index by the bit
/// position (0..7). X/Y/Z are stored inline; extras (A/B/C/U) use
/// a small vec.
#[derive(Clone, Default)]
pub(crate) struct AxisMap {
    x: f64,
    y: f64,
    z: f64,
    extras: Vec<(Axis, f64)>,
}

impl AxisMap {
    fn get(&self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
            _ => self
                .extras
                .iter()
                .find_map(|(a, v)| (*a == axis).then_some(*v))
                .unwrap_or(0.0),
        }
    }

    fn set(&mut self, axis: Axis, value: f64) {
        match axis {
            Axis::X => self.x = value,
            Axis::Y => self.y = value,
            Axis::Z => self.z = value,
            _ => {
                if let Some(slot) =
                    self.extras.iter_mut().find(|(a, _)| *a == axis)
                {
                    slot.1 = value;
                } else {
                    self.extras.push((axis, value));
                }
            }
        }
    }
}

/// Encoder state machine.
///
/// Encapsulates all the per-job mutable tracking (current power, speed,
/// laser on/off, position, etc.). One instance encodes exactly one
/// \(Ops\) sequence into a \([`EncodeResult`]\).
pub(crate) struct GcodeEncoder<'a> {
    pub(crate) dialect: &'a GcodeDialectSpec,
    pub(crate) ctx: &'a EncodeContext,

    pub(crate) power: Option<f64>,
    pub(crate) cut_speed: Option<f64>,
    pub(crate) travel_speed: Option<f64>,
    pub(crate) emitted_speed: Option<f64>,
    #[allow(dead_code)]
    pub(crate) emitted_power: Option<f64>,
    pub(crate) emitted_cut_feed: Option<f64>,
    pub(crate) air_assist: bool,
    pub(crate) laser_active: bool,
    pub(crate) active_laser_uid: Option<String>,
    pub(crate) frequency: Option<i32>,
    pub(crate) pulse_width: Option<f64>,
    pub(crate) spindle_rpm: u32,
    pub(crate) coolant_mode: Option<CoolantMode>,
    pub(crate) current_pos: AxisMap,
    pub(crate) active_wcs: Option<String>,
    pub(crate) path_vars: HashMap<String, String>,

    pub(crate) gcode: Vec<String>,
    pub(crate) op_to_machine_code: HashMap<usize, Vec<usize>>,
    pub(crate) machine_code_to_op: HashMap<usize, usize>,
}

impl<'a> GcodeEncoder<'a> {
    pub(crate) fn new(
        dialect: &'a GcodeDialectSpec,
        ctx: &'a EncodeContext,
    ) -> Self {
        Self {
            dialect,
            ctx,
            power: None,
            cut_speed: None,
            travel_speed: None,
            emitted_speed: None,
            emitted_power: None,
            emitted_cut_feed: None,
            air_assist: false,
            laser_active: false,
            active_laser_uid: None,
            frequency: None,
            pulse_width: None,
            spindle_rpm: 0,
            coolant_mode: None,
            current_pos: AxisMap::default(),
            active_wcs: None,
            path_vars: ctx.path_vars.clone(),
            gcode: Vec::new(),
            op_to_machine_code: HashMap::new(),
            machine_code_to_op: HashMap::new(),
        }
    }

    fn format_coord(&self, value: f64) -> String {
        let v = if value.abs() < COORD_TOLERANCE {
            0.0
        } else {
            value
        };
        strip_trailing_zeros(&format_with_precision(v, self.fmt_precision()))
    }

    fn format_feed(&self, value: f64) -> String {
        strip_trailing_zeros(&format_with_precision(
            value,
            self.fmt_precision(),
        ))
    }

    fn format_power(&self, value: f64) -> String {
        strip_trailing_zeros(&format_with_precision(
            value,
            self.fmt_precision(),
        ))
    }

    fn fmt_precision(&self) -> u8 {
        self.ctx.gcode_precision.max(self.dialect.gcode_precision)
    }

    fn get_current_laser_head_uid(&mut self) -> Option<String> {
        if self.active_laser_uid.is_none() {
            self.active_laser_uid = Some(self.ctx.default_head_uid.clone());
        }
        self.active_laser_uid.clone()
    }

    fn max_power_for_head(&self, uid: &str) -> Option<f64> {
        self.ctx
            .heads
            .iter()
            .find(|h| h.uid == uid)
            .map(|h| h.max_power)
    }

    fn current_pos_xyz(&self) -> Point3D {
        Point3D::new(self.current_pos.x, self.current_pos.y, self.current_pos.z)
    }

    fn update_current_pos(&mut self, ops: &Ops, idx: usize) {
        let end = ops.endpoint(idx);
        self.current_pos.set(Axis::X, end.x);
        self.current_pos.set(Axis::Y, end.y);
        self.current_pos.set(Axis::Z, end.z);
        if let Some(ea) = ops.extra_axes(idx) {
            for &(axis, value) in ea {
                self.current_pos.set(axis, value);
            }
        }
    }

    fn push_line(&mut self, line: impl Into<String>) {
        self.gcode.push(line.into());
    }

    fn record_op_lines(&mut self, op_idx: usize, start_line: usize) {
        let end_line = self.gcode.len();
        if end_line > start_line {
            self.op_to_machine_code
                .insert(op_idx, (start_line..end_line).collect());
            for ln in start_line..end_line {
                self.machine_code_to_op.insert(ln, op_idx);
            }
        } else {
            self.op_to_machine_code.insert(op_idx, Vec::new());
        }
    }

    fn emit_macros(&mut self, macro_block: Option<&Macro>) {
        let Some(m) = macro_block.filter(|m| m.enabled) else {
            return;
        };
        let mut call_stack = HashSet::new();
        let lines =
            expand_macro(m, &self.ctx.macros, &self.path_vars, &mut call_stack);
        for line in lines {
            self.push_line(line);
        }
    }

    fn format_script_lines(&self, lines: &[String]) -> Vec<String> {
        lines
            .iter()
            .map(|l| resolve_path_vars(l, &self.path_vars))
            .collect()
    }

    fn build_coord_commands(
        &self,
        x: f64,
        y: f64,
        z: f64,
        extra_axes: Option<&[(Axis, f64)]>,
    ) -> CoordVars {
        let prev_x = self.current_pos.x;
        let prev_y = self.current_pos.y;
        let prev_z = self.current_pos.z;

        let x_cmd = format!(" X{}", self.format_coord(x));
        let y_cmd = format!(" Y{}", self.format_coord(y));
        let z_cmd = format!(" Z{}", self.format_coord(z));

        let mut extra_cmd = String::new();
        if let Some(ea) = extra_axes {
            for &(axis, value) in ea {
                let letter = letter_for_axis(axis);
                let prev_val = self.current_pos.get(axis);
                let mut formatted =
                    format!(" {letter}{}", self.format_coord(value));
                if self.dialect.omit_unchanged_coords
                    && (value - prev_val).abs() < 1e-12
                {
                    formatted.clear();
                }
                extra_cmd.push_str(&formatted);
            }
        }

        let (x_cmd, y_cmd, z_cmd) = if self.dialect.omit_unchanged_coords {
            let x_changed = (x - prev_x).abs() > 1e-12;
            let y_changed = (y - prev_y).abs() > 1e-12;
            let z_changed = (z - prev_z).abs() > 1e-12;
            let none_changed = !x_changed && !y_changed && !z_changed;

            (
                if x_changed || none_changed {
                    x_cmd
                } else {
                    String::new()
                },
                if y_changed { y_cmd } else { String::new() },
                if z_changed { z_cmd } else { String::new() },
            )
        } else {
            (x_cmd, y_cmd, z_cmd)
        };

        CoordVars {
            x: self.format_coord(x),
            y: self.format_coord(y),
            z: self.format_coord(z),
            x_cmd,
            y_cmd,
            z_cmd,
            extra_cmd,
        }
    }

    fn emit_modal_speed(&mut self, speed: f64) {
        if !self.dialect.set_speed.is_empty()
            && Some(speed) != self.emitted_speed
        {
            let mut vars = NamedVars::default();
            vars.set_num("speed", speed);
            let out = render_named(&self.dialect.set_speed, &vars);
            if !out.is_empty() {
                self.push_line(out);
            }
            self.emitted_speed = Some(speed);
        }
    }

    fn laser_on(&mut self) {
        if !self.laser_active {
            let needs_emit = self.power.unwrap_or(0.0) > 0.0
                || self.dialect.continuous_laser_mode;
            if needs_emit {
                if let Some(uid) = self.get_current_laser_head_uid() {
                    let max_power =
                        self.max_power_for_head(&uid).unwrap_or(0.0);
                    let power_abs = self.power.unwrap_or(0.0) * max_power;
                    let mut vars = NamedVars::default();
                    vars.set_num("power", power_abs);
                    let out = render_named(&self.dialect.laser_on, &vars);
                    if !out.is_empty() {
                        self.push_line(out);
                    }
                }
                self.laser_active = true;
            }
        }
    }

    fn laser_off(&mut self) {
        if self.laser_active && !self.dialect.continuous_laser_mode {
            if !self.dialect.laser_off.is_empty() {
                self.push_line(self.dialect.laser_off.clone());
            }
            self.laser_active = false;
        }
    }

    fn update_power(&mut self, power: f64) {
        if let Some(p) = self.power {
            if (power - p).abs() < 1e-12 {
                return;
            }
        }
        self.power = Some(power);

        if self.laser_active && !self.dialect.continuous_laser_mode {
            if power > 0.0 {
                if let Some(uid) = self.get_current_laser_head_uid() {
                    let max_power =
                        self.max_power_for_head(&uid).unwrap_or(0.0);
                    let power_abs = power * max_power;
                    let mut vars = NamedVars::default();
                    vars.set_num("power", power_abs);
                    let out = render_named(&self.dialect.laser_on, &vars);
                    if !out.is_empty() {
                        self.push_line(out);
                    }
                }
            } else {
                self.laser_off();
            }
        }
    }

    fn handle_air_assist(&mut self, mode: AirAssistMode) {
        let on = mode == AirAssistMode::On;
        if self.air_assist == on {
            return;
        }
        self.air_assist = on;
        let cmd = if on {
            &self.dialect.air_assist_on
        } else {
            &self.dialect.air_assist_off
        };
        if !cmd.is_empty() {
            self.push_line(cmd.clone());
        }
    }

    fn handle_coolant(&mut self, mode: CoolantMode) {
        if Some(mode) == self.coolant_mode {
            return;
        }
        let cmd = match mode {
            CoolantMode::Flood => &self.dialect.coolant_flood,
            CoolantMode::Mist => &self.dialect.coolant_mist,
            CoolantMode::Off => &self.dialect.coolant_off,
        };
        if !cmd.is_empty() {
            self.push_line(cmd.clone());
        }
        self.coolant_mode = Some(mode);
    }

    fn handle_spindle(&mut self, rpm: u32) {
        if rpm > 0 {
            if self.spindle_rpm > 0
                && rpm != self.spindle_rpm
                && !self.dialect.spindle_off.is_empty()
            {
                self.push_line(self.dialect.spindle_off.clone());
            }
            let mut vars = NamedVars::default();
            vars.set_str("rpm", &rpm.to_string());
            let out = render_named(&self.dialect.spindle_on_cw, &vars);
            if !out.is_empty() {
                self.push_line(out);
            }
        } else if self.spindle_rpm > 0 && !self.dialect.spindle_off.is_empty() {
            self.push_line(self.dialect.spindle_off.clone());
        }
        self.spindle_rpm = rpm;
    }

    fn handle_set_laser(&mut self, laser_uid: &str) {
        if self.active_laser_uid.as_deref() == Some(laser_uid) {
            return;
        }
        let tool_number = self
            .ctx
            .heads
            .iter()
            .find(|h| h.uid == laser_uid)
            .map(|h| h.tool_number);
        if let Some(tn) = tool_number {
            let mut vars = NamedVars::default();
            vars.set_str("tool_number", &tn.to_string());
            let out = render_named(&self.dialect.tool_change, &vars);
            if !out.is_empty() {
                self.push_line(out);
            }
            self.active_laser_uid = Some(laser_uid.to_string());
        }
    }

    fn build_cut_move_vars(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra_axes: Option<&[(Axis, f64)]>,
    ) -> NamedVars {
        self.laser_on();
        let cut_speed = self.cut_speed.unwrap_or(0.0);
        self.emit_modal_speed(cut_speed);

        let f_command = if self.dialect.modal_feedrate {
            let needs_emit = self.emitted_cut_feed.is_none()
                || (Some(self.cut_speed.unwrap_or(0.0))
                    != self.emitted_cut_feed)
                    && (self
                        .cut_speed
                        .map(|s| {
                            (s - self.emitted_cut_feed.unwrap_or(-1.0)).abs()
                                > 1e-12
                        })
                        .unwrap_or(false));
            if let Some(cs) = self.cut_speed {
                if needs_emit {
                    let formatted = self.format_feed(cs);
                    self.emitted_cut_feed = Some(cs);
                    format!(" F{formatted}")
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            self.cut_speed
                .map(|s| format!(" F{}", self.format_feed(s)))
                .unwrap_or_default()
        };

        let (power_abs, s_command) = match self.power {
            Some(p) if p > 0.0 || self.dialect.continuous_laser_mode => {
                let uid = self.get_current_laser_head_uid().unwrap_or_default();
                let max_power = self.max_power_for_head(&uid).unwrap_or(0.0);
                let abs = p * max_power;
                let formatted = self.format_power(abs);
                (abs, format!(" S{formatted}"))
            }
            _ => (0.0, String::new()),
        };

        let coord_vars = self.build_coord_commands(x, y, z, extra_axes);
        let mut vars = NamedVars::default();
        vars.set_str("x", &coord_vars.x);
        vars.set_str("y", &coord_vars.y);
        vars.set_str("z", &coord_vars.z);
        vars.set_str("x_cmd", &coord_vars.x_cmd);
        vars.set_str("y_cmd", &coord_vars.y_cmd);
        vars.set_str("z_cmd", &coord_vars.z_cmd);
        vars.set_str("extra_cmd", &coord_vars.extra_cmd);
        vars.set_str("f_command", &f_command);
        vars.set_str("s_command", &s_command);
        vars.set_num("power", power_abs);
        vars
    }

    fn handle_move_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra_axes: Option<&[(Axis, f64)]>,
    ) {
        self.laser_off();

        let coord_vars = self.build_coord_commands(x, y, z, extra_axes);
        let mut vars = NamedVars::default();
        vars.set_str("x", &coord_vars.x);
        vars.set_str("y", &coord_vars.y);
        vars.set_str("z", &coord_vars.z);
        vars.set_str("x_cmd", &coord_vars.x_cmd);
        vars.set_str("y_cmd", &coord_vars.y_cmd);
        vars.set_str("z_cmd", &coord_vars.z_cmd);
        vars.set_str("extra_cmd", &coord_vars.extra_cmd);

        let mut f_command = String::new();
        if self.dialect.can_g0_with_speed {
            let travel = self.travel_speed.unwrap_or(0.0);
            self.emit_modal_speed(travel);
            if let Some(ts) = self.travel_speed {
                let formatted = self.format_feed(ts);
                f_command = format!(" F{formatted}");
                self.emitted_cut_feed = None;
            }
            vars.set_str("f_command", &f_command);
        } else {
            vars.set_str("f_command", "");
        }

        let s_command =
            if self.laser_active && self.dialect.continuous_laser_mode {
                " S0".to_string()
            } else {
                String::new()
            };
        vars.set_str("s_command", &s_command);

        let out = render_named(&self.dialect.travel_move, &vars);
        self.push_line(out);
    }

    fn handle_line_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra_axes: Option<&[(Axis, f64)]>,
    ) {
        let vars = self.build_cut_move_vars(x, y, z, extra_axes);
        let out = render_named(&self.dialect.linear_move, &vars);
        self.push_line(out);
    }

    fn handle_arc_to(
        &mut self,
        end: Point3D,
        center: (f64, f64),
        cw: bool,
        extra_axes: Option<&[(Axis, f64)]>,
    ) {
        let template = if cw {
            &self.dialect.arc_cw
        } else {
            &self.dialect.arc_ccw
        };
        let mut vars =
            self.build_cut_move_vars(end.x, end.y, end.z, extra_axes);
        vars.set_str("i", &self.format_coord(center.0));
        vars.set_str("j", &self.format_coord(center.1));
        let out = render_named(template, &vars);
        self.push_line(out);
    }

    fn handle_bezier_to(
        &mut self,
        ops: &Ops,
        idx: usize,
        extra_axes: Option<&[(Axis, f64)]>,
    ) {
        if self.dialect.bezier_cubic.is_empty() {
            // Linearize and emit as line segments.
            let start = self.current_pos_xyz();
            let sub_ops = ops.linearize(idx, start);
            for j in 0..sub_ops.len() {
                if sub_ops.command_type(j) == CommandType::LineTo {
                    let end = sub_ops.endpoint(j);
                    let sub_ea = sub_ops.extra_axes(j);
                    self.handle_line_to(end.x, end.y, end.z, sub_ea);
                }
            }
            return;
        }

        let start = self.current_pos_xyz();
        let end = ops.endpoint(idx);
        let (c1, c2) = ops_bezier_params(ops, idx);
        let mut vars =
            self.build_cut_move_vars(end.x, end.y, end.z, extra_axes);
        vars.set_str("i", &self.format_coord(c1.x - start.x));
        vars.set_str("j", &self.format_coord(c1.y - start.y));
        vars.set_str("p", &self.format_coord(c2.x - end.x));
        vars.set_str("q", &self.format_coord(c2.y - end.y));
        let out = render_named(&self.dialect.bezier_cubic, &vars);
        self.push_line(out);
    }

    fn handle_moving(&mut self, ops: &Ops, idx: usize, ct: CommandType) {
        let ea = ops.extra_axes(idx);
        match ct {
            CommandType::MoveTo => {
                let end = ops.endpoint(idx);
                self.handle_move_to(end.x, end.y, end.z, ea);
                self.update_current_pos(ops, idx);
            }
            CommandType::LineTo => {
                let end = ops.endpoint(idx);
                self.handle_line_to(end.x, end.y, end.z, ea);
                self.update_current_pos(ops, idx);
            }
            CommandType::ArcTo => {
                let end = ops.endpoint(idx);
                let (i, j, cw) = ops_arc_params(ops, idx);
                self.handle_arc_to(end, (i, j), cw, ea);
                self.update_current_pos(ops, idx);
            }
            CommandType::BezierTo => {
                self.handle_bezier_to(ops, idx, ea);
                self.update_current_pos(ops, idx);
            }
            CommandType::ScanLine => {
                let start = self.current_pos_xyz();
                let sub_ops = ops.linearize(idx, start);
                for j in 0..sub_ops.len() {
                    self.handle_command(&sub_ops, j);
                }
                // Avoid float precision errors: explicitly set final pos.
                self.update_current_pos(ops, idx);
            }
            _ => {}
        }
    }

    fn handle_command(&mut self, ops: &Ops, idx: usize) {
        let ct = ops.command_type(idx);

        match ct {
            CommandType::SetPower => {
                self.update_power(ops_power(ops, idx));
            }
            CommandType::SetFeedRate => {
                self.cut_speed = Some(ops_rate(ops, idx) as f64);
            }
            CommandType::SetRapidRate => {
                let raw = ops_rate(ops, idx) as f64;
                self.travel_speed = Some(raw.min(self.ctx.max_travel_speed));
            }
            CommandType::SetFrequency => {
                self.frequency = Some(ops_frequency(ops, idx));
            }
            CommandType::SetPulseWidth => {
                self.pulse_width = Some(ops_pulse_width(ops, idx));
            }
            CommandType::SetAirAssist => {
                self.handle_air_assist(ops_air_assist(ops, idx));
            }
            CommandType::SetCoolant => {
                self.handle_coolant(ops_coolant(ops, idx));
            }
            CommandType::SetSpindleRpm => {
                self.handle_spindle(ops_spindle_rpm(ops, idx));
            }
            CommandType::SetHead => {
                if let Some(uid) = ops_head_uid(ops, idx) {
                    self.handle_set_laser(&uid);
                }
            }
            CommandType::Dwell => {
                if !self.dialect.dwell.is_empty() {
                    let duration = ops_dwell(ops, idx);
                    let mut vars = NamedVars::default();
                    vars.set_str("seconds", &format!("{}", duration / 1000.0));
                    vars.set_str("milliseconds", &format!("{}", duration));
                    let out = render_named(&self.dialect.dwell, &vars);
                    if !out.is_empty() {
                        self.push_line(out);
                    }
                }
            }
            CommandType::JobStart => {
                // 1. Preamble
                let lines = self.format_script_lines(&self.dialect.preamble);
                self.gcode.extend(lines);

                // 2. Active WCS after preamble.
                if self.dialect.inject_wcs_after_preamble
                    && !self.ctx.active_wcs.is_empty()
                {
                    self.push_line(self.ctx.active_wcs.clone());
                    self.active_wcs = Some(self.ctx.active_wcs.clone());
                }
            }
            CommandType::JobEnd => {
                self.laser_off();
                if self.air_assist {
                    self.handle_air_assist(AirAssistMode::Off);
                }
                if self.spindle_rpm > 0 {
                    self.handle_spindle(0);
                }
                if let Some(cm) = self.coolant_mode {
                    if cm != CoolantMode::Off {
                        self.handle_coolant(CoolantMode::Off);
                    }
                }
                let lines = self.format_script_lines(&self.dialect.postscript);
                self.gcode.extend(lines);
            }
            CommandType::LayerStart => {
                let uid = ops_layer_uid(ops, idx);
                if let Some(uid) = uid {
                    // Update path_vars with layer-specific variables
                    if let Some(vars) = self.ctx.layer_path_vars.get(&uid) {
                        self.path_vars.extend(
                            vars.iter().map(|(k, v)| (k.clone(), v.clone())),
                        );
                    }
                    if let Some(wcs) = self.ctx.layer_wcs.get(&uid) {
                        if self.dialect.inject_wcs_after_preamble
                            && Some(wcs.as_str()) != self.active_wcs.as_deref()
                        {
                            if !wcs.is_empty() {
                                self.push_line(wcs.clone());
                            }
                            self.active_wcs = Some(wcs.clone());
                        }
                    }
                }
                self.emit_macros(self.ctx.macros.layer_start.as_ref());
            }
            CommandType::LayerEnd => {
                self.emit_macros(self.ctx.macros.layer_end.as_ref());
            }
            CommandType::WorkpieceStart => {
                let uid = ops_workpiece_uid(ops, idx);
                if let Some(uid) = uid {
                    if let Some(vars) = self.ctx.workpiece_path_vars.get(&uid) {
                        self.path_vars.extend(
                            vars.iter().map(|(k, v)| (k.clone(), v.clone())),
                        );
                    }
                }
                self.emit_macros(self.ctx.macros.workpiece_start.as_ref());
            }
            CommandType::WorkpieceEnd => {
                self.emit_macros(self.ctx.macros.workpiece_end.as_ref());
            }
            _ => {
                if ct.category() == crate::ops::enums::CommandCategory::Moving {
                    self.handle_moving(ops, idx, ct);
                }
            }
        }
    }

    fn finalize(&mut self) {
        let needs_trailing_empty = match self.gcode.last() {
            Some(line) => !line.is_empty(),
            None => false,
        };
        if needs_trailing_empty {
            self.gcode.push(String::new());
        }
    }
}

struct CoordVars {
    x: String,
    y: String,
    z: String,
    x_cmd: String,
    y_cmd: String,
    z_cmd: String,
    extra_cmd: String,
}

fn strip_trailing_zeros(s: &str) -> String {
    if s.find('.').is_some() {
        let stripped = s.trim_end_matches('0');
        let stripped = stripped.trim_end_matches('.');
        if stripped.is_empty() || stripped == "-" {
            s.to_string()
        } else {
            stripped.to_string()
        }
    } else {
        s.to_string()
    }
}

fn letter_for_axis(axis: Axis) -> &'static str {
    match axis {
        Axis::A => "A",
        Axis::B => "B",
        Axis::C => "C",
        Axis::U => "U",
        Axis::X => "X",
        Axis::Y => "Y",
        Axis::Z => "Z",
        _ => "?",
    }
}

// --- Ops access helpers ---

fn ops_power(ops: &Ops, idx: usize) -> f64 {
    if let crate::ops::types::OpCategory::State(
        crate::ops::types::StateCmd::SetPower(p),
    ) = &ops.commands[idx].category
    {
        *p
    } else {
        0.0
    }
}

fn ops_rate(ops: &Ops, idx: usize) -> i32 {
    let node = &ops.commands[idx];
    match &node.category {
        crate::ops::types::OpCategory::State(
            crate::ops::types::StateCmd::SetFeedRate(r),
        ) => *r,
        crate::ops::types::OpCategory::State(
            crate::ops::types::StateCmd::SetRapidRate(r),
        ) => *r,
        _ => 0,
    }
}

fn ops_frequency(ops: &Ops, idx: usize) -> i32 {
    if let crate::ops::types::OpCategory::State(
        crate::ops::types::StateCmd::SetFrequency(f),
    ) = &ops.commands[idx].category
    {
        *f
    } else {
        0
    }
}

fn ops_pulse_width(ops: &Ops, idx: usize) -> f64 {
    if let crate::ops::types::OpCategory::State(
        crate::ops::types::StateCmd::SetPulseWidth(p),
    ) = &ops.commands[idx].category
    {
        *p
    } else {
        0.0
    }
}

fn ops_air_assist(ops: &Ops, idx: usize) -> AirAssistMode {
    if let crate::ops::types::OpCategory::State(
        crate::ops::types::StateCmd::SetAirAssist(m),
    ) = &ops.commands[idx].category
    {
        *m
    } else {
        AirAssistMode::Off
    }
}

fn ops_coolant(ops: &Ops, idx: usize) -> CoolantMode {
    if let crate::ops::types::OpCategory::State(
        crate::ops::types::StateCmd::SetCoolant(m),
    ) = &ops.commands[idx].category
    {
        *m
    } else {
        CoolantMode::Off
    }
}

fn ops_spindle_rpm(ops: &Ops, idx: usize) -> u32 {
    if let crate::ops::types::OpCategory::State(
        crate::ops::types::StateCmd::SetSpindleRpm(r),
    ) = &ops.commands[idx].category
    {
        *r
    } else {
        0
    }
}

fn ops_head_uid(ops: &Ops, idx: usize) -> Option<String> {
    if let crate::ops::types::OpCategory::State(
        crate::ops::types::StateCmd::SetHead(h),
    ) = &ops.commands[idx].category
    {
        Some(h.to_string())
    } else {
        None
    }
}

fn ops_dwell(ops: &Ops, idx: usize) -> f64 {
    if let crate::ops::types::OpCategory::State(
        crate::ops::types::StateCmd::Dwell(ms),
    ) = &ops.commands[idx].category
    {
        *ms
    } else {
        0.0
    }
}

fn ops_layer_uid(ops: &Ops, idx: usize) -> Option<String> {
    let node = &ops.commands[idx];
    if let crate::ops::types::OpCategory::Marker(
        crate::ops::types::MarkerCmd::LayerStart(uid),
    ) = &node.category
    {
        return Some(uid.to_string());
    }
    None
}

fn ops_workpiece_uid(ops: &Ops, idx: usize) -> Option<String> {
    let node = &ops.commands[idx];
    if let crate::ops::types::OpCategory::Marker(
        crate::ops::types::MarkerCmd::WorkpieceStart(uid),
    ) = &node.category
    {
        return Some(uid.to_string());
    }
    None
}

fn ops_arc_params(ops: &Ops, idx: usize) -> (f64, f64, bool) {
    if let crate::ops::types::OpCategory::Moving {
        cmd: MoveCmd::ArcTo { center, cw },
        ..
    } = &ops.commands[idx].category
    {
        (center.x, center.y, *cw)
    } else {
        (0.0, 0.0, true)
    }
}

fn ops_bezier_params(ops: &Ops, idx: usize) -> (Point3D, Point3D) {
    if let crate::ops::types::OpCategory::Moving {
        cmd: MoveCmd::BezierTo { control1, control2 },
        ..
    } = &ops.commands[idx].category
    {
        (*control1, *control2)
    } else {
        (Point3D::ZERO, Point3D::ZERO)
    }
}

/// Expand a single macro: process `@include(name)` directives and
/// resolve path-style variables on each line against `path_vars`.
///
/// Matches `TemplateFormatter._recursive_expand`: recursive, with a
/// `call_stack` set for cycle detection. Disabled or missing
/// referenced macros emit a `; WARNING:` line.
pub(crate) fn expand_macro(
    macro_block: &Macro,
    table: &MacroTable,
    path_vars: &HashMap<String, String>,
    call_stack: &mut HashSet<String>,
) -> Vec<String> {
    if call_stack.contains(&macro_block.name) {
        return vec![format!(
            "; ERROR: Circular dependency detected. Macro '{}' was included again.",
            macro_block.name
        )];
    }
    call_stack.insert(macro_block.name.clone());

    let mut out = Vec::with_capacity(macro_block.code.len());

    for line in &macro_block.code {
        if let Some(name) = parse_include_directive(line) {
            let found = table.all_macros.get(&name);
            if let Some(found) = found {
                if found.enabled {
                    let inner =
                        expand_macro(found, table, path_vars, call_stack);
                    out.extend(inner);
                } else {
                    out.push(format!(
                        "; WARNING: Macro '{}' not found or disabled.",
                        name
                    ));
                }
            } else {
                out.push(format!(
                    "; WARNING: Macro '{}' not found or disabled.",
                    name
                ));
            }
        } else {
            let formatted = resolve_path_vars(line, path_vars);
            out.push(formatted);
        }
    }

    call_stack.remove(&macro_block.name);
    out
}

/// Entry point: encode an \(Ops\) sequence into
/// \([`EncodeResult`]\).
pub fn encode_gcode(
    ops: &Ops,
    dialect: &GcodeDialectSpec,
    ctx: &EncodeContext,
) -> EncodeResult {
    let mut enc = GcodeEncoder::new(dialect, ctx);

    for i in 0..ops.len() {
        let start_line = enc.gcode.len();
        enc.handle_command(ops, i);
        enc.record_op_lines(i, start_line);
    }

    enc.finalize();

    EncodeResult {
        text: enc.gcode.join("\n"),
        op_to_machine_code: enc.op_to_machine_code,
        machine_code_to_op: enc.machine_code_to_op,
    }
}

/// Spec for the G-code encoder.
///
/// `context_json` is a JSON-serialised
/// [`EncodeContext`](crate::ops::convert::gcode_types::EncodeContext).
/// The context is deserialised inside [`Encoder::encode`] so the
/// spec stays cheap to construct and serialise across the Python
/// boundary.
#[derive(Clone, Debug)]
pub struct GcodeSpec {
    /// Dialect templates.
    pub dialect: GcodeDialectSpec,
    /// JSON-serialised `EncodeContext`.
    pub context_json: String,
}

impl Encoder for GcodeSpec {
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String> {
        ctx.callbacks.report_progress(0.0, "gcode: parse context");
        let context: EncodeContext = parse_context(&self.context_json)?;

        ctx.callbacks.report_progress(0.5, "gcode: encode");
        let result = encode_gcode(ctx.ops, &self.dialect, &context);

        ctx.callbacks.report_progress(1.0, "gcode: done");
        Ok(EncodeOutput::MachineCode {
            text: result.text,
            op_to_machine_code: result.op_to_machine_code,
            machine_code_to_op: result.machine_code_to_op,
        })
    }

    fn name(&self) -> &'static str {
        "gcode"
    }
}

impl Cacheable<EncodeOutput> for GcodeSpec {}

/// Parse a JSON-serialised `EncodeContext`.
fn parse_context<T: DeserializeOwned>(json: &str) -> Result<T, String> {
    serde_json::from_str(json)
        .map_err(|e| format!("failed to deserialise encode context: {e}"))
}
