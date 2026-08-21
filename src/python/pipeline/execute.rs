pyo3_stub_gen::module_doc!("raygeo.pipeline.execute", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "Pipeline execution entry point.";

use std::sync::{Arc, Mutex, OnceLock};

use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::pipeline::cache::Cache;
use crate::pipeline::pipeline::Pipeline as CorePipeline;
use crate::python::errors::PipelineCancelled;
use crate::python::pipeline::request::PyNodeRequest;

// ── Injected execution hook (set by cnc layer during module init) ──

type ExecuteHook = dyn Fn(
        Python<'_>,
        Vec<Py<PyNodeRequest>>,
        Py<PyAny>,
        Option<Py<PyAny>>,
        Option<Arc<Mutex<Cache>>>,
    ) -> PyResult<()>
    + Send
    + Sync;

static EXECUTE_HOOK: OnceLock<Box<ExecuteHook>> = OnceLock::new();

pub fn set_execute_hook(hook: Box<ExecuteHook>) {
    let _ = EXECUTE_HOOK.set(hook);
}

// ── User-facing execute_stages (delegates to hook) ─────────────────

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo
    import raygeo.pipeline.completed
    import raygeo.pipeline.request

    def execute_stages(
        nodes: list[raygeo.pipeline.request.NodeRequest],
        on_completed: collections.abc.Callable[[raygeo.pipeline.completed.CompletedNode], None],
        on_batch_progress: collections.abc.Callable[[float, str], None] | None = None,
    ) -> None:
        """Run all nodes in a single rayon::scope.

        Fires ``on_completed`` for every node (success, failure, or
        cancellation) with a :class:`CompletedNode` carrying the
        node's ``key``, ``generation_id``, and either ``output`` or
        ``error``. ``on_batch_progress`` (optional) fires with the
        aggregate fraction and a status message on every per-node
        progress report and every completion.
        """ "#,
    module = "raygeo.pipeline.execute"
)]
#[pyfunction]
#[pyo3(signature = (nodes, on_completed, on_batch_progress=None))]
fn execute_stages(
    py: Python<'_>,
    nodes: Vec<Py<PyNodeRequest>>,
    on_completed: Py<PyAny>,
    on_batch_progress: Option<Py<PyAny>>,
) -> PyResult<()> {
    match EXECUTE_HOOK.get() {
        Some(hook) => hook(py, nodes, on_completed, on_batch_progress, None),
        None => Err(pyo3::exceptions::PyRuntimeError::new_err(
            "pipeline execute hook not initialized. Import raygeo.cnc first.",
        )),
    }
}

// ── Cache management ──────────────────────────────────────────────

#[gen_stub_pyfunction(module = "raygeo.pipeline.execute")]
#[pyfunction]
fn clear_cache() {
    default_pipeline().clear_cache();
}

// ── Pipeline class ────────────────────────────────────────────────

#[gen_stub_pyclass(module = "raygeo.pipeline.execute")]
#[pyclass(
    name = "Pipeline",
    module = "raygeo.pipeline.execute",
    skip_from_py_object
)]
pub struct PyPipeline {
    pub(crate) inner: CorePipeline,
}

impl PyPipeline {
    pub(crate) fn cache_handle(&self) -> Arc<Mutex<Cache>> {
        self.inner.cache_handle()
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyPipeline {
    /// Construct a pipeline with the given cache byte budget
    /// (default 2 GiB).
    #[new]
    #[pyo3(signature = (budget_bytes=2147483648))]
    fn new(budget_bytes: usize) -> Self {
        PyPipeline {
            inner: CorePipeline::new(budget_bytes),
        }
    }

    /// Run all nodes in a single ``rayon::scope``.
    ///
    /// :param nodes: List of
    ///     :class:`~raygeo.pipeline.request.NodeRequest` instances.
    /// :param on_completed: Callable ``(node: CompletedNode) -> None``
    ///     fired for every node.
    /// :param on_batch_progress: Optional callable
    ///     ``(fraction: float, message: str) -> None``.
    fn execute(
        &self,
        py: Python<'_>,
        nodes: Vec<Py<PyNodeRequest>>,
        on_completed: Py<PyAny>,
        on_batch_progress: Option<Py<PyAny>>,
    ) -> PyResult<()> {
        match EXECUTE_HOOK.get() {
        Some(hook) => hook(
                py,
                nodes,
                on_completed,
                on_batch_progress,
                Some(self.inner.cache_handle()),
            ),
            None => Err(pyo3::exceptions::PyRuntimeError::new_err(
                "pipeline execute hook not initialized. Import raygeo.cnc first.",
            )),
        }
    }

    /// Clear the entire cache.
    fn clear_cache(&self) {
        self.inner.clear_cache();
    }

    /// Clear all entries whose tag starts with ``prefix``.
    fn clear_cache_prefix(&self, prefix: &str) {
        self.inner.clear_cache_prefix(prefix);
    }

    /// Current bytes in use by the cache.
    #[getter]
    fn cache_used_bytes(&self) -> usize {
        self.inner.cache_used_bytes()
    }

    /// Configured byte budget.
    #[getter]
    fn cache_budget_bytes(&self) -> usize {
        self.inner.cache_budget_bytes()
    }

    /// Override the cache byte budget at runtime.
    ///
    /// If the new budget is smaller than current usage, entries are
    /// evicted (oldest first) until usage fits within the new limit.
    fn set_cache_budget_bytes(&self, budget: usize) {
        if let Ok(mut c) = self.inner.cache_handle().lock() {
            c.set_budget_bytes(budget);
        }
    }
}

/// Process-global default pipeline, used by the bare
/// `execute_stages` function. Per-document callers should construct
/// their own `Pipeline` instance for independent cache pruning.
pub(crate) fn default_cache_handle() -> Arc<Mutex<Cache>> {
    default_pipeline().cache_handle()
}

fn default_pipeline() -> &'static CorePipeline {
    use std::sync::LazyLock;
    static DEFAULT: LazyLock<CorePipeline> =
        LazyLock::new(CorePipeline::default);
    &DEFAULT
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let execute_mod = PyModule::new(py, "execute")?;
    execute_mod.setattr("__doc__", "Pipeline execution entry point.")?;
    execute_mod.add("PipelineCancelled", py.get_type::<PipelineCancelled>())?;
    execute_mod
        .add_function(wrap_pyfunction!(execute_stages, &execute_mod)?)?;
    execute_mod.add_function(wrap_pyfunction!(clear_cache, &execute_mod)?)?;
    execute_mod.add_class::<PyPipeline>()?;
    m.add_submodule(&execute_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.pipeline.execute", &execute_mod)?;

    Ok(())
}
