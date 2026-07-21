/// Callback bundle for pipeline-level stage execution.
///
/// # Cancellation model
///
/// Cancellation is **cooperative**: stage implementations poll
/// [`is_cancelled`](Callbacks::is_cancelled) between meaningful units
/// of work and return `Err("cancelled")` when it returns `true`.
///
/// rayon tasks **cannot be force-aborted**. Once a stage's `run()`
/// starts on a rayon worker, it runs to completion or until it
/// voluntarily polls `is_cancelled()`.
///
/// **Supersession** (a node invalidated while still computing) is
/// handled by the per-node epoch counter on the pipeline cache: the
/// in-flight task finishes, but its result is discarded at completion
/// if the epoch has advanced (see `execute::spawn_one`).
pub trait Callbacks: Send {
    fn report_progress(&self, frac: f64, msg: &str);
    fn is_cancelled(&self) -> bool;
    fn emit_chunk(&self, _chunk: Box<dyn Any + Send + Sync>) {}
}

use std::any::Any;

#[derive(Debug, Default, Clone, Copy)]
pub struct NoCallbacks;

impl Callbacks for NoCallbacks {
    fn report_progress(&self, _frac: f64, _msg: &str) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}
