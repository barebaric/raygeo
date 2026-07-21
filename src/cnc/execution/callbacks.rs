use crate::ops::callbacks::{Callbacks as OpsCallbacks, ChunkPayload};
use crate::pipeline::callbacks::Callbacks as PipelineCallbacks;

pub(crate) struct OpsCallbacksAdapter<'a> {
    pub inner: &'a dyn PipelineCallbacks,
}

impl<'a> OpsCallbacks for OpsCallbacksAdapter<'a> {
    fn report_progress(&self, frac: f64, msg: &str) {
        self.inner.report_progress(frac, msg);
    }

    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    fn emit_chunk(&self, chunk: ChunkPayload) {
        self.inner.emit_chunk(Box::new(chunk));
    }
}

pub(crate) struct ScaledCallbacks<'a> {
    inner: &'a dyn OpsCallbacks,
    base: f64,
    span: f64,
}

impl<'a> ScaledCallbacks<'a> {
    pub(crate) fn new(
        inner: &'a dyn OpsCallbacks,
        base: f64,
        span: f64,
    ) -> Self {
        ScaledCallbacks { inner, base, span }
    }
}

impl<'a> OpsCallbacks for ScaledCallbacks<'a> {
    fn report_progress(&self, frac: f64, msg: &str) {
        let scaled = self.base + self.span * frac;
        self.inner.report_progress(scaled, msg);
    }

    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    fn emit_chunk(&self, chunk: ChunkPayload) {
        self.inner.emit_chunk(chunk);
    }
}

unsafe impl<'a> Send for ScaledCallbacks<'a> {}
unsafe impl<'a> Send for OpsCallbacksAdapter<'a> {}
