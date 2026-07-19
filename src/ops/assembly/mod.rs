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
//! implements [`Assembler`]. The pipeline's `Compute` stage drives any
//! assembler through this trait, mirroring how `ops::transform` drives
//! post-processors through the `Transformer` trait.

pub mod adaptive;
pub mod callbacks;
pub mod contour;
pub mod helix;
pub mod material_test_grid;
pub mod profile;
pub mod ramp;
pub(crate) mod result;
pub use result::AssemblyMeta;
pub mod slot;
pub mod spiral;
pub mod toroid;
pub(crate) mod trace_utils;
pub mod tracelet;
pub mod wavefront;

pub use callbacks::{ChunkPayload, NoCallbacks, TaskCallbacks};
pub use tracelet::{write_polyline, ProgressEvent, Tracelet};

use crate::ops::part::Part;
use crate::ops::state::State;

/// Context passed to [`Assembler::assemble`].
///
/// Bundles the mutable `Part`, the `Tracelet` that accumulates the
/// produced ops and drives the progress callback, the cut `State`
/// (feed rate / power), and the
/// [`TaskCallbacks`](crate::ops::assembly::callbacks::TaskCallbacks)
/// for progress reports and cancellation. Machine capability flags
/// are intentionally NOT here — each assembler carries its own
/// arc/curve parameters in its spec, and rayforge is responsible for
/// resolving those before constructing the spec.
pub struct AssembleCtx<'a> {
    /// The part being assembled. The assembler may mutate `cleared`
    /// and `stock_region` as it works.
    pub part: &'a mut Part,
    /// Tracelet accumulating the produced `Ops`; also drives the
    /// per-node progress callback.
    pub trace: &'a mut Tracelet,
    /// Cut-state (feed rate, power) for the assembler's cutting moves.
    pub state: &'a State,
    /// Per-node callbacks (progress, cancellation, chunks).
    pub callbacks: &'a dyn TaskCallbacks,
}

/// A typed assembler spec that the pipeline's `Compute` stage can
/// dispatch through.
///
/// Each assembler under this module (e.g. [`contour::ContourSpec`],
/// [`adaptive::AdaptiveClearingSpec`], [`spiral::SpiralSpec`], ...)
/// implements this trait so the `Compute` stage driver needs no
/// knowledge of concrete assembler types. Adding a new assembler is
/// purely additive: define a spec struct, implement [`Assembler`],
/// and the pipeline picks it up via [`Box<dyn Assembler>`].
///
/// `Send + Sync` is required so that `Box<dyn Assembler>` can be
/// moved across rayon worker threads by the pipeline executor.
pub trait Assembler: Send + Sync {
    /// Run the assembler against the supplied [`AssembleCtx`].
    ///
    /// On success, returns the [`AssemblyMeta`] (start/end tool
    /// poses). The produced `Ops` are accumulated in
    /// [`AssembleCtx::trace`] (the tracelet's `into_ops` is called
    /// by the driver). On failure, returns a human-readable error
    /// string (the string `"cancelled"` is treated as cancellation
    /// by the driver).
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String>;

    /// Short, human-readable name used in progress messages.
    fn name(&self) -> &'static str;
}
