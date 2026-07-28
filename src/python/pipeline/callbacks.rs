use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use pyo3::prelude::*;

use crate::ops::callbacks::ChunkPayload;
use crate::pipeline::callbacks::Callbacks;

pub struct PyTaskCallbacks {
    on_progress: Option<Py<PyAny>>,
    on_cancelled: Option<Py<PyAny>>,
    on_chunk: Option<Py<PyAny>>,
    cancel_flag: Arc<AtomicBool>,
}

impl PyTaskCallbacks {
    pub fn new(
        on_progress: Option<Py<PyAny>>,
        on_cancelled: Option<Py<PyAny>>,
        on_chunk: Option<Py<PyAny>>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Self {
        PyTaskCallbacks {
            on_progress,
            on_cancelled,
            on_chunk,
            cancel_flag,
        }
    }
}

impl Callbacks for PyTaskCallbacks {
    fn report_progress(&self, frac: f64, msg: &str) {
        if let Some(ref cb) = self.on_progress {
            Python::attach(|py| {
                let _ = cb.call1(py, (frac, msg));
            });
        }
    }

    fn is_cancelled(&self) -> bool {
        // No GIL, no Python callbacks from rayon workers.
        if self.cancel_flag.load(Ordering::SeqCst) {
            return true;
        }
        self.on_cancelled.as_ref().is_some_and(|cb| {
            Python::attach(|py| {
                cb.call0(py)
                    .and_then(|v| v.extract::<bool>(py))
                    .unwrap_or(false)
            })
        })
    }

    fn emit_chunk(&self, chunk: Box<dyn Any + Send + Sync>) {
        if let Some(ref cb) = self.on_chunk {
            if let Some(payload) = chunk.downcast_ref::<ChunkPayload>() {
                Python::attach(|py| {
                    let _ = cb.call1(
                        py,
                        (payload.y_start, payload.y_end, &payload.message),
                    );
                });
            }
        }
    }
}
