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

pub use tracelet::{ProgressEvent, Tracelet};

use std::any::Any;

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
/// the caller is responsible for resolving those before constructing
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
    /// Id of the face currently being assembled. `""` is the default
    /// face, `"1"`, `"2"`, ... are additional faces detected by
    /// `Part::from_geometry_multi_face`. Assemblers copy it into the
    /// [`AssemblyWarning::face_id`] of any region/face-level warning
    /// they emit; the Compute stage sets it from the active face id
    /// before invoking [`Assembler::assemble`].
    pub face_id: String,
    /// When set, this call is scoped to a single sub-region of the face
    /// (the plan-time path: one `assemble()` call per region).  The
    /// face's `stock_region.boundary` has already been replaced with
    /// this region boundary by the Compute stage, so the assembler must
    /// clear just that region.  When `None`, the assembler operates on
    /// the whole face and may split it into regions itself as a
    /// runtime fallback.
    pub region_boundary: Option<(Polygon, Vec<Polygon>)>,
    /// Non-fatal warnings accumulated during this assembly. Assemblers push
    /// [`AssemblyWarning`]s here instead of aborting; the Compute stage moves
    /// the vec into [`AssemblyOutput::warnings`] after `assemble` returns.
    pub warnings: &'a mut Vec<AssemblyWarning>,
}

/// The kind of a non-fatal [`AssemblyWarning`] produced during assembly.
///
/// Warnings are typed so the consumer can translate and surface them
/// to the user. The raw, non-translatable detail string lives in
/// [`AssemblyWarning::detail`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssemblyWarningKind {
    /// A whole face's assembly failed; processing continued with other faces.
    FaceFailed,
    /// A single region within a face failed; other regions still cleared.
    RegionFailed,
}

/// A non-fatal warning emitted by an assembler.
///
/// Assemblers push these into [`AssembleCtx::warnings`] during assembly; the
/// Compute stage collects them into [`AssemblyOutput::warnings`] so the
/// consumer can translate and surface them. Unlike an error, a warning does
/// not abort the pipeline — the affected face/region is skipped and the
/// rest of the part is still machined.
#[derive(Clone, Debug)]
pub struct AssemblyWarning {
    /// What failed, determining the translation template.
    pub kind: AssemblyWarningKind,
    /// Face id; `""` is the default face, `"1"`, `"2"`, ... others.
    pub face_id: String,
    /// Region index within the face; `None` for whole-face failures.
    pub region: Option<usize>,
    /// Raw, non-translatable diagnostic (the assembler's error string).
    pub detail: String,
}

/// The output of an assembler, packaged for caching.
///
/// Carries the assembled `Ops`, metadata, and optional post-assembly
/// cleared fragments for face-state restoration on cache hit, plus any
/// non-fatal [`AssemblyWarning`]s encountered along the way.
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
    /// Start/end tool poses returned by the assembler.
    pub meta: AssemblyMeta,
    /// Non-fatal warnings emitted while assembling this output. The
    /// Compute stage fills this from [`AssembleCtx::warnings`]; cache
    /// store/restore preserve it.
    pub warnings: Vec<AssemblyWarning>,
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
    fn name(&self) -> &str;

    /// Reconstruct the cached output from a stored entry.
    ///
    /// Defaults to returning a clone of the cached output. Override
    /// for assembler-specific cache reconstruction (e.g. adaptive
    /// clearing restores cleared fragments into the face).
    fn restore_cache(&self, cached: &AssemblyOutput) -> Option<AssemblyOutput> {
        Some(cached.clone())
    }

    /// Prepare the just-computed output for cache storage.
    ///
    /// Defaults to returning a clone of the output. Override for
    /// assembler-specific storage preparation (e.g. adaptive clearing
    /// injects cleared fragments into the stored copy).
    fn store_cache(&self, output: &AssemblyOutput) -> Option<AssemblyOutput> {
        Some(output.clone())
    }

    /// Clone the assembler spec into a new boxed trait object.
    ///
    /// Required so that `Arc<dyn Assembler>` (stored in `PlanStep`) can
    /// produce independent `Box<dyn Assembler>` instances for each
    /// `NodeRequest` in `cnc::execution::create_intent`.
    fn boxed_clone(&self) -> Box<dyn Assembler>;

    /// Downcast to `&dyn Any` for concrete-type inspection.
    ///
    /// Needed by `cnc::plan` Python bindings to read spec parameters
    /// from a `Box<dyn Assembler>` trait object.
    fn as_any(&self) -> &dyn Any;
}
