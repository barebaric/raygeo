//! Encode machine-space Ops as a Ruida rpascript job.
//!
//! Ruida controllers run a binary job format. The encoder drives the
//! native GlueScript authoring model: jobs and layers are declared,
//! settings and movements are recorded as a canonical transcript, and
//! the transcript is compiled into rpascript. The transcript is the
//! readable listing (`text`, with the op map); the compiled rpascript
//! is the byte stream the controller consumes (`payload`).
//!
//! Ruida has no arc or bezier command, so curves are linearized into
//! cut segments with per-segment power. Image/depth layers use
//! immediate power; vector layers use a power ramp whose minimum rises
//! as the cut speed falls, gated by the job's power policy.

use std::collections::HashMap;

use serde::Deserialize;

use crate::geo::types::Point3D;
use crate::ops::container::Ops;
use crate::ops::convert::{
    EncodeCtx, EncodeOutput, Encoder, OpLineRange, MACHINE_CODE_TO_OP_NONE,
};
use crate::ops::enums::{CommandType, RasterMode, SectionType};
use crate::ops::state::{AirAssistMode, CoolantMode, PowerMode, State};
use crate::ops::types::{MarkerCmd, OpCategory, StateCmd};

use crate::ops::convert::gluescript::GlueScript;

const OVERSCAN_ANGLE_TOLERANCE_DEG: f64 = 0.01;
const OVERSCAN_ANGLE_EPSILON: f64 = 1e-6;
const DEFAULT_JOB_LABEL: &str = "Rayforge Job";
const DEFAULT_POWER_FLOOR_PCT: f64 = 8.0;
const DEFAULT_IMAGE_POWER_BIAS_PCT: f64 = 8.0;

/// One document layer's settings, flattened by the caller.
#[derive(Clone, Debug, Deserialize)]
pub struct RuidaLayer {
    pub uid: String,
    pub name: String,
    pub color: String,
    /// Cut speed in mm/s.
    pub speed_mm_s: f64,
    /// Laser frequency in kHz.
    pub frequency_khz: f64,
    /// Maximum layer power in percent.
    pub power_pct: f64,
    /// Optional layer minimum power in percent.
    #[serde(default)]
    pub min_power_pct: Option<f64>,
}

/// JSON-serialised job configuration.
#[derive(Clone, Debug, Deserialize)]
pub struct RuidaContext {
    /// Job label shown on the controller.
    #[serde(default = "default_job_label")]
    pub job_label: String,
    /// Reference point: MACHINE, ABSOLUTE, CURRENT or SET_POINT.
    #[serde(default = "default_ref_point")]
    pub ref_point: String,
    /// Optional job power cap as a fraction (0..=1).
    #[serde(default)]
    pub power_cap: Option<f64>,
    /// Minimum firing power in percent.
    #[serde(default = "default_power_floor")]
    pub power_floor_pct: f64,
    /// Whether the vector power floor is enabled.
    #[serde(default)]
    pub power_floor_enabled: bool,
    /// Image power bias in percent.
    #[serde(default = "default_image_power_bias")]
    pub image_power_bias_pct: f64,
    /// Whether effective-min power scaling may be used.
    #[serde(default = "default_true")]
    pub power_scaling_enabled: bool,
    /// Laser head UID → 1-based device number.
    #[serde(default)]
    pub head_tool_numbers: HashMap<String, i64>,
    /// Document layers by UID.
    #[serde(default)]
    pub layers: Vec<RuidaLayer>,
}

fn default_job_label() -> String {
    DEFAULT_JOB_LABEL.to_string()
}

fn default_ref_point() -> String {
    "MACHINE".to_string()
}

fn default_power_floor() -> f64 {
    DEFAULT_POWER_FLOOR_PCT
}

fn default_image_power_bias() -> f64 {
    DEFAULT_IMAGE_POWER_BIAS_PCT
}

fn default_true() -> bool {
    true
}

/// Spec for the Ruida job encoder.
#[derive(Clone, Debug)]
pub struct RuidaSpec {
    /// JSON-serialised [`RuidaContext`].
    pub context_json: String,
}

impl Encoder for RuidaSpec {
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String> {
        ctx.callbacks.report_progress(0.0, "ruida: parse context");
        let context: RuidaContext = serde_json::from_str(&self.context_json)
            .map_err(|error| {
                format!("failed to deserialise ruida context: {error}")
            })?;
        ctx.callbacks.report_progress(0.25, "ruida: encode");
        let mut encoder = JobEncoder::new(&context);
        encoder.encode(ctx.ops, ctx.callbacks)?;
        ctx.callbacks.report_progress(1.0, "ruida: done");
        encoder.finish()
    }

    fn name(&self) -> &str {
        "ruida"
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DeclaredLayer {
    label: String,
    color: String,
    mode: String,
    overscan: String,
    speed: f64,
    frequency: f64,
    min_power: f64,
    max_power: f64,
}

/// Drives GlueScript from the Ops sequence.
struct JobEncoder<'a> {
    ctx: &'a RuidaContext,
    gs: GlueScript,
    op_count: usize,
    op_contributions: Vec<Vec<(usize, usize)>>,
    snapshot_len: usize,
    layer_key: usize,
    layer_uid: Option<String>,
    layer_declared: bool,
    declared_layer: Option<DeclaredLayer>,
    section_type: Option<SectionType>,
    section_raster_mode: Option<RasterMode>,
    layer_mode: String,
    overscan: String,
    power_fraction: f64,
    power_min_fraction: f64,
    emitted_min_fraction: f64,
    last_requested_power: Option<f64>,
    power_floor: f64,
    image_power_bias: f64,
    current_pos: Point3D,
    active_laser: i64,
    job_started: bool,
    job_ended: bool,
}

impl<'a> JobEncoder<'a> {
    fn new(ctx: &'a RuidaContext) -> Self {
        Self {
            ctx,
            gs: GlueScript::new(),
            op_count: 0,
            op_contributions: Vec::new(),
            snapshot_len: 0,
            layer_key: 0,
            layer_uid: None,
            layer_declared: false,
            declared_layer: None,
            section_type: None,
            section_raster_mode: None,
            layer_mode: "VECTOR".to_string(),
            overscan: "NONE".to_string(),
            power_fraction: 0.0,
            power_min_fraction: 0.0,
            emitted_min_fraction: 0.0,
            last_requested_power: None,
            power_floor: ctx.power_floor_pct / 100.0,
            image_power_bias: ctx.image_power_bias_pct / 100.0,
            current_pos: Point3D::new(0.0, 0.0, 0.0),
            active_laser: 1,
            job_started: false,
            job_ended: false,
        }
    }

    fn encode(
        &mut self,
        ops: &Ops,
        callbacks: &dyn crate::ops::callbacks::Callbacks,
    ) -> Result<(), String> {
        self.op_count = ops.len();
        self.op_contributions = vec![Vec::new(); self.op_count];
        let mut state = State::default();
        for index in 0..ops.len() {
            if index % 1000 == 0 && callbacks.is_cancelled() {
                return Err("cancelled".to_string());
            }
            self.snapshot_len = self.gs.gluescript.len();
            self.handle(ops, index, &mut state)?;
            self.record_contribution(index);
        }
        Ok(())
    }

    fn finish(mut self) -> Result<EncodeOutput, String> {
        if self.op_count == 0 {
            return Ok(EncodeOutput::MachineCode {
                text: String::new(),
                op_to_machine_code: Vec::new(),
                machine_code_to_op: Vec::new(),
                payload: Vec::new(),
            });
        }
        if !self.job_started || !self.job_ended {
            return Err(
                "Ops sequence must start with JOB_START and end with JOB_END"
                    .to_string(),
            );
        }
        self.gs
            .stage_gluescript(None, true)
            .map_err(|error| error.to_string())?;
        let line_count = self.gs.gluescript.len();
        let (op_spans, line_ops) = self.build_op_map(line_count);
        let text = self.gs.gluescript.join("\n");
        let payload = self.gs.rpascript.join("\n").into_bytes();
        Ok(EncodeOutput::MachineCode {
            text,
            op_to_machine_code: op_spans,
            machine_code_to_op: line_ops,
            payload,
        })
    }

    fn record_contribution(&mut self, op_index: usize) {
        let end = self.gs.gluescript.len();
        if end > self.snapshot_len {
            self.op_contributions[op_index].push((self.snapshot_len, end));
        }
    }

    fn build_op_map(
        &self,
        line_count: usize,
    ) -> (Vec<OpLineRange>, Vec<usize>) {
        let mut spans = Vec::with_capacity(self.op_count);
        let mut line_ops = vec![MACHINE_CODE_TO_OP_NONE; line_count];
        for (op_index, contributions) in
            self.op_contributions.iter().enumerate()
        {
            let mut start = 0;
            let mut end = 0;
            for (contrib_start, contrib_end) in contributions {
                if contrib_end > contrib_start {
                    start = *contrib_start;
                    end = (*contrib_end).min(line_count);
                    break;
                }
            }
            if end <= start {
                spans.push(OpLineRange { start: 0, len: 0 });
                continue;
            }
            spans.push(OpLineRange {
                start: start as u32,
                len: (end - start) as u32,
            });
            for line in &mut line_ops[start..end] {
                *line = op_index;
            }
        }
        (spans, line_ops)
    }

    fn require_layer(&self) -> Result<(), String> {
        if self.layer_key == 0 {
            return Err(
                "layer-scoped op encountered before LAYER_START".to_string()
            );
        }
        Ok(())
    }

    fn emit_power(&mut self, power_fraction: f64) -> Result<(), String> {
        self.require_layer()?;
        let section_type = self.section_type;
        let mut power_fraction = power_fraction;
        if let Some(cap) = self.ctx.power_cap {
            power_fraction = power_fraction.min(cap);
        }
        if power_fraction == 0.0 {
            return Ok(());
        }
        if section_type.is_some()
            && matches!(self.layer_mode.as_str(), "IMAGE" | "DEPTHMAP")
        {
            self.gs.power(Some(power_fraction * 100.0));
            return Ok(());
        }
        let mut min_fraction = power_fraction;
        if self.layer_mode == "VECTOR"
            && self.overscan == "NONE"
            && self.power_min_fraction > 0.0
            && self.power_min_fraction < power_fraction
        {
            min_fraction = self.power_min_fraction;
        }
        if power_fraction != self.power_fraction
            || min_fraction != self.emitted_min_fraction
        {
            self.power_fraction = power_fraction;
            self.emitted_min_fraction = min_fraction;
            self.gs
                .power_range(
                    Some(min_fraction * 100.0),
                    Some(power_fraction * 100.0),
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn movement_form(&self, dx: f64, dy: f64) -> &'static str {
        if self.overscan == "NONE" {
            return "XY";
        }
        if dx.abs() < OVERSCAN_ANGLE_EPSILON
            && dy.abs() < OVERSCAN_ANGLE_EPSILON
        {
            return "XY";
        }
        let angle = dy.abs().atan2(dx.abs()).to_degrees();
        if angle <= OVERSCAN_ANGLE_TOLERANCE_DEG {
            return "X";
        }
        if angle >= 90.0 - OVERSCAN_ANGLE_TOLERANCE_DEG {
            return "Y";
        }
        "XY"
    }

    fn emit_movement(
        &mut self,
        x: f64,
        y: f64,
        cut: bool,
    ) -> Result<(), String> {
        let dx = x - self.current_pos.x;
        let dy = y - self.current_pos.y;
        let form = self.movement_form(dx, dy);
        let result = match (cut, form) {
            (true, "X") => self.gs.cut_x_to(x),
            (true, "Y") => self.gs.cut_y_to(y),
            (true, _) => self.gs.cut_xy_to(x, y),
            (false, "X") => {
                self.gs.move_x_to(x);
                Ok(())
            }
            (false, "Y") => {
                self.gs.move_y_to(y);
                Ok(())
            }
            (false, _) => {
                self.gs.move_xy_to(x, y);
                Ok(())
            }
        };
        result.map_err(|error| error.to_string())
    }

    fn linearize_curve(&mut self, ops: &Ops, idx: usize) -> Result<(), String> {
        self.require_layer()?;
        let start = self.current_pos;
        let end = ops.endpoint(idx);
        let sub_ops = ops.linearize(idx, start);
        let mut power = self.power_fraction;
        for j in 0..sub_ops.len() {
            match sub_ops.command_type(j) {
                CommandType::LineTo => {
                    let point = sub_ops.endpoint(j);
                    let cut = power > 0.0;
                    self.emit_movement(point.x, point.y, cut)?;
                    self.current_pos = point;
                }
                CommandType::SetPower => {
                    power = sub_ops.state_at(j).power;
                    if power > 0.0 {
                        let bias = if self.layer_mode == "VECTOR" {
                            if self.ctx.power_floor_enabled {
                                self.power_floor
                            } else {
                                0.0
                            }
                        } else {
                            self.image_power_bias
                        };
                        self.emit_power((power + bias).min(1.0))?;
                    } else {
                        self.emit_power(0.0)?;
                    }
                }
                _ => {}
            }
        }
        self.current_pos = end;
        Ok(())
    }

    fn handle(
        &mut self,
        ops: &Ops,
        idx: usize,
        state: &mut State,
    ) -> Result<(), String> {
        let command = ops.command_type(idx);
        apply_state(state, &ops.commands[idx].category);
        match command {
            CommandType::SetPower => {
                self.last_requested_power = Some(state.power);
                let power = state.power;
                self.emit_power(power)?;
            }
            CommandType::SetPowerMode => {
                if !self.ctx.power_scaling_enabled {
                    self.gs.set_power_scaling_enabled(false);
                } else {
                    let dynamic = state.power_mode == Some(PowerMode::Dynamic);
                    self.gs.set_power_scaling_enabled(dynamic);
                }
            }
            CommandType::SetFeedRate => {
                self.require_layer()?;
                let speed = f64::from(state.feed_rate.unwrap_or(0)) / 60.0;
                self.gs.cut_speed(speed);
            }
            CommandType::SetRapidRate => {
                self.require_layer()?;
                let speed = f64::from(state.rapid_rate.unwrap_or(0)) / 60.0;
                self.gs.move_speed(speed);
            }
            CommandType::SetFrequency => {
                self.require_layer()?;
                let frequency =
                    f64::from(state.frequency.unwrap_or(0)) / 1000.0;
                self.gs.frequency(frequency);
            }
            CommandType::SetPulseWidth => {
                self.require_layer()?;
                self.gs.pwm(state.pulse_width.unwrap_or(0.0));
            }
            CommandType::SetCoolant => {
                let mode = state.coolant.unwrap_or_default();
                if mode != CoolantMode::Off {
                    log_crate::warn!(
                        "SET_COOLANT {mode:?} is not acted upon — \
                         air assist requires SET_AIR_ASSIST"
                    );
                }
                self.set_air_assist(false)?;
            }
            CommandType::SetAirAssist => {
                let on = state.air_assist == Some(AirAssistMode::On);
                self.set_air_assist(on)?;
            }
            CommandType::SetHead => {
                self.select_laser(state.active_head_uid.as_deref())?;
            }
            CommandType::MoveTo => {
                self.require_layer()?;
                let end = ops.endpoint(idx);
                if end.z != 0.0 {
                    log_crate::warn!(
                        "Ignoring Z={:.3}mm on MOVE_TO — laser jobs are 2D",
                        end.z
                    );
                }
                self.emit_movement(end.x, end.y, false)?;
                self.current_pos = end;
            }
            CommandType::LineTo => {
                self.require_layer()?;
                let end = ops.endpoint(idx);
                if end.z != 0.0 {
                    log_crate::warn!(
                        "Ignoring Z={:.3}mm on LINE_TO — laser jobs are 2D",
                        end.z
                    );
                }
                self.emit_movement(end.x, end.y, true)?;
                self.current_pos = end;
            }
            CommandType::ArcTo
            | CommandType::ScanLine
            | CommandType::BezierTo
            | CommandType::QuadraticBezierTo => {
                self.linearize_curve(ops, idx)?
            }
            CommandType::Dwell => {
                log_crate::warn!(
                    "DWELL is not supported — the Ruida controller has \
                     no direct equivalent; no delay was emitted"
                );
            }
            CommandType::JobStart => self.job_start(),
            CommandType::JobEnd => {
                self.job_ended = true;
                self.gs.end_job().map_err(|error| error.to_string())?;
            }
            CommandType::LayerStart => {
                if let OpCategory::Marker(MarkerCmd::LayerStart(uid)) =
                    &ops.commands[idx].category
                {
                    self.layer_uid = Some(uid.to_string());
                }
                self.layer_key += 1;
                self.layer_declared = false;
                self.declared_layer = None;
                self.last_requested_power = None;
            }
            CommandType::LayerEnd => {
                if !self.layer_declared {
                    return Err(
                        "LAYER_END encountered before any WORKPIECE_START"
                            .to_string(),
                    );
                }
                self.section_type = None;
                self.section_raster_mode = None;
                self.layer_mode = "VECTOR".to_string();
                self.power_fraction = 0.0;
                self.power_min_fraction = 0.0;
                self.emitted_min_fraction = 0.0;
                self.layer_uid = None;
            }
            CommandType::WorkpieceStart => self.workpiece_start(ops, idx)?,
            CommandType::WorkpieceEnd => {
                self.overscan = "NONE".to_string();
                self.power_fraction = 0.0;
                self.power_min_fraction = 0.0;
                self.emitted_min_fraction = 0.0;
                self.gs.comment(&["# Workpiece End".to_string()]);
            }
            CommandType::OpsSectionStart => self.section_start(ops, idx)?,
            CommandType::OpsSectionEnd => {
                self.section_type = None;
                self.section_raster_mode = None;
                self.gs.comment(&["# Ops Section End".to_string()]);
            }
            CommandType::SetSpindleRpm
            | CommandType::SetHeadCoolant
            | CommandType::StateBlockStart
            | CommandType::StateBlockEnd => {}
            other => log_crate::warn!("Unknown command type: {other:?}"),
        }
        self.gs
            .comment(&[format!("# Op {idx}: {}", command_name(command))]);
        Ok(())
    }

    fn job_start(&mut self) {
        self.job_started = true;
        let label = if self.ctx.job_label.is_empty() {
            DEFAULT_JOB_LABEL
        } else {
            self.ctx.job_label.as_str()
        };
        if let Err(error) = self.gs.declare_job(
            label,
            &self.ctx.ref_point,
            None,
            1,
            1,
            0.0,
            0.0,
        ) {
            log_crate::error!("Ruida declare_job failed: {error}");
        }
    }

    fn select_laser(&mut self, uid: Option<&str>) -> Result<(), String> {
        let Some(uid) = uid else {
            return Ok(());
        };
        let mut device = match uid
            .rsplit('_')
            .next()
            .and_then(|value| value.parse::<i64>().ok())
        {
            Some(number) => (number - 1).rem_euclid(2) + 1,
            None => {
                (uid.chars().map(|ch| ch as u32 as i64).sum::<i64>() % 2) + 1
            }
        };
        if let Some(tool) = self.ctx.head_tool_numbers.get(uid) {
            device = *tool;
        }
        if device == self.active_laser {
            return Ok(());
        }
        self.active_laser = device;
        self.gs.select_laser(device);
        Ok(())
    }

    fn set_air_assist(&mut self, enabled: bool) -> Result<(), String> {
        self.require_layer()?;
        if enabled {
            self.gs.air_assist_on();
        } else {
            self.gs.air_assist_off();
        }
        Ok(())
    }

    fn section_layer_mode(
        section_type: SectionType,
        raster_mode: Option<RasterMode>,
    ) -> &'static str {
        if section_type != SectionType::RasterFill {
            return "VECTOR";
        }
        match raster_mode {
            Some(RasterMode::Dither) => "DITHER",
            Some(RasterMode::DepthMap) => "DEPTHMAP",
            Some(RasterMode::ConstantPower) => "RASTER",
            Some(RasterMode::VariablePower) => "IMAGE",
            _ => "VECTOR",
        }
    }

    fn compute_layer_mode(&self, ops: &Ops, idx: usize) -> &'static str {
        for i in idx + 1..ops.len() {
            match &ops.commands[i].category {
                OpCategory::Marker(
                    MarkerCmd::LayerEnd(_)
                    | MarkerCmd::LayerStart(_)
                    | MarkerCmd::WorkpieceEnd(_)
                    | MarkerCmd::WorkpieceStart(_)
                    | MarkerCmd::JobEnd,
                ) => break,
                OpCategory::Marker(MarkerCmd::OpsSectionStart {
                    section_type,
                    raster_mode,
                    ..
                }) if *section_type == SectionType::RasterFill => {
                    return Self::section_layer_mode(
                        *section_type,
                        *raster_mode,
                    );
                }
                _ => {}
            }
        }
        "VECTOR"
    }

    fn classify_overscan(
        start: &Point3D,
        end: &Point3D,
    ) -> Option<&'static str> {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        if dx.abs() < OVERSCAN_ANGLE_EPSILON
            && dy.abs() < OVERSCAN_ANGLE_EPSILON
        {
            return None;
        }
        let angle = dy.abs().atan2(dx.abs()).to_degrees();
        if angle <= OVERSCAN_ANGLE_TOLERANCE_DEG {
            return Some("X_BI");
        }
        if angle >= 90.0 - OVERSCAN_ANGLE_TOLERANCE_DEG {
            return Some("Y_BI");
        }
        Some("NONE")
    }

    fn compute_overscan(
        &self,
        ops: &Ops,
        idx: usize,
        layer_mode: &str,
        section_bounded: bool,
    ) -> &'static str {
        if layer_mode == "VECTOR" {
            return "NONE";
        }
        let mut pos = Point3D::new(0.0, 0.0, 0.0);
        let mut in_raster_fill = section_bounded;
        for i in idx + 1..ops.len() {
            match &ops.commands[i].category {
                OpCategory::Marker(
                    MarkerCmd::LayerEnd(_)
                    | MarkerCmd::LayerStart(_)
                    | MarkerCmd::WorkpieceEnd(_)
                    | MarkerCmd::WorkpieceStart(_)
                    | MarkerCmd::JobEnd,
                ) => break,
                OpCategory::Marker(MarkerCmd::OpsSectionEnd { .. }) => {
                    if section_bounded {
                        break;
                    }
                    in_raster_fill = false;
                }
                OpCategory::Marker(MarkerCmd::OpsSectionStart {
                    section_type,
                    ..
                }) => {
                    if section_bounded {
                        break;
                    }
                    in_raster_fill = *section_type == SectionType::RasterFill;
                }
                OpCategory::Moving { .. } => {
                    let kind = ops.command_type(i);
                    let end = ops.endpoint(i);
                    if matches!(
                        kind,
                        CommandType::LineTo | CommandType::ScanLine
                    ) && in_raster_fill
                    {
                        if let Some(overscan) =
                            Self::classify_overscan(&pos, &end)
                        {
                            return overscan;
                        }
                    }
                    pos = end;
                }
                _ => {}
            }
        }
        "NONE"
    }

    fn workpiece_start(&mut self, ops: &Ops, idx: usize) -> Result<(), String> {
        let Some(layer_uid) = self.layer_uid.clone() else {
            return Err(
                "WORKPIECE_START encountered before LAYER_START".to_string()
            );
        };
        let workpiece_uid = ops.workpiece_uid(idx).to_string();
        self.gs
            .comment(&[format!("# Workpiece Start uid={workpiece_uid}")]);
        let layer = self
            .ctx
            .layers
            .iter()
            .find(|layer| layer.uid == layer_uid)
            .ok_or_else(|| {
                format!(
                    "Layer uid '{layer_uid}' was not found in the document \
                     — the Ruida encoder requires a declared layer for \
                     every workpiece"
                )
            })?;
        let speed = layer.speed_mm_s;
        let frequency = layer.frequency_khz;
        let power_pct = layer.power_pct;
        let layer_mode = self.compute_layer_mode(ops, idx);
        self.layer_mode = layer_mode.to_string();
        let overscan = self.compute_overscan(ops, idx, layer_mode, false);
        self.overscan = overscan.to_string();
        self.power_min_fraction = if self.ctx.power_floor_enabled {
            let raw = layer.min_power_pct.unwrap_or(self.ctx.power_floor_pct);
            (raw / 100.0).clamp(self.power_floor, 1.0)
        } else {
            1.0
        };
        let (min_power, max_power) = if layer_mode == "VECTOR"
            && overscan == "NONE"
            && self.power_min_fraction * 100.0 < power_pct
        {
            (self.power_min_fraction * 100.0, power_pct)
        } else {
            (power_pct, power_pct)
        };
        let declared = DeclaredLayer {
            label: layer.name.clone(),
            color: layer.color.clone(),
            mode: layer_mode.to_string(),
            overscan: overscan.to_string(),
            speed,
            frequency,
            min_power,
            max_power,
        };
        if self.declared_layer.as_ref() == Some(&declared) {
            return Ok(());
        }
        self.gs
            .declare_layer(
                &declared.label,
                &declared.color,
                &declared.mode,
                &declared.overscan,
                declared.speed,
                declared.frequency,
                declared.min_power,
                declared.max_power,
            )
            .map_err(|error| error.to_string())?;
        self.declared_layer = Some(declared);
        self.layer_declared = true;
        self.reapply_last_power()?;
        Ok(())
    }

    fn reapply_last_power(&mut self) -> Result<(), String> {
        let Some(power) = self.last_requested_power else {
            return Ok(());
        };
        self.power_fraction = 0.0;
        self.emitted_min_fraction = 0.0;
        self.emit_power(power)
    }

    fn section_start(&mut self, ops: &Ops, idx: usize) -> Result<(), String> {
        let OpCategory::Marker(MarkerCmd::OpsSectionStart {
            section_type,
            raster_mode,
            ..
        }) = &ops.commands[idx].category
        else {
            return Ok(());
        };
        self.section_type = Some(*section_type);
        self.section_raster_mode = *raster_mode;
        self.gs.comment(&[
            "# Ops Actions".to_string(),
            "# Ops Section Start".to_string(),
        ]);
        let section_mode =
            Self::section_layer_mode(*section_type, *raster_mode);
        let section_overscan =
            self.compute_overscan(ops, idx, section_mode, true);
        if section_overscan != self.overscan {
            self.gs
                .set_overscan(section_overscan)
                .map_err(|error| error.to_string())?;
            self.overscan = section_overscan.to_string();
        }
        Ok(())
    }
}

fn apply_state(state: &mut State, category: &OpCategory) {
    let OpCategory::State(command) = category else {
        return;
    };
    match command {
        StateCmd::SetPower(value) => state.power = *value,
        StateCmd::SetFeedRate(value) => state.feed_rate = Some(*value),
        StateCmd::SetRapidRate(value) => state.rapid_rate = Some(*value),
        StateCmd::Dwell(value) => state.dwell_ms = Some(*value),
        StateCmd::SetHead(uid) => state.active_head_uid = Some(uid.to_string()),
        StateCmd::SetFrequency(value) => state.frequency = Some(*value),
        StateCmd::SetPulseWidth(value) => state.pulse_width = Some(*value),
        StateCmd::SetCoolant(mode) => state.coolant = Some(*mode),
        StateCmd::SetAirAssist(mode) => state.air_assist = Some(*mode),
        StateCmd::SetHeadCoolant(mode) => state.head_coolant = Some(*mode),
        StateCmd::SetPowerMode(mode) => state.power_mode = Some(*mode),
        _ => {}
    }
}

fn command_name(command: CommandType) -> &'static str {
    match command {
        CommandType::MoveTo => "MOVE_TO",
        CommandType::LineTo => "LINE_TO",
        CommandType::ArcTo => "ARC_TO",
        CommandType::ScanLine => "SCAN_LINE",
        CommandType::Dwell => "DWELL",
        CommandType::BezierTo => "BEZIER_TO",
        CommandType::QuadraticBezierTo => "QUADRATIC_BEZIER_TO",
        CommandType::SetPower => "SET_POWER",
        CommandType::SetFeedRate => "SET_FEED_RATE",
        CommandType::SetRapidRate => "SET_RAPID_RATE",
        CommandType::SetHead => "SET_HEAD",
        CommandType::SetFrequency => "SET_FREQUENCY",
        CommandType::SetPulseWidth => "SET_PULSE_WIDTH",
        CommandType::SetSpindleRpm => "SET_SPINDLE_RPM",
        CommandType::SetSpindleDirection => "SET_SPINDLE_DIRECTION",
        CommandType::SetSyncPitch => "SET_SYNC_PITCH",
        CommandType::SetCoolant => "SET_COOLANT",
        CommandType::SetAirAssist => "SET_AIR_ASSIST",
        CommandType::SetHeadCoolant => "SET_HEAD_COOLANT",
        CommandType::SetPowerMode => "SET_POWER_MODE",
        CommandType::Custom => "CUSTOM",
        CommandType::JobStart => "JOB_START",
        CommandType::JobEnd => "JOB_END",
        CommandType::LayerStart => "LAYER_START",
        CommandType::LayerEnd => "LAYER_END",
        CommandType::WorkpieceStart => "WORKPIECE_START",
        CommandType::WorkpieceEnd => "WORKPIECE_END",
        CommandType::OpsSectionStart => "OPS_SECTION_START",
        CommandType::OpsSectionEnd => "OPS_SECTION_END",
        CommandType::StateBlockStart => "STATE_BLOCK_START",
        CommandType::StateBlockEnd => "STATE_BLOCK_END",
    }
}
