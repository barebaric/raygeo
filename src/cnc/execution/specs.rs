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
