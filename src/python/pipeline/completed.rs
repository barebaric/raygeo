pyo3_stub_gen::module_doc!("raygeo.pipeline.completed", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "Completion record types.";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum};

/// Machine-readable error category for a failed pipeline node.
#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.pipeline.completed",
    name = "ErrorKind",
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyErrorKind {
    /// Node was cancelled (normal during rapid rebuilds).
    #[pyo3(name = "CANCELLED")]
    Cancelled,
    /// A dependency of this node failed.
    #[pyo3(name = "UPSTREAM_FAILED")]
    UpstreamFailed,
    /// The cache budget does not allow storing this node's output.
    #[pyo3(name = "CACHE_BUDGET_EXCEEDED")]
    CacheBudgetExceeded,
    /// The pipeline cache mutex is poisoned.
    #[pyo3(name = "CACHE_LOCK_POISONED")]
    CacheLockPoisoned,
    /// Any other execution failure.
    #[pyo3(name = "OTHER")]
    Other,
}

#[pymethods]
impl PyErrorKind {
    fn __repr__(&self) -> String {
        match self {
            PyErrorKind::Cancelled => "ErrorKind.CANCELLED".into(),
            PyErrorKind::UpstreamFailed => "ErrorKind.UPSTREAM_FAILED".into(),
            PyErrorKind::CacheBudgetExceeded => {
                "ErrorKind.CACHE_BUDGET_EXCEEDED".into()
            }
            PyErrorKind::CacheLockPoisoned => {
                "ErrorKind.CACHE_LOCK_POISONED".into()
            }
            PyErrorKind::Other => "ErrorKind.OTHER".into(),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    #[getter]
    fn value(&self) -> &str {
        match self {
            PyErrorKind::Cancelled => "cancelled",
            PyErrorKind::UpstreamFailed => "upstream_failed",
            PyErrorKind::CacheBudgetExceeded => "cache_budget_exceeded",
            PyErrorKind::CacheLockPoisoned => "cache_lock_poisoned",
            PyErrorKind::Other => "other",
        }
    }
}

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
    #[pyo3(get)]
    pub error_kind: Option<PyErrorKind>,
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let completed_mod = PyModule::new(py, "completed")?;
    completed_mod.setattr("__doc__", "Completion record types.")?;
    completed_mod.add_class::<PyErrorKind>()?;
    completed_mod.add_class::<PyCompletedNode>()?;
    m.add_submodule(&completed_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.pipeline.completed", &completed_mod)?;

    Ok(())
}
