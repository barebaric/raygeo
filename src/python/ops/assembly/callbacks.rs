pyo3_stub_gen::module_doc!("raygeo.ops.assembly.callbacks", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "Per-node callback bridge types.";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyclass;

use crate::ops::assembly::callbacks::{ChunkPayload, TaskCallbacks};

/// Python-side ``TaskCallbacks`` that reacquires the GIL and calls
/// the stored Python callables.
///
/// Each stage node on the Rust side holds a ``PyTaskCallbacks`` as a
/// ``Box<dyn TaskCallbacks>``. When the stage calls
/// ``report_progress`` / ``is_cancelled`` / ``emit_chunk``, this
/// impl reacquires the GIL and delegates to the Python closures.
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

impl TaskCallbacks for PyTaskCallbacks {
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

    fn emit_chunk(&self, chunk: ChunkPayload) {
        if let Some(ref cb) = self.on_chunk {
            Python::attach(|py| {
                let payload = chunk.to_py(py);
                let _ = cb.call1(py, (payload,));
            });
        }
    }
}

/// Python-visible chunk payload. Mirrors ``ChunkPayload``.
#[gen_stub_pyclass(module = "raygeo.ops.assembly.callbacks")]
#[pyclass(
    name = "ChunkPayload",
    module = "raygeo.ops.assembly.callbacks",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyChunkPayload {
    #[pyo3(get)]
    pub y_start: u32,
    #[pyo3(get)]
    pub y_end: u32,
    #[pyo3(get)]
    pub message: String,
}

impl ChunkPayload {
    fn to_py(&self, _py: Python<'_>) -> PyChunkPayload {
        PyChunkPayload {
            y_start: self.y_start,
            y_end: self.y_end,
            message: self.message.clone(),
        }
    }
}

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let callbacks_mod = PyModule::new(py, "callbacks")?;
    callbacks_mod.setattr("__doc__", MODULE_DOC)?;
    callbacks_mod.add_class::<PyChunkPayload>()?;
    parent.add_submodule(&callbacks_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.callbacks", &callbacks_mod)?;

    Ok(())
}
