use std::any::Any;

use pyo3::prelude::*;

use crate::pipeline::callbacks::Callbacks;

pub struct PyTaskCallbacks {
    on_progress: Option<Py<PyAny>>,
    on_cancelled: Option<Py<PyAny>>,
    on_chunk: Option<Py<PyAny>>,
}

impl PyTaskCallbacks {
    pub fn new(
        on_progress: Option<Py<PyAny>>,
        on_cancelled: Option<Py<PyAny>>,
        on_chunk: Option<Py<PyAny>>,
    ) -> Self {
        PyTaskCallbacks {
            on_progress,
            on_cancelled,
            on_chunk,
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
        self.on_cancelled.as_ref().is_some_and(|cb| {
            Python::attach(|py| {
                cb.call0(py)
                    .and_then(|v| v.extract::<bool>(py))
                    .unwrap_or(false)
            })
        })
    }

    fn emit_chunk(&self, _chunk: Box<dyn Any + Send + Sync>) {
        if let Some(ref cb) = self.on_chunk {
            Python::attach(|py| {
                let _ = cb.call0(py);
            });
        }
    }
}
