//! Transform: operations that take existing Ops and return modified Ops.
//!
//! Modules in this layer consume [`Ops`](crate::ops::Ops) sequences and
//! produce new or mutated sequences — travel optimization, flipping,
//! pass linking, lead-in/out, overscan, tabs, linearization, merging,
//! grouping, and clipping.
//!
//! ## Trait shape
//!
//! Each transformer is a typed spec struct that implements
//! [`Transformer`]. [`apply_transformers`] sorts specs by [`Phase`],
//! polls cancellation, reports progress, and invokes
//! [`Transformer::apply`] with a fresh [`TransformCtx`] that bundles
//! the target [`Ops`] with a [`TaskCallbacks`]. Adding a transformer
//! is purely additive: define a spec struct, implement
//! [`Transformer`], and pass it to [`apply_transformers`].
//!
//! ## Layering
//!
//! This module is part of the `ops` layer: it knows about `Ops` and
//! the [`TaskCallbacks`] shape (defined in
//! [`crate::ops::callbacks`]) but nothing about Python. The
//! Python bindings live in [`crate::python::ops::transform`] and only
//! translate pyclass instances into spec structs that implement
//! [`Transformer`].

use crate::ops::callbacks::Callbacks;
use crate::ops::Ops;

pub mod affine;
pub mod bidir_scan_offset;
pub mod clip;
pub mod flip;
pub mod frame;
pub mod group;
pub mod layer;
pub mod lead_in_out;
pub mod linearize;
pub mod link;
pub mod merge_lines;
pub mod multipass;
pub mod optimize;
pub mod overscan;
pub mod smooth;
pub mod split;
pub mod tabs;

pub use bidir_scan_offset::apply_bidir_scan_offset;
pub use flip::flip_ops;
pub use group::{group_by_auxiliary_state, without_state};
pub use lead_in_out::apply_lead_in_out;
pub use link::{link_passes, LinkStrategy};
pub use merge_lines::merge_overlapping_lines;
pub use multipass::apply_multipass;
pub use optimize::optimize_travel;
pub use overscan::apply_overscan;
pub use tabs::{apply_tab_gaps, apply_tab_power, ClipPoint};

/// Execution phase of a transformer.
///
/// Phases are applied in this order: [`Phase::GeometryRefinement`]
/// first, then [`Phase::PathInterruption`], then
/// [`Phase::PostProcessing`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// Modify continuous paths (e.g. smooth, optimize, merge lines).
    GeometryRefinement,
    /// Create gaps in paths (e.g. lead-in/out, crop, tabs).
    PathInterruption,
    /// Operate on final paths (e.g. overscan, multipass, bidir offset).
    PostProcessing,
}

/// Per-call context handed to [`Transformer::apply`].
///
/// Bundles the mutable [`Ops`] being modified with the caller's
/// [`Callbacks`] so transformers can report progress, poll for
/// cancellation, and (for raster sources) emit progressive chunks
/// without depending on a separate progress trait.
pub struct TransformCtx<'a> {
    /// The ops being modified in place.
    pub ops: &'a mut Ops,
    /// The caller's callback bundle.
    pub callbacks: &'a dyn Callbacks,
}

/// A typed transformer spec driven by [`apply_transformers`].
///
/// Each spec struct implements this trait so [`apply_transformers`]
/// needs no knowledge of concrete types. `Send + Sync` is required
/// so a `Box<dyn Transformer>` can be held across thread boundaries
/// by the caller.
pub trait Transformer: Send + Sync {
    /// The execution phase this transformer belongs to.
    fn phase(&self) -> Phase;

    /// Apply the transformer to `ctx.ops` in place. Implementations
    /// may poll `ctx.callbacks.is_cancelled()` between meaningful
    /// units of work and report progress through
    /// [`Callbacks::report_progress`].
    fn apply(&self, ctx: &mut TransformCtx<'_>);

    /// Short, human-readable name used in progress messages.
    fn name(&self) -> &'static str;

    /// Hash the parameters that affect this transformer's output.
    ///
    /// The returned hash is folded into the owning node's cache key
    /// so that changing a transformer (or any of its parameters)
    /// invalidates the cache independently of the assembler's hash.
    /// Returning `0` is allowed for transformers whose effect is
    /// already captured by the upstream cache key, but the default
    /// is to hash the transformer's `name()` so that adding any
    /// transformer (even with default params) changes the cache key.
    fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name().hash(&mut h);
        h.finish()
    }
}

/// Combine an assembler hash with a list of transformer hashes into
/// a single cache-key payload hash.
///
/// The two components are kept separate so that changing either
/// independently invalidates the cache. `assembler_hash` may be `0`
/// (when the assembler opts out of caching); `transformer_hashes`
/// may be empty (when no transformers are attached).
pub fn combine_cache_hashes(
    assembler_hash: u64,
    transformer_hashes: &[u64],
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    assembler_hash.hash(&mut h);
    transformer_hashes.hash(&mut h);
    h.finish()
}

/// Apply a sequence of transformer specs to an `Ops` in phase order.
///
/// The specs are sorted by [`Phase`] and dispatched via [`Transformer`].
/// Before every transformer `callbacks.is_cancelled()` is polled; if
/// it returns `true`, the loop aborts with `Err(Cancelled)`. Then
/// `callbacks.report_progress(i / total, spec.name())` is called so
/// the caller's overall progress tracks the batch.
///
/// Takes `specs` by mutable reference so the caller can retain
/// ownership (e.g. a borrowed `AggregateSpec` whose `transformers`
/// field is `Vec<Box<dyn Transformer>>`). The sort happens in place.
pub fn apply_transformers(
    ops: &mut Ops,
    specs: &mut [Box<dyn Transformer>],
    callbacks: &dyn Callbacks,
) -> Result<(), Cancelled> {
    specs.sort_by_key(|s| s.phase());

    let total = specs.len();
    for (i, spec) in specs.iter().enumerate() {
        if callbacks.is_cancelled() {
            return Err(Cancelled);
        }
        let p = if total == 0 {
            0.0
        } else {
            i as f64 / total as f64
        };
        callbacks.report_progress(p, spec.name());
        let mut ctx = TransformCtx {
            ops: &mut *ops,
            callbacks,
        };
        spec.apply(&mut ctx);
    }
    Ok(())
}

/// Error returned by [`apply_transformers`] when cancellation was
/// requested before a transformer.
#[derive(Debug)]
pub struct Cancelled;
