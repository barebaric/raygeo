//! The GlueScript job authoring and compilation model.
//!
//! GlueScript is the high-level scripting API the Ruida encoder drives:
//! jobs and layers are declared, settings and movements are recorded as a
//! canonical transcript, and `stage_gluescript` compiles that transcript
//! into the controller-facing rpascript. The transcript is the source of
//! truth: it can be persisted and replayed byte-for-byte, and the
//! compiler's output is deterministic.
//!
//! The model is a port of the Ruida Protocol Analyzer's GlueScript.

use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};

/// A transcript parse or replay error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlueError(pub String);

impl fmt::Display for GlueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GlueError {}

fn error(message: impl Into<String>) -> GlueError {
    GlueError(message.into())
}

const WRAP_WIDTH: usize = 100;
const INDENT: usize = 4;

/// A literal value in a transcript call.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
    List(Vec<Literal>),
}

impl Literal {
    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(value) => Some(*value as f64),
            Self::Float(value) => Some(*value),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            Self::Str(value) => Some(value),
            _ => None,
        }
    }

    fn repr(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Float(value) => repr_float(*value),
            Self::Str(value) => repr_string(value),
            Self::Bool(value) => {
                if *value { "True" } else { "False" }.to_string()
            }
            Self::None => "None".to_string(),
            Self::List(values) => {
                let items: Vec<String> =
                    values.iter().map(Self::repr).collect();
                format!("[{}]", items.join(", "))
            }
        }
    }
}

fn repr_float(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e16 {
        return format!("{value:.1}");
    }
    let text = format!("{value}");
    text
}

fn repr_string(value: &str) -> String {
    if value.contains('\'') && !value.contains('"') {
        let escaped: String = value
            .chars()
            .flat_map(|ch| match ch {
                '\\' => "\\\\".chars().collect::<Vec<_>>(),
                '"' => "\\\"".chars().collect::<Vec<_>>(),
                '\n' => "\\n".chars().collect::<Vec<_>>(),
                '\r' => "\\r".chars().collect::<Vec<_>>(),
                '\t' => "\\t".chars().collect::<Vec<_>>(),
                _ => vec![ch],
            })
            .collect();
        return format!("\"{escaped}\"");
    }
    let escaped: String = value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\'' => "\\'".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect();
    format!("'{escaped}'")
}

fn format_float(value: f64) -> String {
    repr_float(value)
}

/// The GlueScript authoring and compilation state.
#[derive(Debug, Clone)]
pub struct GlueScript {
    /// The canonical transcript lines.
    pub gluescript: Vec<String>,
    /// The compiled controller script.
    pub rpascript: Vec<String>,
    pub max_cut_speed: f64,
    pub power_floor: f64,
    pub power_scaling_enabled: bool,

    job_complete: bool,
    job_declared: bool,
    assembling: bool,
    job_header: Vec<String>,
    layer_attributes: BTreeMap<usize, Vec<String>>,
    layer_actions: BTreeMap<usize, Vec<String>>,
    layer_overscan: BTreeMap<usize, Vec<String>>,
    inline_prelude: Vec<String>,
    inline_epilogue: Vec<String>,
    inline_used: bool,
    comment_only_used: bool,
    stage_complete: bool,
    doc_tr_x: f64,
    doc_tr_y: f64,
    doc_bl_x: f64,
    doc_bl_y: f64,
    current_x: f64,
    current_y: f64,
    abs_xy: [f64; 2],
    layer: usize,
    current_layer_mode: String,
    current_layer_overscan: String,
    current_imd_power: Option<f64>,
    layer_trx: f64,
    layer_try: f64,
    layer_blx: f64,
    layer_bly: f64,
    last_layer_has_content: bool,
    current_layer_min_power: f64,
    current_layer_max_power: f64,
    current_layer_speed: f64,
    current_layer_frequency: f64,
    frequency_changed: bool,
    pending_min_power: f64,
    pending_max_power: f64,
    pending_warnings: Vec<String>,
    speed_dirty: bool,
    power_dirty: bool,
}

impl Default for GlueScript {
    fn default() -> Self {
        Self::new()
    }
}

impl GlueScript {
    /// Creates an empty script.
    pub fn new() -> Self {
        let mut script = Self {
            gluescript: Vec::new(),
            rpascript: Vec::new(),
            max_cut_speed: 400.0,
            power_floor: 8.0,
            power_scaling_enabled: true,
            job_complete: false,
            job_declared: false,
            assembling: false,
            job_header: Vec::new(),
            layer_attributes: BTreeMap::new(),
            layer_actions: BTreeMap::new(),
            layer_overscan: BTreeMap::new(),
            inline_prelude: Vec::new(),
            inline_epilogue: Vec::new(),
            inline_used: false,
            comment_only_used: false,
            stage_complete: false,
            doc_tr_x: 0.0,
            doc_tr_y: 0.0,
            doc_bl_x: 0.0,
            doc_bl_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            abs_xy: [0.0, 0.0],
            layer: 0,
            current_layer_mode: "VECTOR".to_string(),
            current_layer_overscan: "NONE".to_string(),
            current_imd_power: None,
            layer_trx: 0.0,
            layer_try: 0.0,
            layer_blx: 0.0,
            layer_bly: 0.0,
            last_layer_has_content: false,
            current_layer_min_power: 8.0,
            current_layer_max_power: 70.0,
            current_layer_speed: 100.0,
            current_layer_frequency: 20.0,
            frequency_changed: false,
            pending_min_power: 8.0,
            pending_max_power: 70.0,
            pending_warnings: Vec::new(),
            speed_dirty: false,
            power_dirty: false,
        };
        script.reset_boxes();
        script
    }

    fn reset_boxes(&mut self) {
        self.doc_tr_x = f64::INFINITY;
        self.doc_tr_y = f64::INFINITY;
        self.doc_bl_x = f64::NEG_INFINITY;
        self.doc_bl_y = f64::NEG_INFINITY;
        self.layer_trx = f64::INFINITY;
        self.layer_try = f64::INFINITY;
        self.layer_blx = f64::NEG_INFINITY;
        self.layer_bly = f64::NEG_INFINITY;
        self.last_layer_has_content = false;
    }

    /// Whether `end_job` finalized the job.
    #[cfg_attr(not(feature = "python"), allow(dead_code))]
    pub fn job_complete(&self) -> bool {
        self.job_complete
    }

    /// Resets all per-job state.
    pub fn new_gluescript(&mut self) {
        self.gluescript.clear();
        self.job_header.clear();
        self.layer_attributes.clear();
        self.layer_actions.clear();
        self.layer_overscan.clear();
        self.inline_prelude.clear();
        self.inline_epilogue.clear();
        self.rpascript.clear();
        self.job_complete = false;
        self.job_declared = false;
        self.assembling = false;
        self.inline_used = false;
        self.comment_only_used = false;
        self.stage_complete = false;
        self.layer = 0;
        self.current_layer_mode = "VECTOR".to_string();
        self.current_layer_overscan = "NONE".to_string();
        self.current_imd_power = None;
        self.current_layer_min_power = 8.0;
        self.current_layer_max_power = 70.0;
        self.current_layer_speed = 100.0;
        self.current_layer_frequency = 20.0;
        self.frequency_changed = false;
        self.pending_min_power = 8.0;
        self.pending_max_power = 70.0;
        self.pending_warnings.clear();
        self.speed_dirty = false;
        self.power_dirty = false;
        self.reset_boxes();
    }

    // -- transcript -------------------------------------------------------

    fn record(&mut self, name: &str, args: &[Literal]) {
        let rendered = format_call(name, args);
        self.gluescript.extend(rendered);
    }

    /// Appends comment lines to the job header.
    pub fn comment(&mut self, comments: &[String]) {
        for line in comments {
            self.record(
                "comment",
                &[Literal::List(vec![Literal::Str(line.clone())])],
            );
            let line = if line.starts_with('#') {
                line.clone()
            } else {
                format!("# {line}")
            };
            self.job_header.push(line);
        }
    }

    /// Declares a job and resets the script.
    #[allow(clippy::too_many_arguments)]
    pub fn declare_job(
        &mut self,
        label: &str,
        ref_point: &str,
        abs_xy: Option<[f64; 2]>,
        columns: i64,
        rows: i64,
        xstep: f64,
        ystep: f64,
    ) -> Result<(), GlueError> {
        let abs_xy = abs_xy.unwrap_or([0.0, 0.0]);
        self.new_gluescript();
        self.abs_xy = abs_xy;
        let ref_lines: Vec<String> = match ref_point {
            "MACHINE" => vec![
                "REF_POINT_MACHINE".to_string(),
                "SET_ABSOLUTE".to_string(),
            ],
            "ABSOLUTE" => vec![
                format!(
                    "JOG_XY Rel=MACHINE X={} Y={}",
                    repr_float(self.abs_xy[0]),
                    repr_float(self.abs_xy[1])
                ),
                "REF_POINT_CURRENT".to_string(),
            ],
            "CURRENT" => vec!["REF_POINT_CURRENT".to_string()],
            "SET_POINT" => vec!["REF_POINT_ORIGIN".to_string()],
            _ => {
                return Err(error(format!(
                    "invalid reference point: {ref_point:?}"
                )))
            }
        };
        self.job_declared = true;
        self.assembling = true;
        self.record(
            "declare_job",
            &[
                Literal::Str(label.to_string()),
                Literal::Str(ref_point.to_string()),
                Literal::List(vec![
                    Literal::Float(abs_xy[0]),
                    Literal::Float(abs_xy[1]),
                ]),
                Literal::Int(columns),
                Literal::Int(rows),
                Literal::Float(xstep),
                Literal::Float(ystep),
            ],
        );
        self.job_header.push(format!("# Job: {label}"));
        self.job_header
            .push("# Generated by: raydriver GlueScript".to_string());
        self.job_header.extend(ref_lines);
        self.job_header.push("REF_POINT_SET".to_string());
        self.job_header.push("START_JOB".to_string());
        self.job_header.push("FEED_REPEAT 0 0".to_string());
        self.job_header
            .push("SET_FEED_AUTO_PAUSE State:OFF".to_string());
        self.job_header.push("# Job settings".to_string());
        self.job_header.push(
            "JOB_TOP_RIGHT X={self.doc_tr_x}mm Y={self.doc_tr_y}mm".to_string(),
        );
        self.job_header.push(
            "JOB_BOTTOM_LEFT X={self.doc_bl_x}mm Y={self.doc_bl_y}mm"
                .to_string(),
        );
        self.job_header.push(
            "DOCUMENT_TOP_RIGHT X={self.doc_tr_x}mm Y={self.doc_tr_y}mm"
                .to_string(),
        );
        self.job_header.push(
            "DOCUMENT_BOTTOM_LEFT X={self.doc_bl_x}mm Y={self.doc_bl_y}mm"
                .to_string(),
        );
        self.job_header.push(format!(
            "JOB_COPIES Columns={columns} Rows={rows} XStep={}mm YStep={}mm",
            format_float(xstep),
            format_float(ystep)
        ));
        Ok(())
    }

    /// Finalizes the job.
    pub fn end_job(&mut self) -> Result<(), GlueError> {
        if self.job_complete {
            return Err(error("end_job called twice"));
        }
        if !self.job_declared {
            return Err(error("declare_job must be called first"));
        }
        self.close_layer_bounds();
        self.job_complete = true;
        self.assembling = false;
        self.record("end_job", &[]);
        Ok(())
    }

    fn close_layer_bounds(&mut self) {
        if self.layer > 0 && self.last_layer_has_content {
            let layer = self.layer - 1;
            self.layer_attributes
                .entry(self.layer)
                .or_default()
                .push(format!(
                    "LAYER_TOP_RIGHT Layer:{layer} X={:.3}mm Y={:.3}mm",
                    self.layer_trx, self.layer_try
                ));
            self.layer_attributes
                .entry(self.layer)
                .or_default()
                .push(format!(
                    "LAYER_BOTTOM_LEFT Layer:{layer} X={:.3}mm Y={:.3}mm",
                    self.layer_blx, self.layer_bly
                ));
        }
    }

    /// Declares a layer and stores its attribute block.
    #[allow(clippy::too_many_arguments)]
    pub fn declare_layer(
        &mut self,
        label: &str,
        color: &str,
        mode: &str,
        overscan: &str,
        speed: f64,
        frequency: f64,
        min_power: f64,
        max_power: f64,
    ) -> Result<(), GlueError> {
        let mode_override = match mode {
            "VECTOR" => Some("NONE"),
            "RASTER" | "DITHER" | "IMAGE" | "DEPTHMAP" => Some(""),
            _ => return Err(error(format!("invalid layer mode: {mode:?}"))),
        };
        if !matches!(overscan, "NONE" | "X" | "X_BI" | "Y" | "Y_BI" | "XY") {
            return Err(error(format!("invalid overscan mode: {overscan:?}")));
        }
        let mut warnings = Vec::new();
        if min_power < self.power_floor {
            warnings.push(format!(
                "# warning: min_power_1 {min_power}% is below the \
                 recommended minimum of {}%",
                self.power_floor
            ));
        }
        if max_power > 70.0 {
            warnings.push(format!(
                "# warning: max_power_1 {max_power}% exceeds the \
                 recommended maximum of 70%"
            ));
        }
        let resolved =
            mode_override.filter(|v| !v.is_empty()).unwrap_or(overscan);
        self.close_layer_bounds();
        self.layer += 1;
        self.current_layer_mode = mode.to_string();
        self.current_layer_overscan = resolved.to_string();
        self.current_imd_power = None;
        self.current_layer_speed = speed;
        self.layer_trx = f64::INFINITY;
        self.layer_try = f64::INFINITY;
        self.layer_blx = f64::NEG_INFINITY;
        self.layer_bly = f64::NEG_INFINITY;
        self.last_layer_has_content = false;
        self.current_layer_min_power = min_power;
        self.current_layer_max_power = max_power;
        self.current_layer_frequency = frequency;
        self.frequency_changed = false;
        self.pending_min_power = 8.0;
        self.pending_max_power = 70.0;
        self.pending_warnings.clear();
        self.speed_dirty = false;
        self.power_dirty = false;
        self.record(
            "declare_layer",
            &[
                Literal::Str(label.to_string()),
                Literal::Str(color.to_string()),
                Literal::Str(mode.to_string()),
                Literal::Str(overscan.to_string()),
                Literal::Float(speed),
                Literal::Float(frequency),
                Literal::Float(min_power),
                Literal::Float(max_power),
            ],
        );
        let layer = self.layer - 1;
        let mut attrs = vec![format!("# Layer {layer}: {label}")];
        attrs.push(format!(
            "LAYER_COLOR Layer:{layer} Color:{}",
            color.replace('#', "\\#")
        ));
        attrs.push(format!(
            "CUT_SPEED_LASER_1 Layer:{layer} Speed:{}mm/S",
            format_float(speed)
        ));
        attrs.push(format!(
            "LAYER_MIN_POWER_1 Layer:{layer} Power:{}%",
            format_float(min_power)
        ));
        attrs.push(format!(
            "LAYER_MAX_POWER_1 Layer:{layer} Power:{}%",
            format_float(max_power)
        ));
        attrs.extend(warnings);
        attrs.push(format!(
            "LAYER_FREQUENCY Laser:0 Layer:{layer} Freq:{frequency:.3}KHz"
        ));
        attrs.push(format!("LAYER_ATTRIBUTES Layer:{layer} 0"));
        self.layer_attributes.insert(self.layer, attrs);
        self.layer_overscan
            .insert(self.layer, overscan_lines(resolved));
        Ok(())
    }

    // -- settings ---------------------------------------------------------

    /// Sets immediate power for an IMAGE/DEPTHMAP layer.
    pub fn power(&mut self, percent: Option<f64>) {
        if !matches!(self.current_layer_mode.as_str(), "IMAGE" | "DEPTHMAP") {
            return;
        }
        let Some(percent) = percent else {
            return;
        };
        self.record("power", &[Literal::Float(percent)]);
        self.emit_imd_power(percent);
    }

    fn emit_imd_power(&mut self, percent: f64) {
        self.layer_actions
            .entry(self.layer)
            .or_default()
            .push(format!("IMD_POWER_1 Power:{percent:.1}%"));
        self.layer_actions
            .entry(self.layer)
            .or_default()
            .push(format!("IMD_POWER_3 Power:{percent:.1}%"));
        self.current_imd_power = Some(percent);
    }

    /// Saves a power ramp range for the next cut.
    pub fn power_range(
        &mut self,
        min_power: Option<f64>,
        max_power: Option<f64>,
    ) -> Result<(), GlueError> {
        if self.layer < 1 {
            return Err(error(
                "power_range requires a declared layer".to_string(),
            ));
        }
        let resolved_min = min_power.unwrap_or(self.current_layer_min_power);
        let resolved_max = max_power.unwrap_or(self.current_layer_max_power);
        let mut warnings = Vec::new();
        if resolved_min > resolved_max {
            warnings.push(format!(
                "# warning: min_power_1 {resolved_min}% exceeds \
                 max_power_1 {resolved_max}%"
            ));
        }
        if resolved_min < self.power_floor {
            warnings.push(format!(
                "# warning: min_power_1 {resolved_min}% is below the \
                 recommended minimum of {}%",
                self.power_floor
            ));
        }
        if resolved_max > 70.0 {
            warnings.push(format!(
                "# warning: max_power_1 {resolved_max}% exceeds the \
                 recommended maximum of 70%"
            ));
        }
        self.record(
            "power_range",
            &[optional_literal(min_power), optional_literal(max_power)],
        );
        self.pending_min_power = resolved_min;
        self.pending_max_power = resolved_max;
        self.pending_warnings = warnings;
        self.power_dirty = true;
        Ok(())
    }

    /// Saves the cut speed for the next cut.
    pub fn cut_speed(&mut self, speed: f64) {
        self.record("cut_speed", &[Literal::Float(speed)]);
        self.current_layer_speed = speed;
        self.speed_dirty = true;
    }

    /// Records a move speed as a comment.
    pub fn move_speed(&mut self, speed: f64) {
        self.record("move_speed", &[Literal::Float(speed)]);
        self.layer_actions
            .entry(self.layer)
            .or_default()
            .push(format!("# move_speed({})", repr_float(speed)));
        self.comment_only_used = true;
    }

    /// Saves the layer frequency; emitted by the next flush.
    pub fn frequency(&mut self, frequency: f64) {
        self.record("frequency", &[Literal::Float(frequency)]);
        if frequency != self.current_layer_frequency {
            self.frequency_changed = true;
        }
        self.current_layer_frequency = frequency;
    }

    /// Records the pulse width as a comment.
    pub fn pwm(&mut self, duration: f64) {
        self.record("pwm", &[Literal::Float(duration)]);
        self.layer_actions
            .entry(self.layer)
            .or_default()
            .push(format!("# pwm({})", repr_float(duration)));
        self.comment_only_used = true;
    }

    /// Selects laser head 1.
    pub fn select_laser(&mut self, laser: i64) {
        self.record("select_laser", &[Literal::Int(laser)]);
        if laser == 1 {
            self.layer_actions
                .entry(self.layer)
                .or_default()
                .push("LASER_DEVICE_1".to_string());
        }
    }

    /// Enables air assist.
    pub fn air_assist_on(&mut self) {
        self.record("air_assist_on", &[]);
        self.layer_actions
            .entry(self.layer)
            .or_default()
            .push("AIR_ASSIST_ON".to_string());
    }

    /// Disables air assist.
    pub fn air_assist_off(&mut self) {
        self.record("air_assist_off", &[]);
        self.layer_actions
            .entry(self.layer)
            .or_default()
            .push("AIR_ASSIST_OFF".to_string());
    }

    /// Switches the layer mode mid-stream.
    pub fn set_mode(&mut self, mode: &str) -> Result<(), GlueError> {
        if self.layer < 1 {
            return Err(error("set_mode requires a declared layer"));
        }
        if !matches!(
            mode,
            "VECTOR" | "RASTER" | "DITHER" | "IMAGE" | "DEPTHMAP"
        ) {
            return Err(error(format!("invalid layer mode: {mode:?}")));
        }
        self.record("set_mode", &[Literal::Str(mode.to_string())]);
        if mode == self.current_layer_mode {
            return Ok(());
        }
        self.current_layer_mode = mode.to_string();
        if let Some(override_mode) = match mode {
            "VECTOR" => Some("NONE"),
            _ => None,
        } {
            if self.current_layer_overscan != override_mode {
                self.layer_actions
                    .entry(self.layer)
                    .or_default()
                    .extend(overscan_lines(override_mode));
                self.current_layer_overscan = override_mode.to_string();
            }
        }
        Ok(())
    }

    /// Sets the overscan for the current layer.
    pub fn set_overscan(&mut self, overscan: &str) -> Result<(), GlueError> {
        if self.layer < 1 {
            return Err(error("set_overscan requires a declared layer"));
        }
        if !matches!(overscan, "NONE" | "X" | "X_BI" | "Y" | "Y_BI" | "XY") {
            return Err(error(format!("invalid overscan mode: {overscan:?}")));
        }
        self.record("set_overscan", &[Literal::Str(overscan.to_string())]);
        if overscan == self.current_layer_overscan {
            return Ok(());
        }
        self.layer_actions
            .entry(self.layer)
            .or_default()
            .extend(overscan_lines(overscan));
        self.current_layer_overscan = overscan.to_string();
        Ok(())
    }

    /// Sets the maximum cut speed used by power scaling.
    pub fn set_max_cut_speed(&mut self, speed: f64) -> Result<(), GlueError> {
        if !speed.is_finite() || speed <= 0.0 {
            return Err(error(format!(
                "max_cut_speed must be > 0, got {speed}"
            )));
        }
        self.max_cut_speed = speed;
        Ok(())
    }

    /// Sets the laser's minimum firing power.
    pub fn set_power_floor(&mut self, floor: f64) -> Result<(), GlueError> {
        if !(0.0..=100.0).contains(&floor) {
            return Err(error(format!(
                "power_floor must be between 0 and 100, got {floor}"
            )));
        }
        self.power_floor = floor;
        Ok(())
    }

    /// Enables or disables effective-min power scaling.
    pub fn set_power_scaling_enabled(&mut self, enabled: bool) {
        self.power_scaling_enabled = enabled;
        self.record("set_power_scaling_enabled", &[Literal::Bool(enabled)]);
    }

    fn effective_min_power(
        &self,
        min_power: f64,
        max_power: f64,
        cut_speed: f64,
    ) -> f64 {
        let max_cut_speed = self.max_cut_speed;
        let floor = self.power_floor;
        let eff_cut_speed = cut_speed.clamp(0.0, max_cut_speed);
        let unbiased_min = (min_power - floor).max(0.0);
        let unbiased_max = max_power.min(100.0) - floor;
        let span = unbiased_max - unbiased_min;
        unbiased_max - ((eff_cut_speed / max_cut_speed) * span) + floor
    }

    // -- movement ---------------------------------------------------------

    fn choose_move_form(delta: f64) -> &'static str {
        if (-8.192..=8.191).contains(&delta) {
            "NEAR"
        } else {
            "FAR"
        }
    }

    fn format_coord(
        form: &str,
        axis: &str,
        absolute: f64,
        delta: f64,
    ) -> String {
        if form == "NEAR" {
            format!("near{axis}={delta:.3}mm")
        } else {
            format!("{axis}={absolute:.3}mm")
        }
    }

    fn expand_bounding_boxes(&mut self, x: f64, y: f64) {
        self.last_layer_has_content = true;
        self.layer_trx = self.layer_trx.min(x);
        self.layer_try = self.layer_try.min(y);
        self.layer_blx = self.layer_blx.max(x);
        self.layer_bly = self.layer_bly.max(y);
        self.doc_tr_x = self.doc_tr_x.min(x);
        self.doc_tr_y = self.doc_tr_y.min(y);
        self.doc_bl_x = self.doc_bl_x.max(x);
        self.doc_bl_y = self.doc_bl_y.max(y);
    }

    /// Moves to an absolute XY position.
    pub fn move_xy_to(&mut self, x: f64, y: f64) {
        self.record("move_xy_to", &[Literal::Float(x), Literal::Float(y)]);
        self.emit_move_xy(x, y);
    }

    fn emit_move_xy(&mut self, x: f64, y: f64) {
        let dx = x - self.current_x;
        let dy = y - self.current_y;
        let form = if Self::choose_move_form(dx) == "NEAR"
            && Self::choose_move_form(dy) == "NEAR"
        {
            "NEAR"
        } else {
            "FAR"
        };
        self.layer_actions
            .entry(self.layer)
            .or_default()
            .push(format!(
                "MOVE_{form}_XY {} {}",
                Self::format_coord(form, "X", x, dx),
                Self::format_coord(form, "Y", y, dy)
            ));
        self.current_x = x;
        self.current_y = y;
        self.expand_bounding_boxes(x, y);
    }

    /// Moves to an absolute X position.
    pub fn move_x_to(&mut self, x: f64) {
        self.record("move_x_to", &[Literal::Float(x)]);
        self.emit_move_x(x);
    }

    fn emit_move_x(&mut self, x: f64) {
        let dx = x - self.current_x;
        let form = Self::choose_move_form(dx);
        let line = if form == "NEAR" {
            format!("MOVE_NEAR_X {}", Self::format_coord(form, "X", x, dx))
        } else {
            format!("MOVE_FAR_XY X={x:.3}mm Y={:.3}mm", self.current_y)
        };
        self.layer_actions.entry(self.layer).or_default().push(line);
        self.current_x = x;
        self.expand_bounding_boxes(x, self.current_y);
    }

    /// Moves to an absolute Y position.
    pub fn move_y_to(&mut self, y: f64) {
        self.record("move_y_to", &[Literal::Float(y)]);
        self.emit_move_y(y);
    }

    fn emit_move_y(&mut self, y: f64) {
        let dy = y - self.current_y;
        let form = Self::choose_move_form(dy);
        let line = if form == "NEAR" {
            format!("MOVE_NEAR_Y {}", Self::format_coord(form, "Y", y, dy))
        } else {
            format!("MOVE_FAR_XY X={:.3}mm Y={y:.3}mm", self.current_x)
        };
        self.layer_actions.entry(self.layer).or_default().push(line);
        self.current_y = y;
        self.expand_bounding_boxes(self.current_x, y);
    }

    /// Cuts to an absolute XY position.
    pub fn cut_xy_to(&mut self, x: f64, y: f64) -> Result<(), GlueError> {
        self.record("cut_xy_to", &[Literal::Float(x), Literal::Float(y)]);
        self.emit_cut_xy(x, y)
    }

    fn emit_cut_xy(&mut self, x: f64, y: f64) -> Result<(), GlueError> {
        self.flush_layer_settings()?;
        let dx = x - self.current_x;
        let dy = y - self.current_y;
        let form = if Self::choose_move_form(dx) == "NEAR"
            && Self::choose_move_form(dy) == "NEAR"
        {
            "NEAR"
        } else {
            "FAR"
        };
        self.layer_actions
            .entry(self.layer)
            .or_default()
            .push(format!(
                "CUT_{form}_XY {} {}",
                Self::format_coord(form, "X", x, dx),
                Self::format_coord(form, "Y", y, dy)
            ));
        self.current_x = x;
        self.current_y = y;
        self.expand_bounding_boxes(x, y);
        Ok(())
    }

    /// Cuts to an absolute X position.
    pub fn cut_x_to(&mut self, x: f64) -> Result<(), GlueError> {
        self.record("cut_x_to", &[Literal::Float(x)]);
        self.emit_cut_x(x)
    }

    fn emit_cut_x(&mut self, x: f64) -> Result<(), GlueError> {
        self.flush_layer_settings()?;
        let dx = x - self.current_x;
        let form = Self::choose_move_form(dx);
        let line = if form == "NEAR" {
            format!("CUT_NEAR_X {}", Self::format_coord(form, "X", x, dx))
        } else {
            format!("CUT_FAR_XY X={x:.3}mm Y={:.3}mm", self.current_y)
        };
        self.layer_actions.entry(self.layer).or_default().push(line);
        self.current_x = x;
        self.expand_bounding_boxes(x, self.current_y);
        Ok(())
    }

    /// Cuts to an absolute Y position.
    pub fn cut_y_to(&mut self, y: f64) -> Result<(), GlueError> {
        self.record("cut_y_to", &[Literal::Float(y)]);
        self.emit_cut_y(y)
    }

    fn emit_cut_y(&mut self, y: f64) -> Result<(), GlueError> {
        self.flush_layer_settings()?;
        let dy = y - self.current_y;
        let form = Self::choose_move_form(dy);
        let line = if form == "NEAR" {
            format!("CUT_NEAR_Y {}", Self::format_coord(form, "Y", y, dy))
        } else {
            format!("CUT_FAR_XY X={:.3}mm Y={y:.3}mm", self.current_x)
        };
        self.layer_actions.entry(self.layer).or_default().push(line);
        self.current_y = y;
        self.expand_bounding_boxes(self.current_x, y);
        Ok(())
    }

    fn flush_layer_settings(&mut self) -> Result<(), GlueError> {
        if !self.speed_dirty && !self.power_dirty {
            return Ok(());
        }
        let layer = self.layer;
        let speed_dirty = self.speed_dirty;
        let power_dirty = self.power_dirty;
        let mut emitted = Vec::new();
        if speed_dirty {
            emitted.push(format!(
                "CUT_SPEED_LASER_1 Layer:{} Speed={}",
                layer - 1,
                format_float(self.current_layer_speed)
            ));
        }
        if power_dirty {
            if self.frequency_changed {
                emitted.push(format!(
                    "LAYER_FREQUENCY Laser:0 Layer:{} Freq:{:.3}KHz",
                    layer - 1,
                    self.current_layer_frequency
                ));
            }
            emitted.push(format!("SELECT_LAYER Layer:{}", layer - 1));
            let emitted_min = if self.power_scaling_enabled {
                self.effective_min_power(
                    self.pending_min_power,
                    self.pending_max_power,
                    self.current_layer_speed,
                )
                .min(self.pending_max_power)
            } else {
                self.pending_min_power
            };
            emitted.push(format!(
                "LAYER_MIN_POWER_1 Layer:{} Power:{}%",
                layer - 1,
                format_float(emitted_min)
            ));
            emitted.push(format!(
                "LAYER_MAX_POWER_1 Layer:{} Power:{}%",
                layer - 1,
                format_float(self.pending_max_power)
            ));
            emitted.extend(self.pending_warnings.iter().cloned());
        }
        self.layer_actions.entry(layer).or_default().extend(emitted);
        if speed_dirty {
            self.speed_dirty = false;
        }
        if power_dirty {
            self.frequency_changed = false;
            self.power_dirty = false;
            self.pending_warnings.clear();
        }
        Ok(())
    }

    // -- compilation ------------------------------------------------------

    /// Assembles the rpascript, optionally replaying a transcript first.
    pub fn stage_gluescript(
        &mut self,
        lines: Option<&[String]>,
        require_complete: bool,
    ) -> Result<String, GlueError> {
        if let Some(lines) = lines {
            self.new_gluescript();
            self.power_dirty = true;
            self.assembling = true;
            let result = self.replay(lines);
            self.assembling = false;
            result?;
            if require_complete && !self.job_complete {
                return Err(error(
                    "re-staged gluescript is missing end_job".to_string(),
                ));
            }
        } else if !self.job_complete {
            return Err(error(
                "end_job must be called before stage_gluescript".to_string(),
            ));
        }
        self.assemble_rpascript();
        Ok(self.signature())
    }

    fn replay(&mut self, lines: &[String]) -> Result<(), GlueError> {
        for line in join_continuation_lines(lines) {
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let (name, args) = parse_call(&line)
                .map_err(|err| error(format!("{err}: {line:?}")))?;
            self.dispatch(&name, &args)?;
        }
        Ok(())
    }

    fn dispatch(
        &mut self,
        name: &str,
        args: &[Literal],
    ) -> Result<(), GlueError> {
        match name {
            "comment" => {
                let lines = args.first().and_then(Literal::as_list);
                if let Some(lines) = lines {
                    let comments: Vec<String> = lines
                        .iter()
                        .filter_map(|line| line.as_string().map(str::to_string))
                        .collect();
                    self.comment(&comments);
                }
            }
            "declare_job" => {
                self.declare_job(
                    string_arg(args, 0)?,
                    string_arg(args, 1).unwrap_or("MACHINE"),
                    list_arg(args, 2),
                    int_arg(args, 3).unwrap_or(1),
                    int_arg(args, 4).unwrap_or(1),
                    float_arg(args, 5).unwrap_or(0.0),
                    float_arg(args, 6).unwrap_or(0.0),
                )?;
            }
            "end_job" => self.end_job()?,
            "declare_layer" => {
                self.declare_layer(
                    string_arg(args, 0)?,
                    string_arg(args, 1)?,
                    string_arg(args, 2).unwrap_or("VECTOR"),
                    string_arg(args, 3).unwrap_or("NONE"),
                    float_arg(args, 4).unwrap_or(100.0),
                    float_arg(args, 5).unwrap_or(20.0),
                    float_arg(args, 6).unwrap_or(8.0),
                    float_arg(args, 7).unwrap_or(70.0),
                )?;
            }
            "power" => self.power(float_arg(args, 0)),
            "power_range" => {
                self.power_range(float_arg(args, 0), float_arg(args, 1))?
            }
            "cut_speed" => self.cut_speed(float_arg(args, 0).unwrap_or(100.0)),
            "move_speed" => {
                self.move_speed(float_arg(args, 0).unwrap_or(100.0))
            }
            "frequency" => self.frequency(float_arg(args, 0).unwrap_or(20.0)),
            "pwm" => self.pwm(float_arg(args, 0).unwrap_or(0.0)),
            "select_laser" => self.select_laser(int_arg(args, 0).unwrap_or(1)),
            "air_assist_on" => self.air_assist_on(),
            "air_assist_off" => self.air_assist_off(),
            "set_mode" => self.set_mode(string_arg(args, 0)?)?,
            "set_overscan" => self.set_overscan(string_arg(args, 0)?)?,
            "set_max_cut_speed" => {
                self.set_max_cut_speed(float_arg(args, 0).unwrap_or(400.0))?
            }
            "set_power_floor" => {
                self.set_power_floor(float_arg(args, 0).unwrap_or(8.0))?
            }
            "set_power_scaling_enabled" => self
                .set_power_scaling_enabled(bool_arg(args, 0).unwrap_or(true)),
            "move_xy_to" => {
                let (x, y) = xy_args(args);
                self.move_xy_to(x, y);
            }
            "move_x_to" => self.move_x_to(float_arg(args, 0).unwrap_or(0.0)),
            "move_y_to" => self.move_y_to(float_arg(args, 0).unwrap_or(0.0)),
            "cut_xy_to" => {
                let (x, y) = xy_args(args);
                self.cut_xy_to(x, y)?;
            }
            "cut_x_to" => self.cut_x_to(float_arg(args, 0).unwrap_or(0.0))?,
            "cut_y_to" => self.cut_y_to(float_arg(args, 0).unwrap_or(0.0))?,
            "stage_gluescript" | "stage_gluescript_delta" => {}
            _ => {
                return Err(error(format!(
                    "unknown gluescript command: {name:?}"
                )))
            }
        }
        Ok(())
    }

    fn assemble_rpascript(&mut self) {
        let mut script = Vec::new();
        script.extend(self.job_header.iter().cloned());
        script.extend(self.inline_prelude.iter().cloned());
        for attrs in self.layer_attributes.values() {
            script.extend(attrs.iter().cloned());
        }
        if !self.layer_attributes.is_empty() {
            let last = *self.layer_attributes.keys().max().unwrap_or(&1);
            script.push(format!("LAST_LAYER Layer:{}", last - 1));
        }
        let mut layers: Vec<usize> =
            self.layer_actions.keys().copied().collect();
        for layer in self.layer_overscan.keys() {
            if !layers.contains(layer) {
                layers.push(*layer);
            }
        }
        layers.sort_unstable();
        for layer in layers {
            script.push(format!("SELECT_LAYER Layer:{}", layer - 1));
            if let Some(lines) = self.layer_overscan.get(&layer) {
                script.extend(lines.iter().cloned());
            }
            if let Some(lines) = self.layer_actions.get(&layer) {
                script.extend(lines.iter().cloned());
            }
        }
        script.extend(self.inline_epilogue.iter().cloned());
        script.push("END_JOB".to_string());
        script.push("EOF".to_string());
        self.rpascript = script
            .iter()
            .map(|line| expand_deferred(line, self))
            .collect();
        self.stage_complete = true;
    }

    fn signature(&self) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.gluescript.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

fn overscan_lines(mode: &str) -> Vec<String> {
    match mode {
        "NONE" => vec!["OVERSCAN_OFF".to_string()],
        "X" => vec!["OVERSCAN_H_UNI".to_string()],
        "X_BI" => vec!["OVERSCAN_H_BI".to_string()],
        "Y" => vec!["OVERSCAN_V_UNI".to_string()],
        "Y_BI" => vec!["OVERSCAN_V_BI".to_string()],
        "XY" => vec![
            "# Diagonal overscan is not supported.".to_string(),
            "OVERSCAN_OFF".to_string(),
        ],
        _ => Vec::new(),
    }
}

fn expand_deferred(line: &str, script: &GlueScript) -> String {
    let mut out = line.to_string();
    for (key, value) in [
        ("{self.doc_tr_x}", script.doc_tr_x),
        ("{self.doc_tr_y}", script.doc_tr_y),
        ("{self.doc_bl_x}", script.doc_bl_x),
        ("{self.doc_bl_y}", script.doc_bl_y),
        ("{self._abs_xy[0]}", script.abs_xy[0]),
        ("{self._abs_xy[1]}", script.abs_xy[1]),
    ] {
        if out.contains(key) {
            let text = if value.is_infinite() {
                "inf".to_string()
            } else {
                format!("{value:.3}")
            };
            out = out.replace(key, &text);
        }
    }
    out
}

fn optional_literal(value: Option<f64>) -> Literal {
    match value {
        Some(value) => Literal::Float(value),
        None => Literal::None,
    }
}

fn string_arg(args: &[Literal], index: usize) -> Result<&str, GlueError> {
    args.get(index)
        .and_then(Literal::as_string)
        .ok_or_else(|| error(format!("missing string argument {index}")))
}

fn float_arg(args: &[Literal], index: usize) -> Option<f64> {
    args.get(index).and_then(Literal::as_f64)
}

fn int_arg(args: &[Literal], index: usize) -> Option<i64> {
    match args.get(index) {
        Some(Literal::Int(value)) => Some(*value),
        Some(Literal::Float(value)) => Some(*value as i64),
        _ => None,
    }
}

fn bool_arg(args: &[Literal], index: usize) -> Option<bool> {
    match args.get(index) {
        Some(Literal::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn list_arg(args: &[Literal], index: usize) -> Option<[f64; 2]> {
    let list = args.get(index)?.as_list()?;
    let x = list.first()?.as_f64()?;
    let y = list.get(1)?.as_f64()?;
    Some([x, y])
}

fn xy_args(args: &[Literal]) -> (f64, f64) {
    (
        float_arg(args, 0).unwrap_or(0.0),
        float_arg(args, 1).unwrap_or(0.0),
    )
}

impl Literal {
    fn as_list(&self) -> Option<&[Literal]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }
}

fn format_call(name: &str, args: &[Literal]) -> Vec<String> {
    let single = format!(
        "{name}({})",
        args.iter()
            .map(Literal::repr)
            .collect::<Vec<_>>()
            .join(", ")
    );
    if single.len() <= WRAP_WIDTH {
        return vec![single];
    }
    let mut lines = vec![format!("{name}(")];
    for arg in args {
        let rendered = render_literal(arg, INDENT);
        lines.extend(rendered);
        let last = lines.last_mut().expect("literal line");
        last.push(',');
    }
    lines.push(")".to_string());
    lines
}

fn render_literal(value: &Literal, indent: usize) -> Vec<String> {
    let inline = value.repr();
    if inline.len() + indent <= WRAP_WIDTH {
        return vec![format!("{}{inline}", " ".repeat(indent))];
    }
    let Literal::List(values) = value else {
        return vec![format!("{}{inline}", " ".repeat(indent))];
    };
    let mut lines = vec![format!("{}[", " ".repeat(indent))];
    for item in values {
        let child = indent + INDENT;
        let rendered = render_literal(item, child);
        lines.extend(rendered);
        lines.last_mut().expect("item").push(',');
    }
    lines.push(format!("{}]", " ".repeat(indent)));
    lines
}

fn join_continuation_lines(lines: &[String]) -> Vec<String> {
    let mut joined = Vec::new();
    let mut pending: Option<String> = None;
    for line in lines {
        let mut text = line.to_string();
        if let Some(previous) = pending.take() {
            text = format!("{previous}{}", line.trim_start());
        }
        if bracket_depth(&text) > 0 {
            pending = Some(text);
        } else {
            joined.push(text);
        }
    }
    if let Some(previous) = pending {
        joined.push(previous);
    }
    joined
}

fn bracket_depth(line: &str) -> i32 {
    let mut depth = 0i32;
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(quote) = in_string {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => in_string = Some(ch),
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            '#' => break,
            _ => {}
        }
    }
    depth
}

fn parse_call(line: &str) -> Result<(String, Vec<Literal>), String> {
    let line = line.trim();
    let open = line
        .find('(')
        .ok_or_else(|| "missing argument list".to_string())?;
    let name = line[..open].trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(format!("invalid command name: {name:?}"));
    }
    let body = line
        .strip_prefix(&format!("{name}("))
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or_else(|| "unbalanced call".to_string())?;
    let mut parser = LiteralParser {
        text: body,
        index: 0,
    };
    let mut args = Vec::new();
    parser.skip_whitespace();
    if parser.index < parser.text.len() {
        loop {
            args.push(parser.parse_literal()?);
            parser.skip_whitespace();
            if parser.index >= parser.text.len() {
                break;
            }
            if parser.text[parser.index..].starts_with(',') {
                parser.index += 1;
                parser.skip_whitespace();
                if parser.index >= parser.text.len() {
                    break;
                }
            } else {
                return Err(format!(
                    "unexpected token at {}",
                    &parser.text[parser.index..]
                ));
            }
        }
    }
    Ok((name.to_string(), args))
}

struct LiteralParser<'a> {
    text: &'a str,
    index: usize,
}

impl LiteralParser<'_> {
    fn skip_whitespace(&mut self) {
        while self.text[self.index..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            self.index += 1;
        }
    }

    fn parse_literal(&mut self) -> Result<Literal, String> {
        self.skip_whitespace();
        let rest = &self.text[self.index..];
        if rest.starts_with('[') {
            self.index += 1;
            let mut values = Vec::new();
            loop {
                self.skip_whitespace();
                if self.text[self.index..].starts_with(']') {
                    self.index += 1;
                    break;
                }
                values.push(self.parse_literal()?);
                self.skip_whitespace();
                if self.text[self.index..].starts_with(',') {
                    self.index += 1;
                }
            }
            return Ok(Literal::List(values));
        }
        if rest.starts_with('\'') || rest.starts_with('"') {
            let quote = rest.chars().next().expect("quote");
            self.index += 1;
            let mut value = String::new();
            while let Some(ch) = self.text[self.index..].chars().next() {
                self.index += ch.len_utf8();
                if ch == '\\' {
                    let escaped = self.text[self.index..]
                        .chars()
                        .next()
                        .ok_or("unterminated escape")?;
                    self.index += escaped.len_utf8();
                    value.push(match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    });
                } else if ch == quote {
                    return Ok(Literal::Str(value));
                } else {
                    value.push(ch);
                }
            }
            return Err("unterminated string".to_string());
        }
        if rest.starts_with("True") {
            self.index += 4;
            return Ok(Literal::Bool(true));
        }
        if rest.starts_with("False") {
            self.index += 5;
            return Ok(Literal::Bool(false));
        }
        if rest.starts_with("None") {
            self.index += 4;
            return Ok(Literal::None);
        }
        let end = rest.find([',', ']', ')']).unwrap_or(rest.len());
        let token = rest[..end].trim();
        self.index += end;
        if token.contains('.') || token.contains('e') || token.contains('E') {
            token
                .parse::<f64>()
                .map(Literal::Float)
                .map_err(|_| format!("invalid number: {token:?}"))
        } else {
            token
                .parse::<i64>()
                .map(Literal::Int)
                .map_err(|_| format!("invalid number: {token:?}"))
        }
    }
}
