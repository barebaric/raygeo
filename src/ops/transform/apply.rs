//! Batch dispatch of typed transformer specs.
//!
//! The individual spec structs live alongside their transform functions
//! (e.g. [`crate::ops::transform::smooth::SmoothSpec`]) and implement
//! the [`Transformer`] trait so that [`apply_transformers`] can drive
//! them without knowing their concrete types.
//!
//! This module is part of the `ops` layer: it knows about `Ops` but
//! nothing about Python. The Python bindings live in
//! `crate::python::ops::transform::apply` and only translate pyclass
//! instances into spec structs that implement [`Transformer`].

use crate::ops::Ops;

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

/// A typed transformer spec that can be dispatched by
/// [`apply_transformers`].
///
/// Each spec struct implements this trait so the dispatch driver needs
/// no knowledge of concrete types. Adding a new transformer is purely
/// additive: define a spec struct, implement [`Transformer`], and the
/// driver picks it up.
///
/// `Send + Sync` is required so that `Box<dyn Transformer>` can be
/// moved across rayon worker threads by the pipeline executor.
pub trait Transformer: Send + Sync {
    /// The execution phase this transformer belongs to.
    fn phase(&self) -> Phase;

    /// Apply the transformer to `ops` in place.
    fn apply(&self, ops: &mut Ops);

    /// Short, human-readable name used in progress messages.
    fn name(&self) -> &'static str;
}

/// Apply a sequence of transformer specs to an `Ops` in phase order.
///
/// The specs are sorted by [`Phase`] and dispatched via [`Transformer`].
/// Before every transformer the optional `progress` callback is
/// consulted: its `is_cancelled()` method, if it returns `true`, aborts
/// the loop with `Err(Cancelled)`; then `report(progress, message)` is
/// called with `progress = i / total` and the transformer's name.
///
/// Takes `specs` by mutable reference so the caller can retain
/// ownership (e.g. a borrowed `AggregateSpec` whose `transformers`
/// field is `Vec<Box<dyn Transformer>>`). The sort happens in place.
pub fn apply_transformers(
    ops: &mut Ops,
    specs: &mut [Box<dyn Transformer>],
    progress: Option<&dyn Progress>,
) -> Result<(), Cancelled> {
    specs.sort_by_key(|s| s.phase());

    let total = specs.len();
    for (i, spec) in specs.iter().enumerate() {
        if let Some(cb) = progress {
            if cb.is_cancelled() {
                return Err(Cancelled);
            }
            let p = if total == 0 {
                0.0
            } else {
                i as f64 / total as f64
            };
            cb.report(p, spec.name());
        }
        spec.apply(ops);
    }
    Ok(())
}

/// Progress callback used by [`apply_transformers`].
///
/// `report` is called before each transformer; `is_cancelled` is polled
/// before `report`. The implementor decides whether to release the GIL
/// between iterations.
pub trait Progress {
    /// Called with `progress` in `[0, 1)` and a short message.
    fn report(&self, progress: f64, message: &str);

    /// Polled before each transformer; `true` aborts the loop.
    fn is_cancelled(&self) -> bool;
}

/// Error returned by [`apply_transformers`] when cancellation was
/// requested before a transformer.
#[derive(Debug)]
pub struct Cancelled;
