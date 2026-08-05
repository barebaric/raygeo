use crate::ops::container::Ops;
use crate::ops::transform::Transformer;

pub struct AggregateSpec {
    pub wrap_start: Vec<Marker>,
    pub groups: Vec<AggregateGroup>,
    pub wrap_end: Vec<Marker>,
    pub machine: MachineParams,
    pub transformers: Vec<Box<dyn Transformer>>,
}

impl std::fmt::Debug for AggregateSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AggregateSpec")
            .field("wrap_start", &self.wrap_start)
            .field("groups", &self.groups)
            .field("wrap_end", &self.wrap_end)
            .field("machine", &self.machine)
            .field(
                "transformers",
                &format!("[{} transformers]", self.transformers.len()),
            )
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct AggregateGroup {
    pub start_markers: Vec<Marker>,
    pub inputs: Vec<AggregateInput>,
    pub end_markers: Vec<Marker>,
    pub link_mode: LinkMode,
}

/// Controls inter-input linking behavior in an [`AggregateGroup`].
///
/// When `Sequential`, travel moves (retract → XY travel → plunge)
/// are emitted between consecutive inputs using each input's
/// [`AssemblyOutput`](crate::ops::assembly::AssemblyOutput) meta.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LinkMode {
    /// No linking between inputs — they are simply concatenated.
    #[default]
    None,
    /// Emit travel links between consecutive inputs, retracting to
    /// `safe_z` between moves and lifting to `safe_z` after the last.
    Sequential { safe_z: f64 },
}

#[derive(Debug, Clone)]
pub struct AggregateInput {
    pub source_key: String,
    pub placement_matrix: [[f64; 4]; 4],
    pub uid: String,
    pub target_dimensions: (f64, f64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marker {
    JobStart,
    JobEnd,
    LayerStart { uid: String },
    LayerEnd { uid: String },
    WorkpieceStart { uid: String },
    WorkpieceEnd { uid: String },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MachineParams {
    pub default_feed_rate: f64,
    pub default_rapid_rate: f64,
    pub acceleration: f64,
}

#[derive(Debug, Clone)]
pub struct AggregateOutput {
    pub ops: Ops,
    pub time_estimate: Option<f64>,
}

// ── MachineTransformSpec ─────────────────────────────────────────────

/// Configuration for the machine-transform pipeline stage.
///
/// Converts world-space Ops into machine-space Ops by applying:
/// 1. Curve linearization (if the machine does not support curves)
/// 2. Per-layer rotary axis mapping (Y→degrees)
/// 3. World→machine coordinate transform (origin corner, reverse
///    axes, Z-flip) combined with the default WCS offset
/// 4. Per-layer WCS offset translation
/// 5. AXIS_REPLACEMENT degrees→scaled-mu downstream pass
#[derive(Debug, Clone)]
pub struct MachineTransformSpec {
    /// Key of the upstream node whose Ops to transform.
    pub source_key: String,
    /// When true, linearize Bezier curves before the other transforms.
    pub linearize_curves: bool,
    /// 4×4 world→machine matrix (row-major), including origin-corner
    /// and reverse-X/Y sign flips.
    pub world_to_machine: [[f64; 4]; 4],
    /// Default per-layer WCS command offset (x, y, z), subtracted
    /// from machine coords when a layer has no explicit entry.
    pub default_wcs_offset: [f64; 3],
    /// Per-layer WCS command offsets, keyed by layer UID.
    pub layer_wcs_offsets: Vec<(String, [f64; 3])>,
    /// When true, negate Z after the world→machine and WCS transforms.
    pub reverse_z: bool,
    /// Per-layer rotary mapping configs (empty when no rotary).
    pub rotary_mappings: Vec<RotaryMappingSpec>,
}

#[derive(Debug, Clone)]
pub struct RotaryMappingSpec {
    /// UID of the layer that this rotary config applies to.
    pub layer_uid: String,
    /// Rotary workpiece diameter (mm).
    pub diameter: f64,
    /// Gear ratio (roller-drive compensation).
    pub gear_ratio: f64,
    /// When true, negate the computed degree value.
    pub reverse: bool,
    /// Axis mount position in 3D space (x, y, z).
    pub axis_position_3d: [f64; 3],
    /// Cylinder direction vector (x, y, z — unit length).
    pub cylinder_dir: [f64; 3],
    /// Name of the rotary axis (e.g. "A", "B", "C").
    pub rotary_axis: String,
    /// Name of the world axis the rotary replaces, or None for
    /// TRUE_4TH_AXIS mode (e.g. "Y").
    pub replaced_axis: Option<String>,
    /// Millimetres of travel per full 360° rotation (for
    /// AXIS_REPLACEMENT degrees→mm conversion).  Zero means no
    /// conversion (raw degrees emitted on the replaced axis).
    pub mm_per_rotation: f64,
}
