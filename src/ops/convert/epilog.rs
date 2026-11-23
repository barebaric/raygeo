//! Encode machine-space Ops as an Epilog laser print job.
//!
//! Epilog laser cutters (Zing 16 / Zing 24-6030) do not speak G-code.
//! They accept a print job: a PJL envelope wrapping a PCL raster part
//! and an HPGL vector part, submitted to the machine's print queue.
//! This encoder produces both the human-readable representation that
//! Rayforge shows and exports, and the exact byte stream the driver
//! uploads.
//!
//! The wire format follows LibLaserCut's `EpilogCutter`, the
//! reverse-engineered reference for these machines:
//!
//! ```text
//! \e%-12345X@PJL JOB NAME=<title>
//! \eE@PJL ENTER LANGUAGE=PCL
//! <PCL raster setup>
//! <PCL raster rows>          (optional)
//! \e*rC
//! \e%1BIN;WF0;<HPGL vector>
//! PU;YP000;WF0;
//! \eE\e%-12345X@PJL EOJ
//! <4096 zero bytes>
//! ```
//!
//! Raster engraving uses 1-bit rows for dithered or thresholded
//! rasters (`*b2M` PackBits rows), and 8-bit rows carrying per-dot
//! power for power-modulated rasters (the driver's "3D mode"),
//! matching how the step's depth mode presents the image.

use std::fmt::Write as _;

use serde::Deserialize;

use crate::geo::types::Point3D;
use crate::ops::callbacks::Callbacks;
use crate::ops::container::Ops;
use crate::ops::convert::gcode_types::{OpLineRange, MACHINE_CODE_TO_OP_NONE};
use crate::ops::convert::{EncodeCtx, EncodeOutput, Encoder};
use crate::ops::enums::{CommandType, RasterMode, SectionType};
use crate::ops::state::AirAssistMode;
use crate::ops::types::{MarkerCmd, MoveCmd, OpCategory, StateCmd};

/// Supported raster resolutions in dots per inch.
pub const RESOLUTIONS: [u32; 6] = [100, 200, 250, 400, 500, 1000];

/// Default raster/vector resolution.
pub const DEFAULT_RESOLUTION: u32 = 500;

/// Default vector frequency in Hz.
pub const DEFAULT_FREQUENCY: i32 = 1000;

/// Lower bound of the vector frequency range in Hz.
pub const MIN_FREQUENCY: i32 = 10;
/// Upper bound of the vector frequency range in Hz.
pub const MAX_FREQUENCY: i32 = 5000;

/// Trailing zero padding Epilog expects after the PJL end-of-job.
const TRAILING_PADDING_BYTES: usize = 4096;

const PJL_START: &[u8] = b"\x1b%-12345X@PJL JOB NAME=";
const PJL_ENTER_PCL: &[u8] = b"\x1bE@PJL ENTER LANGUAGE=PCL\r\n";
const PJL_FOOTER: &[u8] = b"\x1bE\x1b%-12345X@PJL EOJ \r\n";

/// Power percent from which a move burns (`PD` instead of `PU`).
const MIN_BURN_PERCENT: u32 = 1;

/// JSON-serialised job configuration.
///
/// The spec stays plain data so it can cross the Python boundary; the
/// Rayforge side fills it from the machine and document models.
#[derive(Clone, Debug, Deserialize)]
pub struct EpilogContext {
    /// Job title shown on the machine's job list.
    #[serde(default)]
    pub title: Option<String>,
    /// Raster and vector resolution in DPI.
    #[serde(default = "default_resolution")]
    pub resolution: u32,
    /// Vector frequency in Hz.
    #[serde(default = "default_frequency")]
    pub frequency: i32,
    /// Machine bed `(width, height)` in millimetres.
    pub bed_mm: (f64, f64),
    /// Speed that corresponds to 100 % on the machine, in mm/min.
    pub max_cut_speed: f64,
    /// Whether the machine uses a top-left origin (Y down).
    ///
    /// The job language only addresses a top-left page, so the
    /// embedding application constrains the machine's origin to match;
    /// a `false` here therefore indicates a configuration that the
    /// application should not allow, not a user choice to report.
    #[serde(default = "default_true")]
    pub origin_top_left: bool,
    /// The document has a rotary layer, which the machine cannot drive.
    #[serde(default)]
    pub rotary: bool,
}

fn default_resolution() -> u32 {
    DEFAULT_RESOLUTION
}

fn default_frequency() -> i32 {
    DEFAULT_FREQUENCY
}

fn default_true() -> bool {
    true
}

/// Spec for the Epilog print-job encoder.
#[derive(Clone, Debug)]
pub struct EpilogSpec {
    /// JSON-serialised [`EpilogContext`].
    pub context_json: String,
}

impl Encoder for EpilogSpec {
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String> {
        ctx.callbacks.report_progress(0.0, "epilog: parse context");
        let context: EpilogContext = serde_json::from_str(&self.context_json)
            .map_err(|e| {
            format!("failed to deserialise epilog context: {e}")
        })?;

        ctx.callbacks.report_progress(0.25, "epilog: encode");
        let mut session = JobEncoder::new(&context)?;
        session.encode(ctx.ops, ctx.callbacks)?;

        ctx.callbacks.report_progress(1.0, "epilog: done");
        Ok(EncodeOutput::MachineCode {
            text: session.listing,
            op_to_machine_code: session.op_spans,
            machine_code_to_op: session.line_ops,
            payload: session.payload,
        })
    }

    fn name(&self) -> &str {
        "epilog"
    }
}

/// Power, speed and frequency last written to the job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MachineSettings {
    power_percent: u32,
    /// `None` until the first cutting feed rate sets it.
    speed_percent: Option<u32>,
    frequency: i32,
}

impl MachineSettings {
    fn new(frequency: i32) -> Self {
        Self {
            power_percent: 0,
            speed_percent: None,
            frequency,
        }
    }
}

/// Accumulates the payload and the printable listing for one job.
struct JobEncoder<'a> {
    ctx: &'a EpilogContext,
    payload: Vec<u8>,
    /// Printable representation, one line per emitted command.
    listing: String,
    /// Number of lines currently in `listing`.
    line_count: u32,
    /// Op index → `(start_line, line_count)`.
    op_spans: Vec<OpLineRange>,
    /// Line index → op index, or `MACHINE_CODE_TO_OP_NONE`.
    line_ops: Vec<usize>,
    settings: MachineSettings,
    /// Position in device dots, once a move has been emitted.
    position_dots: Option<(i64, i64)>,
    /// Laser power (0..=1) of the op being encoded.
    state_power: f64,
    /// Feed rate (mm/min) of the op being encoded, if any.
    feed_rate: Option<i32>,
    /// Air assist was requested somewhere in the job.
    air_assist_requested: bool,
    /// A powered cut was emitted.
    has_cut: bool,
    /// At least one raster row was emitted.
    has_raster: bool,
    /// Raster mode of the section being encoded.
    raster_mode: Option<RasterMode>,
    /// Rows collected in the current raster section.
    rows: Vec<RasterRow>,
    /// Op index currently being encoded.
    current_op: usize,
    /// Display-line cursor for the current op.
    op_line_start: u32,
}

impl<'a> JobEncoder<'a> {
    fn new(ctx: &'a EpilogContext) -> Result<Self, String> {
        if !ctx.origin_top_left {
            // Unreachable when the machine constrains its origin to the
            // driver's supported layout; kept as an invariant check so a
            // caller that bypasses that constraint fails loudly.
            return Err("internal error: the machine origin must be top-left"
                .to_string());
        }
        if !RESOLUTIONS.contains(&ctx.resolution) {
            return Err(format!(
                "unsupported Epilog resolution: {} DPI",
                ctx.resolution
            ));
        }
        if !(MIN_FREQUENCY..=MAX_FREQUENCY).contains(&ctx.frequency) {
            return Err(format!(
                "Epilog frequency must be between {MIN_FREQUENCY} and \
                 {MAX_FREQUENCY} Hz"
            ));
        }
        let (width, height) = ctx.bed_mm;
        let bed_valid = width.is_finite()
            && height.is_finite()
            && width > 0.0
            && height > 0.0;
        if !bed_valid {
            return Err("Epilog bed dimensions must be positive".to_string());
        }
        let speed_valid =
            ctx.max_cut_speed.is_finite() && ctx.max_cut_speed > 0.0;
        if !speed_valid {
            return Err("Epilog maximum cut speed must be positive".to_string());
        }
        if ctx.rotary {
            return Err("Epilog supports flat XY jobs only".to_string());
        }
        Ok(Self {
            ctx,
            payload: Vec::new(),
            listing: String::new(),
            line_count: 0,
            op_spans: Vec::new(),
            line_ops: Vec::new(),
            settings: MachineSettings::new(ctx.frequency),
            position_dots: None,
            state_power: 0.0,
            feed_rate: None,
            air_assist_requested: false,
            has_cut: false,
            has_raster: false,
            raster_mode: None,
            rows: Vec::new(),
            current_op: 0,
            op_line_start: 0,
        })
    }

    // ----------------------------------------------------------------
    // Emission primitives
    // ----------------------------------------------------------------

    /// Append one command: its device bytes and its readable line.
    ///
    /// The two are separate views of the same command.  Control
    /// characters are rendered as escapes so the listing stays
    /// printable and exports as plain text.
    fn emit(&mut self, bytes: &[u8]) {
        self.payload.extend_from_slice(bytes);
        let line = escape_listing(bytes);
        self.push_line(&line);
    }

    fn push_line(&mut self, readable: &str) {
        if self.line_count > 0 {
            self.listing.push('\n');
        }
        self.listing.push_str(readable);
        self.line_ops.push(self.current_op);
        self.line_count += 1;
    }

    fn dots_per_mm(&self) -> f64 {
        f64::from(self.ctx.resolution) / 25.4
    }

    fn to_dots(&self, mm: f64) -> i64 {
        (mm * self.dots_per_mm()).round() as i64
    }

    fn bed_dots(&self) -> (i64, i64) {
        let dpm = self.dots_per_mm();
        let width = (self.ctx.bed_mm.0 * dpm).ceil() as i64;
        let height = (self.ctx.bed_mm.1 * dpm).ceil() as i64;
        (width, height)
    }

    // ----------------------------------------------------------------
    // Job structure
    // ----------------------------------------------------------------

    fn encode(
        &mut self,
        ops: &Ops,
        callbacks: &dyn Callbacks,
    ) -> Result<(), String> {
        self.header();
        for i in 0..ops.len() {
            if i % 1000 == 0 && callbacks.is_cancelled() {
                return Err("cancelled".to_string());
            }
            self.current_op = i;
            self.op_line_start = self.line_count;
            self.handle(ops, i)?;
            self.finish_op_span();
        }
        self.finish()
    }

    fn finish_op_span(&mut self) {
        let len = self.line_count - self.op_line_start;
        self.op_spans.push(OpLineRange {
            start: self.op_line_start,
            len,
        });
    }

    fn header(&mut self) {
        let title =
            sanitize_title(self.ctx.title.as_deref().unwrap_or("Rayforge"));
        let mut job = Vec::new();
        job.extend_from_slice(PJL_START);
        job.extend_from_slice(title.as_bytes());
        job.extend_from_slice(b"\r\n");
        self.emit(&job);
        self.emit(PJL_ENTER_PCL);

        let dpi = self.ctx.resolution;
        let (width_dots, height_dots) = self.bed_dots();
        self.emit(b"\x1b&y0A\x1b&y0C\x1b&y0Z\x1b&l0U\x1b&l0Z");
        self.emit(format!("\x1b&u{dpi}D\x1b*p0X\x1b*p0Y").as_bytes());
        let setup = format!(
            "\x1b*t{dpi}R\x1b*r0F\x1b&y0P\x1b&z1S\x1b&y0A\
             \x1b*r{height_dots}T\x1b*r{width_dots}S\x1b&y0O\x1b*r1A"
        );
        self.emit(setup.as_bytes());
    }

    // ----------------------------------------------------------------
    // Command dispatch
    // ----------------------------------------------------------------

    fn handle(&mut self, ops: &Ops, idx: usize) -> Result<(), String> {
        let kind = ops.command_type(idx);
        let state = ops.state_at(idx);
        self.state_power = state.power;
        self.feed_rate = state.feed_rate;

        match kind {
            CommandType::SetPower
            | CommandType::SetFeedRate
            | CommandType::SetFrequency
            | CommandType::SetPowerMode
            | CommandType::SetRapidRate
            | CommandType::SetHead
            | CommandType::SetPulseWidth
            | CommandType::Dwell
            | CommandType::JobStart
            | CommandType::JobEnd
            | CommandType::LayerStart
            | CommandType::LayerEnd
            | CommandType::WorkpieceStart
            | CommandType::WorkpieceEnd
            | CommandType::StateBlockStart
            | CommandType::StateBlockEnd => Ok(()),

            CommandType::OpsSectionStart => {
                self.raster_mode = section_raster_mode(ops, idx);
                if self.raster_mode.is_some() {
                    self.begin_raster();
                }
                Ok(())
            }

            CommandType::SetAirAssist => {
                self.note_air_assist(ops, idx);
                Ok(())
            }

            CommandType::MoveTo if self.raster_mode.is_some() => Ok(()),
            CommandType::MoveTo => {
                let end = ops.endpoint(idx);
                self.move_to(end, false)
            }
            CommandType::LineTo => {
                let end = ops.endpoint(idx);
                self.move_to(end, true)
            }
            CommandType::ScanLine => self.on_scan_line(ops, idx),

            CommandType::ArcTo
            | CommandType::BezierTo
            | CommandType::QuadraticBezierTo => {
                let start = previous_end(ops, idx);
                let linear = ops.linearize(idx, start);
                for sub in 0..linear.len() {
                    let sub_kind = linear.command_type(sub);
                    if sub_kind == CommandType::LineTo {
                        let end = linear.endpoint(sub);
                        self.move_to(end, true)?;
                    } else if sub_kind == CommandType::MoveTo {
                        let end = linear.endpoint(sub);
                        self.move_to(end, false)?;
                    }
                }
                Ok(())
            }

            CommandType::OpsSectionEnd => {
                if self.raster_mode.is_some() {
                    self.flush_raster()?;
                }
                self.raster_mode = None;
                Ok(())
            }

            other => Err(format!("Unsupported Epilog operation: {other}")),
        }
    }

    fn note_air_assist(&mut self, ops: &Ops, idx: usize) {
        if let OpCategory::State(StateCmd::SetAirAssist(mode)) =
            &ops.commands[idx].category
        {
            if *mode == AirAssistMode::On {
                self.air_assist_requested = true;
            }
        }
    }

    // ----------------------------------------------------------------
    // Vector moves (HPGL)
    // ----------------------------------------------------------------

    fn move_to(&mut self, end: Point3D, cut: bool) -> Result<(), String> {
        if end.z.abs() > 1e-9 {
            return Err("Epilog cannot execute Z moves".to_string());
        }
        if cut && self.position_dots.is_none() {
            return Err(
                "move to a start position before an Epilog cut".to_string()
            );
        }
        if !end.x.is_finite() || !end.y.is_finite() {
            return Err("Epilog coordinates must be finite".to_string());
        }
        let power = self.power_percent()?;
        if cut {
            self.write_speed()?;
            self.write_power(power);
        }
        let x = self.to_dots(end.x);
        let y = self.to_dots(end.y);
        let (max_x, max_y) = self.bed_dots();
        let in_bed = (0..=max_x).contains(&x) && (0..=max_y).contains(&y);
        if !in_bed {
            return Err(format!(
                "Epilog path is outside the machine bed: X={:.6}, Y={:.6} mm",
                end.x, end.y
            ));
        }

        let burning = cut && power >= MIN_BURN_PERCENT;
        if burning {
            self.has_cut = true;
        }
        let pen = if burning { "PD" } else { "PU" };
        self.emit(format!("{pen}{x},{y};").as_bytes());
        self.position_dots = Some((x, y));
        Ok(())
    }

    fn power_percent(&self) -> Result<u32, String> {
        let power = self.state_power;
        if !power.is_finite() || !(0.0..=1.0).contains(&power) {
            return Err("Epilog power must be between 0 and 1".to_string());
        }
        Ok((power * 100.0 + 1e-9) as u32)
    }

    fn write_power(&mut self, percent: u32) {
        if percent == self.settings.power_percent {
            return;
        }
        self.settings.power_percent = percent;
        self.emit(format!("YP{percent:03};").as_bytes());
    }

    fn write_speed(&mut self) -> Result<(), String> {
        let percent = self.speed_percent()?;
        if self.settings.speed_percent == Some(percent) {
            return Ok(());
        }
        self.settings.speed_percent = Some(percent);
        self.emit(format!("ZS{percent:03};").as_bytes());
        Ok(())
    }

    fn speed_percent(&self) -> Result<u32, String> {
        // Ops without an explicit feed run at the machine's reference
        // speed (100 %); everything else maps to 1..=100 % of it.
        let reference = self.ctx.max_cut_speed;
        let rate = self.feed_rate.map(f64::from).unwrap_or(reference);
        if !rate.is_finite() || rate <= 0.0 || rate > reference {
            return Err(
                "Epilog cut speed must be within machine limits".to_string()
            );
        }
        let percent = (rate * 100.0 / reference + 0.5) as u32;
        if !(1..=100).contains(&percent) {
            return Err(
                "Epilog minimum speed is 1% of maximum speed".to_string()
            );
        }
        Ok(percent)
    }

    // ----------------------------------------------------------------
    // Raster sections (PCL)
    // ----------------------------------------------------------------

    fn on_scan_line(&mut self, ops: &Ops, idx: usize) -> Result<(), String> {
        let mode = self.raster_mode.unwrap_or(RasterMode::Dither);
        let row = self.row_from_scanline(ops, idx, mode)?;
        self.rows.push(row);
        Ok(())
    }

    fn row_from_scanline(
        &self,
        ops: &Ops,
        idx: usize,
        mode: RasterMode,
    ) -> Result<RasterRow, String> {
        let (power_values, end) = match &ops.commands[idx].category {
            OpCategory::Moving {
                cmd: MoveCmd::ScanLine { power_values },
                end,
            } => (power_values, *end),
            _ => return Err("expected a scan line".to_string()),
        };
        let start = previous_end(ops, idx);
        let dpm = self.dots_per_mm();
        let y_from = (start.y * dpm).round() as f64;
        let y_to = (end.y * dpm).round();
        if (y_from - y_to).abs() > 1.0 {
            return Err("Epilog raster rows must be horizontal (scan angle 0)"
                .to_string());
        }
        let row = y_to as i64;
        let (max_x, max_y) = self.bed_dots();
        if !(0..=max_y).contains(&row) {
            return Err(format!(
                "Epilog raster row outside the bed: Y={:.6} mm",
                end.y
            ));
        }
        let n = power_values.len();
        if n == 0 {
            return Ok(RasterRow {
                row: row as u32,
                start_dot: 0,
                dots: Vec::new(),
            });
        }
        let x0 = start.x * dpm;
        let step = if n > 1 {
            (end.x - start.x) * dpm / (n - 1) as f64
        } else {
            0.0
        };
        let eight_bit =
            matches!(mode, RasterMode::VariablePower | RasterMode::DepthMap);

        let mut dots = Vec::with_capacity(n);
        for &value in power_values.iter() {
            dots.push(if eight_bit {
                // One byte per dot: nominal power scaled by the pixel.
                (u32::from(value) * 100 / 255).min(100) as u8
            } else {
                u8::from(value > 0)
            });
        }
        let first = dots.iter().position(|&d| d > 0).unwrap_or(0);
        let last = dots.iter().rposition(|&d| d > 0).unwrap_or(0);
        if dots.iter().all(|&d| d == 0) {
            return Ok(RasterRow {
                row: row as u32,
                start_dot: 0,
                dots: Vec::new(),
            });
        }
        let start_dot = (x0 + step * first as f64).round();
        if start_dot < 0.0 || start_dot > max_x as f64 {
            return Err(format!(
                "Epilog raster outside the bed: X={:.6} mm",
                start.x
            ));
        }
        Ok(RasterRow {
            row: row as u32,
            start_dot: start_dot as u32,
            dots: dots[first..=last].to_vec(),
        })
    }

    /// Emit the PCL raster mode header for the current section.
    fn begin_raster(&mut self) {
        let power = self.current_power_percent();
        let speed = self.speed_percent();
        if let (Ok(power), Ok(speed)) = (power, speed) {
            self.emit(format!("\x1b&y{power}P\x1b&z{speed}S").as_bytes());
        }
        let eight_bit = matches!(
            self.raster_mode,
            Some(RasterMode::VariablePower | RasterMode::DepthMap)
        );
        if eight_bit {
            self.emit(b"\x1b*b1M\x1b*b7W");
        } else {
            self.emit(b"\x1b*b2M");
        }
    }

    /// Write the collected rows and leave the raster part.
    fn flush_raster(&mut self) -> Result<(), String> {
        let eight_bit = matches!(
            self.raster_mode,
            Some(RasterMode::VariablePower | RasterMode::DepthMap)
        );
        let mut rows = std::mem::take(&mut self.rows);
        rows.sort_by_key(|r| r.row);
        for row in rows {
            if row.dots.is_empty() {
                continue;
            }
            self.has_raster = true;
            self.write_raster_row(&row, eight_bit);
        }
        if self.has_raster {
            self.emit(b"\x1b*rC");
            self.emit(b"\x1b%1BIN;WF0;");
        }
        self.raster_mode = None;
        Ok(())
    }

    fn write_raster_row(&mut self, row: &RasterRow, eight_bit: bool) {
        let row_count = if eight_bit {
            row.dots.len()
        } else {
            row.dots.len().div_ceil(8)
        };
        // Raster rows are written top-to-bottom in screen order; the
        // machine's y=0 is the top edge for a top-left origin.
        self.emit(
            format!(
                "\x1b*p{}X\x1b*p{}Y\x1b*b{row_count}A\x1b*b{}W",
                row.start_dot, row.row, row_count
            )
            .as_bytes(),
        );
        let bytes = if eight_bit {
            encode_asebd(&row.dots)
        } else {
            encode_packbits(&pack_1bit(&row.dots))
        };
        self.emit(&bytes);
    }

    fn current_power_percent(&self) -> Result<u32, String> {
        self.power_percent()
    }

    /// Flush trailing state, close the job and append padding.
    fn finish(&mut self) -> Result<(), String> {
        if self.raster_mode.is_some() {
            self.flush_raster()?;
        }
        if !self.has_raster && !self.has_cut {
            return Err("the Epilog job has no powered paths".to_string());
        }
        self.emit(b"PU;YP000;WF0;");
        self.emit(PJL_FOOTER);
        if self.air_assist_requested {
            // The print protocol cannot switch air assist; it is set
            // at the machine.  Nothing to emit.
        }
        let padding = vec![0u8; TRAILING_PADDING_BYTES];
        self.payload.extend_from_slice(&padding);
        Ok(())
    }
}

/// One raster row: its device row index, first dot column and dots.
struct RasterRow {
    row: u32,
    start_dot: u32,
    dots: Vec<u8>,
}

/// The end point of the op preceding `idx`, or the origin.
fn previous_end(ops: &Ops, idx: usize) -> Point3D {
    (0..idx)
        .rev()
        .find_map(|i| ops.commands[i].as_moving().map(|(end, _)| *end))
        .unwrap_or(Point3D::ZERO)
}

/// The raster mode of the raster section starting at `idx`, if any.
fn section_raster_mode(ops: &Ops, idx: usize) -> Option<RasterMode> {
    match &ops.commands[idx].category {
        OpCategory::Marker(MarkerCmd::OpsSectionStart {
            section_type,
            raster_mode,
            ..
        }) if *section_type == SectionType::RasterFill => *raster_mode,
        _ => None,
    }
}

/// Pack 1-bit dots (one byte per dot, non-zero = burn) MSB-first.
fn pack_1bit(dots: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; dots.len().div_ceil(8)];
    for (i, &dot) in dots.iter().enumerate() {
        if dot > 0 {
            out[i / 8] |= 0x80 >> (i % 8);
        }
    }
    out
}

/// TIFF PackBits run-length compression.
fn encode_packbits(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let run = 1 + data[i..]
            .iter()
            .skip(1)
            .take_while(|&&b| b == data[i])
            .count();
        if run >= 2 {
            let count = run.min(128) as u8;
            out.push(0u8.wrapping_sub(count).wrapping_add(1));
            out.push(data[i]);
            i += count as usize;
        } else {
            let mut j = i;
            while j < data.len() && j - i < 128 {
                let next_run = 1 + data[j..]
                    .iter()
                    .skip(1)
                    .take_while(|&&b| b == data[j])
                    .count();
                if next_run >= 2 {
                    break;
                }
                j += 1;
            }
            let count = (j - i) as u8;
            out.push(count - 1);
            out.extend_from_slice(&data[i..j]);
            i = j;
        }
    }
    out
}

/// Adaptive run-length encode the 8-bit ("3D mode") dot power bytes.
///
/// Mode 1 rows carry one byte per dot; runs are encoded as a count
/// byte followed by the value, which is what the Zing accepts for
/// per-dot power rows.
fn encode_asebd(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let value = data[i];
        let run = 1 + data[i..]
            .iter()
            .skip(1)
            .take_while(|&&b| b == value)
            .count();
        let run = run.min(255);
        out.push(run as u8);
        out.push(value);
        i += run;
    }
    out
}

fn sanitize_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len().min(64));
    for ch in title.chars() {
        if out.len() >= 64 {
            break;
        }
        if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '.' | '_' | '-') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        "Rayforge".to_string()
    } else {
        trimmed
    }
}

/// Render bytes as one printable listing line.
fn escape_listing(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            0x1b => out.push_str("\\e"),
            b'\r' => {}
            b'\n' => out.push_str("\\n"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\x{b:02x}");
            }
        }
    }
    out
}

#[allow(dead_code)]
const _: usize = MACHINE_CODE_TO_OP_NONE;
