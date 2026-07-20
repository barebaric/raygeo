pyo3_stub_gen::module_doc!("raygeo.pipeline.completed", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "Completion record types.";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyclass;

#[gen_stub_pyclass(module = "raygeo.pipeline.completed")]
#[pyclass(
    name = "CompletedNode",
    module = "raygeo.pipeline.completed",
    skip_from_py_object
)]
pub struct PyCompletedNode {
    #[pyo3(get)]
    pub key: String,
    #[pyo3(get)]
    pub generation_id: u64,
    #[pyo3(get)]
    pub output: Option<Py<PyAny>>,
    #[pyo3(get)]
    pub error: Option<String>,
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let completed_mod = PyModule::new(py, "completed")?;
    completed_mod.setattr("__doc__", "Completion record types.")?;
    completed_mod.add_class::<PyCompletedNode>()?;
    m.add_submodule(&completed_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.pipeline.completed", &completed_mod)?;

    Ok(())
}
