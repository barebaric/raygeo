//! Per-node callback interface for assemblers.
//!
//! [`TaskCallbacks`] is the trait assembler implementations (and
//! other long-running ops-layer work) use to report progress, poll
//! for cancellation, and emit progressive chunks back to the caller.
//! The trait is object-safe so a `Box<dyn TaskCallbacks>` can be
//! held without the ops layer depending on pyo3.
//!
//! The Python mirror (`PyTaskCallbacks` in
//! `crate::python::pipeline::callbacks`) implements this trait by
//! reacquiring the GIL and calling the stored `Py<PyAny>` callables.

/// Progressive paint chunk emitted by raster assemblers.
///
/// Raster assemblers that produce partial Ops as slabs complete push
/// a `ChunkPayload` via [`TaskCallbacks::emit_chunk`]. The payload
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

/// Per-node callback bundle.
///
/// `Send + Sync` so the trait object can be held by a
/// `Box<dyn TaskCallbacks>` that crosses rayon worker threads.
pub trait TaskCallbacks: Send + Sync {
    /// Report per-node progress in `[0.0, 1.0]` with a short message.
    ///
    /// Assemblers call this during long compute (per-slab for raster,
    /// per-phase for contour). The implementor decides whether to
    /// reacquire the GIL — the ops layer never holds it.
    fn report_progress(&self, frac: f64, msg: &str);

    /// Poll for cancellation. Assemblers call this between
    /// meaningful units of work (per slab, per contour, per
    /// transformer). Returns `true` when the caller has invalidated
    /// the node and the assembler should return early.
    fn is_cancelled(&self) -> bool;

    /// Emit a progressive paint chunk. Only raster assemblers emit
    /// chunks today; vector assemblers leave this unimplemented (the
    /// default impl is a no-op).
    fn emit_chunk(&self, _chunk: ChunkPayload) {}
}

/// A no-op `TaskCallbacks` for tests and assemblers that don't need
/// callbacks.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoCallbacks;

impl TaskCallbacks for NoCallbacks {
    fn report_progress(&self, _frac: f64, _msg: &str) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}
