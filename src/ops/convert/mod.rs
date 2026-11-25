//! Convert: Ops ↔ other format conversions.
//!
//! This module groups all functions that convert between [`Ops`] and
//! other representations — polylines, geometries, GPU vertex arrays,
//! pixel textures, view bitmaps, and images. Unlike the `transform`
//! module, which produces new `Ops` from existing `Ops`, the converters
//! here cross format boundaries.
//!
//! ## The `Encoder` trait
//!
//! Each encoder under this module exposes a **spec struct** (e.g.
//! [`vertex_arrays::VertexSpec`], [`texture::TextureSpec`],
//! [`view::ViewSpec`]) that implements [`Encoder`]. Callers drive any
//! encoder through this trait, mirroring how `ops::assembly` drives
//! assemblers through `Assembler` and `ops::transform` drives
//! transformers through `Transformer`. All three traits take their
//! callbacks from [`crate::ops::callbacks`].
//!
//! Machine-code encoders for specific devices (G-code dialects, print
//! jobs) live in their driver crate. They implement [`Encoder`] and
//! hand themselves to this module through the [`encoder_capsule_name`]
//! PyCapsule protocol, so the pipeline runs them without either side
//! depending on the other's Python types.

use serde::Serialize;

use crate::ops::container::Ops;
use crate::ops::convert::scene::CompiledSceneData;
use crate::ops::convert::vertex_arrays::VertexArrays;

pub mod dump;
pub mod geometry;
pub mod image;
pub mod polyline;
pub mod scene;
pub mod texture;
pub mod vertex_arrays;
pub mod view;

use crate::ops::callbacks::Callbacks;

/// Non-Ops artifact produced by an [`Encoder`].
///
/// Opaque to the caller; the variant's payload is the encoder's
/// native output shape.
#[derive(Debug, Clone)]
pub enum EncodeOutput {
    /// Machine-code text (G-code or any future machine language) +
    /// bidirectional op-to-line index maps.
    MachineCode {
        /// The machine-code text.
        text: String,
        /// Op index → emitted line span ``[start, start + len)``.
        op_to_machine_code: Vec<OpLineRange>,
        /// Emitted line index → op index (``usize::MAX`` = no op).
        machine_code_to_op: Vec<usize>,
        /// The exact byte stream the device consumes, when it differs
        /// from (or cannot be represented by) ``text``.  Encoders for
        /// binary machine languages (e.g. Epilog PJL/PCL/HPGL) keep a
        /// printable rendering in ``text`` and the real payload here.
        /// Empty for text-only languages such as G-code.
        payload: Vec<u8>,
    },
    /// GPU-friendly flat vertex buffers.
    VertexArrays(VertexArrays),
    /// 2D power texture + its dimensions.
    Texture {
        /// Flat `Vec<u8>` of length `width_px * height_px`.
        power_texture: Vec<u8>,
        /// Texture width in pixels.
        width_px: u32,
        /// Texture height in pixels.
        height_px: u32,
    },
    /// RGBA8 view bitmap + metadata.
    View {
        /// Flat row-major RGBA8 bytes of shape
        /// ``height * width * 4``.
        buffer: Vec<u8>,
        /// Bitmap width in pixels.
        width: usize,
        /// Bitmap height in pixels.
        height: usize,
        /// Bounding box in mm: ``(min_x, min_y, max_x, max_y)``.
        bbox_mm: (f64, f64, f64, f64),
        /// Effective pixels-per-mm applied after clamping.
        effective_ppm: (f64, f64),
    },
    /// GPU-ready 3D scene data (vertex groups + layer metadata).
    Scene(CompiledSceneData),
}

impl EncodeOutput {
    /// Estimated heap-allocated bytes for this output.
    pub fn heap_size(&self) -> usize {
        match self {
            EncodeOutput::MachineCode {
                text,
                op_to_machine_code,
                machine_code_to_op,
                payload,
            } => {
                text.len()
                    + op_to_machine_code.len()
                        * std::mem::size_of::<OpLineRange>()
                    + machine_code_to_op.len() * std::mem::size_of::<usize>()
                    + payload.len()
            }
            EncodeOutput::VertexArrays(va) => {
                let f32_size = std::mem::size_of::<f32>();
                va.powered_vertices.len() * f32_size
                    + va.powered_colors.len() * f32_size
                    + va.travel_vertices.len() * f32_size
                    + va.zero_power_vertices.len() * f32_size
            }
            EncodeOutput::Texture { power_texture, .. } => power_texture.len(),
            EncodeOutput::View { buffer, .. } => buffer.len(),
            EncodeOutput::Scene(data) => {
                let f32_size = std::mem::size_of::<f32>();
                let i32_size = std::mem::size_of::<i32>();
                data.groups
                    .iter()
                    .map(|g| {
                        g.powered_verts.len() * f32_size
                            + g.powered_attrib.len() * f32_size
                            + g.travel_verts.len() * f32_size
                            + g.zero_power_verts.len() * f32_size
                            + g.powered_cmd_offsets.len() * i32_size
                            + g.travel_cmd_offsets.len() * i32_size
                            + g.overlay_positions.len() * f32_size
                            + g.overlay_attrib.len() * f32_size
                            + g.overlay_cmd_offsets.len() * i32_size
                    })
                    .sum()
            }
        }
    }
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

/// Version of the cross-crate encoder handoff.
///
/// Bumped whenever the [`Encoder`] trait or this handoff contract
/// changes incompatibly.
pub const ENCODER_ABI: &str = "1";

/// The PyCapsule name used by the cross-crate encoder handoff.
///
/// Encoders implemented outside this crate expose a
/// ``__raygeo_encoder__()`` method returning a capsule with this name.
/// The name embeds the `raygeo` version the encoder was built against,
/// so a mismatched pairing fails with a clear error instead of being
/// misinterpreted.
pub fn encoder_capsule_name() -> String {
    format!(
        "raygeo.encoder.{}.{}",
        env!("CARGO_PKG_VERSION"),
        ENCODER_ABI
    )
}

/// Object-safe clone support for [`Encoder`].
///
/// Implemented automatically for every `Clone` encoder, so encoder
/// specs need no extra code.
pub trait EncoderClone {
    /// Clone this encoder into a new owned trait object.
    fn clone_box(&self) -> Box<dyn Encoder>;
}

impl<T> EncoderClone for T
where
    T: 'static + Encoder + Clone,
{
    fn clone_box(&self) -> Box<dyn Encoder> {
        Box::new(self.clone())
    }
}

/// Per-call context handed to [`Encoder::encode`].
///
/// Bundles the immutable [`Ops`] being encoded with the caller's
/// [`Callbacks`] so encoders can report progress and poll for
/// cancellation without depending on a separate progress trait.
pub struct EncodeCtx<'a> {
    /// The ops being encoded.
    pub ops: &'a Ops,
    /// The caller's callback bundle.
    pub callbacks: &'a dyn Callbacks,
}

/// A typed encoder spec.
///
/// Each encoder under this module (e.g. [`vertex_arrays::VertexSpec`],
/// [`texture::TextureSpec`]) implements
/// this trait so callers can hold a `Box<dyn Encoder>` without
/// knowing concrete types. Adding a new encoder is purely additive:
/// define a spec struct, implement [`Encoder`], and pass an instance
/// to the caller.
///
/// `Send + Sync` is required so a `Box<dyn Encoder>` can be held
/// across thread boundaries by the caller.
pub trait Encoder: Send + Sync + EncoderClone {
    /// Run the encoder against the supplied [`EncodeCtx`].
    ///
    /// On success, returns the encoder's [`EncodeOutput`]. On
    /// failure, returns a human-readable error string (the string
    /// `"cancelled"` is the conventional cancellation signal).
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String>;

    /// Short, human-readable name used in progress messages.
    fn name(&self) -> &str;
}
