//! Serializable data structures for G-code encoding.
//!
//! These types cross the Python -> Rust boundary as plain serde-compatible
//! data. They encode *operational parameters* for the G-code encoder
//! (machine heads, precision, macro table) and a flat dictionary of
//! *pre-resolved template variables*.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Template strings and flags defining a G-code dialect.
///
/// Built in Python from a `GcodeDialect` dataclass instance. Field
/// names match the dataclass attributes so that a plain `asdict()`
/// result round-trips through serde.
#[derive(Debug, Clone, Deserialize)]
pub struct GcodeDialectSpec {
    pub laser_on: String,
    pub laser_off: String,
    pub tool_change: String,
    pub set_speed: String,
    pub travel_move: String,
    pub linear_move: String,
    pub arc_cw: String,
    pub arc_ccw: String,
    #[serde(default)]
    pub bezier_cubic: String,
    pub air_assist_on: String,
    pub air_assist_off: String,
    #[serde(default = "default_spindle_on_cw")]
    pub spindle_on_cw: String,
    #[serde(default = "default_spindle_on_ccw")]
    pub spindle_on_ccw: String,
    #[serde(default = "default_spindle_off")]
    pub spindle_off: String,
    #[serde(default = "default_coolant_flood")]
    pub coolant_flood: String,
    #[serde(default = "default_coolant_mist")]
    pub coolant_mist: String,
    #[serde(default = "default_coolant_off")]
    pub coolant_off: String,
    #[serde(default = "default_dwell")]
    pub dwell: String,
    #[serde(default)]
    pub preamble: Vec<String>,
    #[serde(default)]
    pub postscript: Vec<String>,
    #[serde(default = "default_true")]
    pub inject_wcs_after_preamble: bool,
    #[serde(default)]
    pub can_g0_with_speed: bool,
    #[serde(default = "default_true")]
    pub omit_unchanged_coords: bool,
    #[serde(default)]
    pub continuous_laser_mode: bool,
    #[serde(default)]
    pub modal_feedrate: bool,
    /// Number of decimal digits for coordinate/feed/power formatting.
    #[serde(default = "default_gcode_precision")]
    pub gcode_precision: u8,
}

fn default_true() -> bool {
    true
}
fn default_gcode_precision() -> u8 {
    3
}
fn default_unit_scale() -> f64 {
    1.0
}
fn default_spindle_on_cw() -> String {
    "M3 S{rpm}".to_string()
}
fn default_spindle_on_ccw() -> String {
    "M4 S{rpm}".to_string()
}
fn default_spindle_off() -> String {
    "M5".to_string()
}
fn default_coolant_flood() -> String {
    "M8".to_string()
}
fn default_coolant_mist() -> String {
    "M7".to_string()
}
fn default_coolant_off() -> String {
    "M9".to_string()
}
fn default_dwell() -> String {
    "G4 P{seconds:.3f}".to_string()
}

/// Description of a single laser head on the machine. Identified by UID
/// because `Ops` marker/state commands reference heads by UID.
#[derive(Debug, Clone, Deserialize)]
pub struct LaserHeadSpec {
    pub uid: String,
    pub tool_number: u32,
    pub max_power: f64,
}

/// A named macro block. Macros are keyed by name so `@include(name)`
/// directives in macro code can resolve them. The Python caller decides
/// which macro to emit at each lifecycle trigger.
#[derive(Debug, Clone, Deserialize)]
pub struct Macro {
    pub name: String,
    pub code: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Macros that may be emitted at lifecycle trigger points. Each is
/// optional; `None` means no macro for that trigger.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MacroTable {
    #[serde(default)]
    pub layer_start: Option<Macro>,
    #[serde(default)]
    pub layer_end: Option<Macro>,
    #[serde(default)]
    pub workpiece_start: Option<Macro>,
    #[serde(default)]
    pub workpiece_end: Option<Macro>,
    /// All macros by name, for `@include(name)` resolution within macro
    /// bodies. This is the union of the four named triggers above plus
    /// any additional user-defined macros that may be referenced via
    /// `@include`.
    #[serde(default)]
    pub all_macros: HashMap<String, Macro>,
}

/// Everything the encoder needs as *operational* data.
///
/// Path-style template variables (e.g. `{machine.name}`,
/// `{job.extents[0]}`, `{layer.name}`) are NOT resolved here; the
/// Python caller pre-resolves them into [`EncodeContext::path_vars`]
/// as flat `String -> String` entries before calling the encoder.
#[derive(Debug, Clone, Deserialize)]
pub struct EncodeContext {
    /// Number of decimal digits for coordinate/feed/power output.
    #[serde(default = "default_gcode_precision")]
    pub gcode_precision: u8,
    /// Max travel (rapid) speed in mm/min, used to clamp rapid rates.
    #[serde(default)]
    pub max_travel_speed: f64,
    /// Multiplier applied to convert internal mm values into the
    /// machine's unit system at G-code emission time. ``1.0`` for
    /// metric machines, ``1/25.4`` for imperial. Coordinates and
    /// feed rates are multiplied by this before formatting.
    #[serde(default = "default_unit_scale")]
    pub unit_scale: f64,
    /// UID of the default laser head (used when no SetHead has run).
    #[serde(default)]
    pub default_head_uid: String,
    /// Laser heads available on the machine.
    #[serde(default)]
    pub heads: Vec<LaserHeadSpec>,
    /// WCS command to inject immediately after the preamble, e.g. "G54".
    /// Empty string => no injection.
    #[serde(default)]
    pub active_wcs: String,
    /// Per-layer WCS name, keyed by the layer UID found in
    /// `LayerStart` markers. Used by the encoder's WCS tracking state
    /// machine (not for template expansion).
    #[serde(default)]
    pub layer_wcs: HashMap<String, String>,
    /// Macro table for lifecycle triggers.
    #[serde(default)]
    pub macros: MacroTable,
    /// Pre-resolved path-style template variables. Keys are the full
    /// path as written in the dialect templates, e.g. `"machine.name"`,
    /// `"job.extents[0]"`, `"wcs_offset[2]"`, `"layer.name"`,
    /// `"workpiece.size[0]"`. Values are the resolved string forms.
    /// Looked up by exact (case-sensitive) match only.
    #[serde(default)]
    pub path_vars: HashMap<String, String>,
    /// Per-layer path_vars overrides. Keyed by layer UID. When the
    /// encoder encounters a `LayerStart` marker, it merges the
    /// corresponding entry into `path_vars` so that macros expanding
    /// in the layer context see `{layer.name}` etc.
    #[serde(default)]
    pub layer_path_vars: HashMap<String, HashMap<String, String>>,
    /// Per-workpiece path_vars overrides. Keyed by workpiece UID.
    /// Same semantics as `layer_path_vars`.
    #[serde(default)]
    pub workpiece_path_vars: HashMap<String, HashMap<String, String>>,
}

/// Result of encoding: the G-code text and the bidirectional map between
/// ops command indices and emitted G-code line indices.
#[derive(Debug, Clone, Serialize)]
pub struct EncodeResult {
    pub text: String,
    pub op_to_machine_code: Vec<OpLineRange>,
    pub machine_code_to_op: Vec<usize>,
}

/// Contiguous run of emitted machine-code lines for a single op.
///
/// Every op's emitted lines form one contiguous span ``[start,
/// start + len)``, so a per-op ``Vec<usize>`` can be replaced by a
/// single (``start``, ``len``) pair — one per op index, in order.
/// ``len == 0`` means the op produced no lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OpLineRange {
    pub start: u32,
    pub len: u32,
}

/// Sentinel value for ``machine_code_to_op`` entries that have no
/// owning op (e.g. the trailing empty line added by the encoder).
pub const MACHINE_CODE_TO_OP_NONE: usize = usize::MAX;
