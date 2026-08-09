//! G-code encoder: converts an ``Ops`` sequence into a G-code string.
//!
//! Handles modal speed/power tracking, coordinate omission, laser
//! safety, macro emission, and op-map building.  Operates on the
//! ``Ops`` commands vector directly without crossing the PyO3
//! boundary per command.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;

use serde::de::DeserializeOwned;

use crate::fstring::{
    parse_include_directive, render_named, render_named_into,
    resolve_path_vars, NamedVars,
};
use crate::geo::types::Point3D;
use crate::ops::axis::Axis;
use crate::ops::callbacks::Callbacks;
use crate::ops::container::Ops;
use crate::ops::convert::gcode_types::{
    EncodeContext, EncodeResult, GcodeDialectSpec, Macro, MacroTable,
    OpLineRange, MACHINE_CODE_TO_OP_NONE,
};
use crate::ops::convert::{EncodeCtx, EncodeOutput, Encoder};
use crate::ops::enums::CommandType;
use crate::ops::state::{AirAssistMode, CoolantMode};
use crate::ops::types::MoveCmd;

const COORD_TOLERANCE: f64 = 1e-6;

/// Format a float into *buf* with the given decimal places, then strip
/// trailing zeros the same way ``strip_trailing_zeros`` did.  Writes
/// directly into the caller's buffer so a per-line ``String``
/// allocation is avoided.
fn write_scaled(buf: &mut String, value: f64, precision: u8) {
    let start = buf.len();
    write!(buf, "{:.*}", precision as usize, value)
        .expect("writing into a String cannot fail");
    let segment = &buf[start..];
    if segment.contains('.') {
        let trimmed = segment.trim_end_matches('0');
        let trimmed = trimmed.trim_end_matches('.');
        if !(trimmed.is_empty() || trimmed == "-") {
            buf.truncate(start + trimmed.len());
        }
    }
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

    pub(crate) gcode: String,
    pub(crate) line_count: usize,
    pub(crate) line_buf: String,
    pub(crate) var_pool: Vec<String>,
    pub(crate) vars: NamedVars,
    pub(crate) op_to_machine_code: Vec<OpLineRange>,
    pub(crate) machine_code_to_op: Vec<usize>,
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
            gcode: String::new(),
            line_count: 0,
            line_buf: String::new(),
            var_pool: Vec::new(),
            vars: NamedVars::default(),
            op_to_machine_code: Vec::new(),
            machine_code_to_op: Vec::new(),
        }
    }

    /// Take a scratch string from the pool, or allocate a fresh one.
    fn take_var(&mut self) -> String {
        self.var_pool.pop().unwrap_or_default()
    }

    /// Return a scratch string to the pool (cleared, bounded capacity).
    fn give_var(&mut self, mut value: String) {
        value.clear();
        if value.capacity() <= 256 {
            self.var_pool.push(value);
        }
    }

    /// Zero out values inside the coordinate tolerance, then write
    /// them into *buf* at the given scale.
    fn write_coord(&self, buf: &mut String, value: f64) {
        let scaled = value * self.ctx.unit_scale;
        let v = if scaled.abs() < COORD_TOLERANCE {
            0.0
        } else {
            scaled
        };
        write_scaled(buf, v, self.fmt_precision());
    }

    /// Write an angular value (rotary axis position in degrees).
    /// Angular values are unit-system agnostic and are never scaled
    /// by ``unit_scale``.
    fn write_angular(&self, buf: &mut String, value: f64) {
        let v = if value.abs() < COORD_TOLERANCE {
            0.0
        } else {
            value
        };
        write_scaled(buf, v, self.fmt_precision());
    }

    fn write_feed(&self, buf: &mut String, value: f64) {
        let scaled = value * self.ctx.unit_scale;
        write_scaled(buf, scaled, self.fmt_precision());
    }

    fn write_power(&self, buf: &mut String, value: f64) {
        write_scaled(buf, value, self.fmt_precision());
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

    fn push_line(&mut self, line: &str) {
        self.gcode.push_str(line);
        self.gcode.push('\n');
        self.line_count += 1;
    }

    fn record_op_lines(&mut self, op_idx: usize, start_line: usize) {
        let end_line = self.line_count;
        self.op_to_machine_code.push(OpLineRange {
            start: start_line as u32,
            len: (end_line - start_line) as u32,
        });
        self.machine_code_to_op
            .resize(end_line, MACHINE_CODE_TO_OP_NONE);
        for ln in start_line..end_line {
            self.machine_code_to_op[ln] = op_idx;
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
            self.push_line(&line);
        }
    }

    fn format_script_lines(&self, lines: &[String]) -> Vec<String> {
        lines
            .iter()
            .map(|l| resolve_path_vars(l, &self.path_vars))
            .collect()
    }

    fn build_coord_commands(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra_axes: Option<&[(Axis, f64)]>,
    ) {
        let prev_x = self.current_pos.x;
        let prev_y = self.current_pos.y;
        let prev_z = self.current_pos.z;

        let mut x_val = self.take_var();
        self.write_coord(&mut x_val, x);
        let mut y_val = self.take_var();
        self.write_coord(&mut y_val, y);
        let mut z_val = self.take_var();
        self.write_coord(&mut z_val, z);

        let mut x_cmd = self.take_var();
        write!(x_cmd, " X{x_val}").expect("writing into a String");
        let mut y_cmd = self.take_var();
        write!(y_cmd, " Y{y_val}").expect("writing into a String");
        let mut z_cmd = self.take_var();
        write!(z_cmd, " Z{z_val}").expect("writing into a String");

        let mut extra_cmd = self.take_var();
        if let Some(ea) = extra_axes {
            for &(axis, value) in ea {
                let letter = letter_for_axis(axis);
                let prev_val = self.current_pos.get(axis);
                let start = extra_cmd.len();
                extra_cmd.push(' ');
                extra_cmd.push_str(letter);
                self.write_angular(&mut extra_cmd, value);
                if self.dialect.omit_unchanged_coords
                    && (value - prev_val).abs() < 1e-12
                {
                    extra_cmd.truncate(start);
                }
            }
        }

        if self.dialect.omit_unchanged_coords {
            let x_changed = (x - prev_x).abs() > 1e-12;
            let y_changed = (y - prev_y).abs() > 1e-12;
            let z_changed = (z - prev_z).abs() > 1e-12;
            let none_changed = !x_changed && !y_changed && !z_changed;
            if !(x_changed || none_changed) {
                self.give_var(x_cmd);
                x_cmd = String::new();
            }
            if !y_changed {
                self.give_var(y_cmd);
                y_cmd = String::new();
            }
            if !z_changed {
                self.give_var(z_cmd);
                z_cmd = String::new();
            }
        }

        self.vars.set("x", x_val);
        self.vars.set("y", y_val);
        self.vars.set("z", z_val);
        self.vars.set("x_cmd", x_cmd);
        self.vars.set("y_cmd", y_cmd);
        self.vars.set("z_cmd", z_cmd);
        self.vars.set("extra_cmd", extra_cmd);
    }

    fn emit_modal_speed(&mut self, speed: f64) {
        let dialect = self.dialect;
        if !dialect.set_speed.is_empty() && Some(speed) != self.emitted_speed {
            let scaled = speed * self.ctx.unit_scale;
            let mut vars = NamedVars::default();
            vars.set_num("speed", scaled);
            let out = render_named(&dialect.set_speed, &vars);
            if !out.is_empty() {
                self.push_line(&out);
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
                    let dialect = self.dialect;
                    let out = render_named(&dialect.laser_on, &vars);
                    if !out.is_empty() {
                        self.push_line(&out);
                    }
                }
                self.laser_active = true;
            }
        }
    }

    fn laser_off(&mut self) {
        let dialect = self.dialect;
        if self.laser_active && !dialect.continuous_laser_mode {
            if !dialect.laser_off.is_empty() {
                self.push_line(&dialect.laser_off);
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
                    let dialect = self.dialect;
                    let out = render_named(&dialect.laser_on, &vars);
                    if !out.is_empty() {
                        self.push_line(&out);
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
        let dialect = self.dialect;
        let cmd = if on {
            &dialect.air_assist_on
        } else {
            &dialect.air_assist_off
        };
        if !cmd.is_empty() {
            self.push_line(cmd);
        }
    }

    fn handle_coolant(&mut self, mode: CoolantMode) {
        if Some(mode) == self.coolant_mode {
            return;
        }
        let dialect = self.dialect;
        let cmd = match mode {
            CoolantMode::Flood => &dialect.coolant_flood,
            CoolantMode::Mist => &dialect.coolant_mist,
            CoolantMode::Off => &dialect.coolant_off,
        };
        if !cmd.is_empty() {
            self.push_line(cmd);
        }
        self.coolant_mode = Some(mode);
    }

    fn handle_spindle(&mut self, rpm: u32) {
        let dialect = self.dialect;
        if rpm > 0 {
            if self.spindle_rpm > 0
                && rpm != self.spindle_rpm
                && !dialect.spindle_off.is_empty()
            {
                self.push_line(&dialect.spindle_off);
            }
            let mut vars = NamedVars::default();
            vars.set_str("rpm", &rpm.to_string());
            let out = render_named(&dialect.spindle_on_cw, &vars);
            if !out.is_empty() {
                self.push_line(&out);
            }
        } else if self.spindle_rpm > 0 && !dialect.spindle_off.is_empty() {
            self.push_line(&dialect.spindle_off);
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
            let dialect = self.dialect;
            let out = render_named(&dialect.tool_change, &vars);
            if !out.is_empty() {
                self.push_line(&out);
            }
            self.active_laser_uid = Some(laser_uid.to_string());
        }
    }
    /// Render ``self.vars`` through *template* into the reusable line
    /// buffer, emit it, then return every var string to the pool and
    /// clear the vars bag (keeping its capacity).
    fn emit_line(&mut self, template: &str) {
        let mut line = std::mem::take(&mut self.line_buf);
        line.clear();
        render_named_into(&mut line, template, &self.vars);
        self.push_line(&line);
        self.line_buf = line;
        let pairs = std::mem::take(&mut self.vars.pairs);
        for (_, value) in pairs {
            self.give_var(value);
        }
    }

    fn build_cut_move_vars(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra_axes: Option<&[(Axis, f64)]>,
    ) {
        self.laser_on();
        let cut_speed = self.cut_speed.unwrap_or(0.0);
        self.emit_modal_speed(cut_speed);

        let mut f_command = self.take_var();
        if self.dialect.modal_feedrate {
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
                    f_command.push_str(" F");
                    self.write_feed(&mut f_command, cs);
                    self.emitted_cut_feed = Some(cs);
                }
            }
        } else if let Some(s) = self.cut_speed {
            f_command.push_str(" F");
            self.write_feed(&mut f_command, s);
        }

        let mut s_command = self.take_var();
        let mut power_abs = 0.0;
        if let Some(p) = self.power {
            if p > 0.0 || self.dialect.continuous_laser_mode {
                let uid = self.get_current_laser_head_uid().unwrap_or_default();
                let max_power = self.max_power_for_head(&uid).unwrap_or(0.0);
                power_abs = p * max_power;
                s_command.push_str(" S");
                self.write_power(&mut s_command, power_abs);
            }
        }

        self.build_coord_commands(x, y, z, extra_axes);
        self.vars.set("f_command", f_command);
        self.vars.set("s_command", s_command);
        let mut power = self.take_var();
        write!(power, "{power_abs}").expect("writing into a String");
        self.vars.set("power", power);
    }

    fn handle_move_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra_axes: Option<&[(Axis, f64)]>,
    ) {
        self.laser_off();
        self.vars.pairs.clear();

        self.build_coord_commands(x, y, z, extra_axes);

        let mut f_command = self.take_var();
        if self.dialect.can_g0_with_speed {
            let travel = self.travel_speed.unwrap_or(0.0);
            self.emit_modal_speed(travel);
            if let Some(ts) = self.travel_speed {
                f_command.push_str(" F");
                self.write_feed(&mut f_command, ts);
                self.emitted_cut_feed = None;
            }
        }
        self.vars.set("f_command", f_command);

        let mut s_command = self.take_var();
        if self.laser_active && self.dialect.continuous_laser_mode {
            s_command.push_str(" S0");
        }
        self.vars.set("s_command", s_command);

        let dialect = self.dialect;
        self.emit_line(&dialect.travel_move);
    }

    fn handle_line_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra_axes: Option<&[(Axis, f64)]>,
    ) {
        self.vars.pairs.clear();
        self.build_cut_move_vars(x, y, z, extra_axes);
        let dialect = self.dialect;
        self.emit_line(&dialect.linear_move);
    }

    fn handle_arc_to(
        &mut self,
        end: Point3D,
        center: (f64, f64),
        cw: bool,
        extra_axes: Option<&[(Axis, f64)]>,
    ) {
        self.vars.pairs.clear();
        let dialect = self.dialect;
        let template = if cw {
            &dialect.arc_cw
        } else {
            &dialect.arc_ccw
        };
        self.build_cut_move_vars(end.x, end.y, end.z, extra_axes);
        let mut i = self.take_var();
        self.write_coord(&mut i, center.0);
        self.vars.set("i", i);
        let mut j = self.take_var();
        self.write_coord(&mut j, center.1);
        self.vars.set("j", j);
        self.emit_line(template);
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

        self.vars.pairs.clear();
        let start = self.current_pos_xyz();
        let end = ops.endpoint(idx);
        let (c1, c2) = ops_bezier_params(ops, idx);
        self.build_cut_move_vars(end.x, end.y, end.z, extra_axes);
        let mut i = self.take_var();
        self.write_coord(&mut i, c1.x - start.x);
        self.vars.set("i", i);
        let mut j = self.take_var();
        self.write_coord(&mut j, c1.y - start.y);
        self.vars.set("j", j);
        let mut p = self.take_var();
        self.write_coord(&mut p, c2.x - end.x);
        self.vars.set("p", p);
        let mut q = self.take_var();
        self.write_coord(&mut q, c2.y - end.y);
        self.vars.set("q", q);
        let dialect = self.dialect;
        self.emit_line(&dialect.bezier_cubic);
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
                let dialect = self.dialect;
                if !dialect.dwell.is_empty() {
                    let duration = ops_dwell(ops, idx);
                    let mut vars = NamedVars::default();
                    vars.set_str("seconds", &format!("{}", duration / 1000.0));
                    vars.set_str("milliseconds", &format!("{}", duration));
                    let out = render_named(&dialect.dwell, &vars);
                    if !out.is_empty() {
                        self.push_line(&out);
                    }
                }
            }
            CommandType::JobStart => {
                // 1. Preamble
                let lines = self.format_script_lines(&self.dialect.preamble);
                for line in lines {
                    self.push_line(&line);
                }

                // 2. Active WCS after preamble.
                let dialect = self.dialect;
                if dialect.inject_wcs_after_preamble
                    && !self.ctx.active_wcs.is_empty()
                {
                    let wcs = self.ctx.active_wcs.clone();
                    self.push_line(&wcs);
                    self.active_wcs = Some(wcs);
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
                for line in lines {
                    self.push_line(&line);
                }
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
                        let dialect = self.dialect;
                        if dialect.inject_wcs_after_preamble
                            && Some(wcs.as_str()) != self.active_wcs.as_deref()
                        {
                            if !wcs.is_empty() {
                                self.push_line(&wcs.clone());
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
        // The old line-array encoder joined lines with ``\n`` and then
        // appended a trailing empty line, so the text always ended with
        // exactly one ``\n``.  The single buffer already appends ``\n``
        // per line; a trailing empty line leaves ``\n\n``, which the
        // join would have collapsed to ``\n``.
        if self.gcode.ends_with("\n\n") {
            self.gcode.pop();
        }
        self.machine_code_to_op
            .resize(self.line_count, MACHINE_CODE_TO_OP_NONE);
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
    callbacks: &dyn Callbacks,
) -> Result<EncodeResult, String> {
    let mut enc = GcodeEncoder::new(dialect, ctx);
    // Pre-size the output buffer so the per-line push_str calls don't
    // trigger a long reallocation cascade as the text grows.  32 bytes
    // is a rough per-command average; the buffer still grows if needed.
    enc.gcode.reserve(ops.len().saturating_mul(32));

    for i in 0..ops.len() {
        if i % 1000 == 0 && callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let start_line = enc.line_count;
        enc.handle_command(ops, i);
        enc.record_op_lines(i, start_line);
    }

    enc.finalize();

    Ok(EncodeResult {
        text: enc.gcode,
        op_to_machine_code: enc.op_to_machine_code,
        machine_code_to_op: enc.machine_code_to_op,
    })
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
        let result =
            encode_gcode(ctx.ops, &self.dialect, &context, ctx.callbacks)?;

        ctx.callbacks.report_progress(1.0, "gcode: done");
        Ok(EncodeOutput::MachineCode {
            text: result.text,
            op_to_machine_code: result.op_to_machine_code,
            machine_code_to_op: result.machine_code_to_op,
        })
    }

    fn name(&self) -> &str {
        "gcode"
    }
}

/// Parse a JSON-serialised `EncodeContext`.
fn parse_context<T: DeserializeOwned>(json: &str) -> Result<T, String> {
    serde_json::from_str(json)
        .map_err(|e| format!("failed to deserialise encode context: {e}"))
}
