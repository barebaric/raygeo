//! Motion-path assembly: turning raw geometry primitives into Ops.
//!
//! Functions in this module compose geo-layer primitives (polylines,
//! arcs, polygons) into complete motion sequences represented as
//! [`crate::ops::Ops`] objects. They decide traversal order, linking
//! strategy, lead-in/out, overscan, and tab insertion — concerns that
//! belong to motion assembly rather than pure geometry.
//!
//! ## The `Assembler` trait
//!
//! Each assembler under this module exposes a **spec struct** (e.g.
//! [`contour::ContourSpec`], [`adaptive::AdaptiveClearingSpec`]) that
//! implements [`Assembler`]. Callers drive any assembler through this
//! trait, mirroring how `ops::transform` drives post-processors
//! through the `Transformer` trait. Both traits take their
//! callbacks from [`crate::ops::callbacks`].

pub mod adaptive;
pub mod contour;
pub mod frame;
pub mod helix;
pub mod material_test_grid;
pub mod profile;
pub mod ramp;
pub mod raster;
pub(crate) mod result;
pub use result::AssemblyMeta;
pub mod shrinkwrap;
pub mod slot;
pub mod spiral;
pub mod toroid;
pub(crate) mod trace_utils;
pub mod tracelet;
pub mod wavefront;

pub use tracelet::{write_polyline, ProgressEvent, Tracelet};

use crate::ops::callbacks::Callbacks;
use crate::ops::container::Ops;
use crate::ops::part::FaceState;
use crate::ops::part::ImageSource;
use crate::ops::state::State;
use crate::types::Polygon;

/// Context passed to [`Assembler::assemble`].
///
/// Bundles the mutable `Part`, the target [`FaceState`] the assembler
/// operates on, the `Tracelet` that accumulates the produced ops and
/// drives the progress callback, the cut `State` (feed rate / power),
/// and the [`Callbacks`] for progress reports and cancellation.
/// Machine capability flags are intentionally NOT here — each
/// assembler carries its own arc/curve parameters in its spec, and
/// rayforge is responsible for resolving those before constructing
/// the spec.
pub struct AssembleCtx<'a> {
    /// The target face's state — geometry, stock region, and cleared
    /// area.  This is what assemblers mutate and read for machining.
    pub face: &'a mut FaceState,
    /// Tracelet accumulating the produced `Ops`; also drives the
    /// progress callback.
    pub trace: &'a mut Tracelet,
    /// Cut-state (feed rate, power) for the assembler's cutting moves.
    pub state: &'a State,
    /// Callbacks (progress, cancellation, chunks).
    pub callbacks: &'a dyn Callbacks,
    /// Physical size of the part in millimetres `(width, height)`.
    /// Needed by raster / shrinkwrap / frame assemblers that scale
    /// pixel coordinates into mm space.
    pub size_mm: (f64, f64),
    /// Pixel density `(x, y)` in pixels per millimetre. `None` for
    /// purely vector work; required by raster assemblers.
    pub pixels_per_mm: Option<(f64, f64)>,
    /// Lazy source of pixel data for raster / shrinkwrap assemblers.
    /// `None` for vector-only assemblers (contour, frame, ...). Set
    /// by the Compute stage from `Part.image_source` before calling
    /// [`Assembler::assemble`].
    pub image_source: Option<&'a dyn ImageSource>,
}

/// The output of an assembler, packaged for caching.
///
/// Carries the assembled `Ops`, metadata, and optional post-assembly
/// cleared fragments for face-state restoration on cache hit.
#[derive(Debug, Clone)]
pub struct AssemblyOutput {
    /// The assembled `Ops` (with transformers already applied).
    pub ops: Ops,
    /// Whether the `Ops` may be uniformly scaled during aggregation.
    pub is_scalable: bool,
    /// Source `(width_mm, height_mm)` of the part that produced `ops`.
    pub source_dimensions: Option<(f64, f64)>,
    /// Post-assembly cleared fragments to restore into
    /// `FaceState.cleared`. `None` for assemblers that don't touch
    /// `cleared`.
    pub cleared_fragments: Option<Vec<Polygon>>,
}

impl AssemblyOutput {
    /// Produce the core output triple `(ops, is_scalable,
    /// source_dimensions)` for the consumer to build its own
    /// output type without an upward dependency on [`AssemblyOutput`].
    pub fn into_parts(self) -> (Ops, bool, Option<(f64, f64)>) {
        (self.ops, self.is_scalable, self.source_dimensions)
    }
}

/// A typed assembler spec.
///
/// Each assembler under this module (e.g. [`contour::ContourSpec`],
/// [`adaptive::AdaptiveClearingSpec`], [`spiral::SpiralSpec`], ...)
/// implements this trait so callers can hold a collection of
/// `Box<dyn Assembler>` without knowing concrete types. Adding a new
/// assembler is purely additive: define a spec struct, implement
/// [`Assembler`] + [`Cacheable<AssemblyOutput>`], and pass an
/// instance to the caller.
///
/// `Send + Sync` is required so a `Box<dyn Assembler>` can be held
/// across thread boundaries by the caller.
pub trait Assembler: Send + Sync {
    /// Run the assembler against the supplied [`AssembleCtx`].
    ///
    /// On success, returns the [`AssemblyMeta`] (start/end tool
    /// poses). The produced `Ops` are accumulated in
    /// [`AssembleCtx::trace`] (the caller drains the tracelet via
    /// `into_ops`). On failure, returns a human-readable error string
    /// (the string `"cancelled"` is the conventional cancellation
    /// signal).
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String>;

    /// Whether the produced `Ops` may be uniformly scaled during
    /// aggregation. Vector assemblers (the default) return `true`;
    /// raster / shrinkwrap assemblers return `false` because their
    /// scanline spacing is physical, not graphical.
    fn is_scalable(&self) -> bool {
        true
    }

    /// Short, human-readable name used in progress messages.
    fn name(&self) -> &'static str;

    /// Compute a cache-key payload hash from face state. The glue
    /// module wraps this hash with the caller's tag to form a
    /// `CacheKey`. Returning `None` opts out of caching.
    fn cache_key_for_face(&self, _face: &FaceState) -> Option<u64> {
        None
    }

    /// Reconstruct the cached output from a stored entry.
    fn restore_cache(
        &self,
        _cached: &AssemblyOutput,
    ) -> Option<AssemblyOutput> {
        None
    }

    /// Store the just-computed output for future cache hits.
    fn store_cache(&self, _output: &AssemblyOutput) -> Option<AssemblyOutput> {
        None
    }
}
