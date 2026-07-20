pyo3_stub_gen::module_doc!("raygeo.pipeline.request", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "Node request types.";

use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass(module = "raygeo.pipeline.request")]
#[pyclass(
    name = "NodeRequest",
    module = "raygeo.pipeline.request",
    skip_from_py_object
)]
pub struct PyNodeRequest {
    #[pyo3(get)]
    pub key: String,
    #[pyo3(get)]
    pub generation_id: u64,
    #[pyo3(get)]
    pub stage: Py<PyAny>,
    #[pyo3(get)]
    pub on_progress: Option<Py<PyAny>>,
    #[pyo3(get)]
    pub on_cancelled: Option<Py<PyAny>>,
    #[pyo3(get)]
    pub on_chunk: Option<Py<PyAny>>,
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyNodeRequest {
    #[new]
    #[pyo3(signature = (key, generation_id, stage, on_progress=None, on_cancelled=None, on_chunk=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        key: String,
        generation_id: u64,
        stage: Py<PyAny>,
        on_progress: Option<Py<PyAny>>,
        on_cancelled: Option<Py<PyAny>>,
        on_chunk: Option<Py<PyAny>>,
    ) -> Self {
        PyNodeRequest {
            key,
            generation_id,
            stage,
            on_progress,
            on_cancelled,
            on_chunk,
        }
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let request_mod = PyModule::new(py, "request")?;
    request_mod.setattr("__doc__", "Node request types.")?;
    request_mod.add_class::<PyNodeRequest>()?;
    m.add_submodule(&request_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.pipeline.request", &request_mod)?;

    Ok(())
}
