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
