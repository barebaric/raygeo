//! Per-task callback interface shared across the `ops` layer.
//!
//! [`Callbacks`] is the trait used by assembler implementations
//! (under [`crate::ops::assembly`]) and transformer implementations
//! (under [`crate::ops::transform`]) to report progress, poll for
//! cancellation, and emit progressive chunks back to the caller. The
//! trait is object-safe so a `Box<dyn Callbacks>` can be held
//! without the ops layer depending on pyo3.
//!
//! This module sits at the root of `ops` because both `assembly` and
//! `transform` consume it; neither owns it. The Python mirror
//! (`PyTaskCallbacks` in [`crate::python::ops::callbacks`]) implements
//! this trait by reacquiring the GIL and calling the stored
//! `Py<PyAny>` callables.

/// Progressive paint chunk emitted by raster assemblers.
///
/// Raster assemblers that produce partial Ops as slabs complete push
/// a `ChunkPayload` via [`Callbacks::emit_chunk`]. The payload
/// carries the slab's row range and the partial Ops; the caller
/// paints it onto the canvas without waiting for the full stage to
/// finish.
#[derive(Debug, Clone)]
pub struct ChunkPayload {
    /// First row of this slab in the source image's pixel space.
    pub y_start: u32,
    /// One-past-last row of this slab in the source image's pixel
    /// space.
    pub y_end: u32,
    /// Human-readable status message (e.g. "slab 12/30").
    pub message: String,
}

/// Callback bundle handed to long-running `ops` work.
///
/// Used by assemblers ([`crate::ops::assembly::Assembler`]) and
/// transformers ([`crate::ops::transform::Transformer`]) so every
/// long-running unit of work shares one progress / cancellation /
/// chunk shape.
///
/// # Cancellation model
///
/// Cancellation is **cooperative**: long-running implementations poll
/// [`is_cancelled`](Callbacks::is_cancelled) between meaningful units
/// of work and return early (typically with an error) when it returns
/// `true`.
///
/// rayon tasks **cannot be force-aborted**. The pipeline executor
/// (`pipeline::execute`) runs each node on a rayon worker; once a
/// node's `run()` has started, it runs to completion or until it
/// voluntarily polls `is_cancelled()`. There is no `JoinHandle::abort`
/// or thread-kill mechanism.
///
/// **Supersession** (a node invalidated while still computing) is
/// handled separately by the per-node epoch counter on the pipeline
/// cache: the in-flight task is allowed to finish, but its result is
/// discarded at completion if the epoch has advanced. See
/// `pipeline::execute::spawn_one`.
///
/// Long-running implementations should poll at a granularity that
/// keeps cancellation latency under a few hundred milliseconds —
/// typically every N loop iterations, where N is chosen so that N
/// iterations of the inner loop complete in well under one second.
pub trait Callbacks: Send {
    /// Report progress in `[0.0, 1.0]` with a short message.
    ///
    /// Implementors decide whether to reacquire the GIL — the ops
    /// layer never holds it.
    fn report_progress(&self, frac: f64, msg: &str);

    /// Poll for cancellation. Callers invoke this between meaningful
    /// units of work (per slab, per contour, per transformer).
    /// Returns `true` when the caller has been invalidated and the
    /// work should return early.
    fn is_cancelled(&self) -> bool;

    /// Emit a progressive paint chunk. Only raster assemblers emit
    /// chunks today; vector assemblers and transformers leave this
    /// unimplemented (the default impl is a no-op).
    fn emit_chunk(&self, _chunk: ChunkPayload) {}
}

/// A no-op `Callbacks` for tests and code paths that don't need
/// callbacks.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoCallbacks;

impl Callbacks for NoCallbacks {
    fn report_progress(&self, _frac: f64, _msg: &str) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}
