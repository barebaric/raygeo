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
//! [`gcode::GcodeSpec`], [`vertex_arrays::VertexSpec`],
//! [`texture::TextureSpec`], [`view::ViewSpec`]) that implements
//! [`Encoder`]. Callers drive any encoder through this trait, mirroring
//! how `ops::assembly` drives assemblers through `Assembler` and
//! `ops::transform` drives transformers through `Transformer`. All
//! three traits take their callbacks from [`crate::ops::callbacks`].

use crate::ops::container::Ops;
use crate::ops::convert::vertex_arrays::VertexArrays;

pub mod dump;
pub mod gcode;
pub mod gcode_types;
pub mod geometry;
pub mod image;
pub mod polyline;
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
        /// Op index → list of emitted line indices.
        op_to_machine_code: std::collections::HashMap<usize, Vec<usize>>,
        /// Emitted line index → op index.
        machine_code_to_op: std::collections::HashMap<usize, usize>,
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
}

impl EncodeOutput {
    /// Estimated heap-allocated bytes for this output.
    pub fn heap_size(&self) -> usize {
        match self {
            EncodeOutput::MachineCode {
                text,
                op_to_machine_code,
                machine_code_to_op,
            } => {
                let inner_vecs: usize = op_to_machine_code
                    .values()
                    .map(|v| v.len() * std::mem::size_of::<usize>())
                    .sum();
                text.len()
                    + inner_vecs
                    + machine_code_to_op.len()
                        * std::mem::size_of::<(usize, usize)>()
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
        }
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
/// Each encoder under this module (e.g. [`gcode::GcodeSpec`],
/// [`vertex_arrays::VertexSpec`], [`texture::TextureSpec`]) implements
/// this trait so callers can hold a `Box<dyn Encoder>` without
/// knowing concrete types. Adding a new encoder is purely additive:
/// define a spec struct, implement [`Encoder`], and pass an instance
/// to the caller.
///
/// `Send + Sync` is required so a `Box<dyn Encoder>` can be held
/// across thread boundaries by the caller.
pub trait Encoder: Send + Sync {
    /// Run the encoder against the supplied [`EncodeCtx`].
    ///
    /// On success, returns the encoder's [`EncodeOutput`]. On
    /// failure, returns a human-readable error string (the string
    /// `"cancelled"` is the conventional cancellation signal).
    fn encode(&self, ctx: &mut EncodeCtx<'_>) -> Result<EncodeOutput, String>;

    /// Short, human-readable name used in progress messages.
    fn name(&self) -> &str;
}
