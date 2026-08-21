use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::cnc::execution::intent;
use crate::cnc::execution::specs::AggregateOutput;
use crate::pipeline::cache::Cache;
use crate::pipeline::completed::CompletedNode;
use crate::pipeline::pipeline::Pipeline as CorePipeline;
use crate::pipeline::request::NodeRequest;
use crate::python::cnc::execution::converter::{
    completed_node_from_core, convert_node_request,
};
use crate::python::cnc::plan::PyPlan;
use crate::python::ops::container::PyOps;
use crate::python::ops::part::part::PyPart;
use crate::python::pipeline::execute::{default_cache_handle, PyPipeline};
use crate::python::pipeline::request::PyNodeRequest;

/// An executable Intent produced by [`create_intent`].
///
/// Holds the raw [`NodeRequest`]s inside a shared container so that
/// [`run_intent`] can move them out at execution time. The
/// ``last_tokens`` map records each node's ``version_token`` from the
/// last [`update <PyIntent::__pyo3_get__update>`] call, enabling
/// diff-based cache invalidation.
#[gen_stub_pyclass(module = "raygeo.cnc.execution.intent")]
#[pyclass(
    name = "Intent",
    module = "raygeo.cnc.execution.intent",
    skip_from_py_object
)]
#[derive(Debug)]
pub struct PyIntent {
    nodes: Arc<Mutex<Vec<NodeRequest>>>,
    last_tokens: Mutex<HashMap<String, u64>>,
    cancel_flag: Mutex<Arc<AtomicBool>>,
}

impl PyIntent {
    fn extract_tokens(nodes: &[NodeRequest]) -> HashMap<String, u64> {
        nodes
            .iter()
            .map(|n| (n.key.clone(), n.version_token))
            .collect()
    }

    fn build_dependents(nodes: &[NodeRequest]) -> HashMap<String, Vec<String>> {
        let mut deps: HashMap<String, Vec<String>> = HashMap::new();
        for n in nodes {
            for src in n.stage.source_keys() {
                deps.entry(src).or_default().push(n.key.clone());
            }
        }
        deps
    }

    fn propagate(
        seeds: Vec<String>,
        dependents: &HashMap<String, Vec<String>>,
    ) -> HashSet<String> {
        let mut affected: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        for s in seeds {
            if affected.insert(s.clone()) {
                queue.push_back(s);
            }
        }
        while let Some(key) = queue.pop_front() {
            if let Some(deps) = dependents.get(&key) {
                for dep in deps {
                    if affected.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }
        affected
    }

    fn evict_keys(cache: &Arc<Mutex<Cache>>, keys: &HashSet<String>) {
        if let Ok(mut c) = cache.lock() {
            for key in keys {
                c.remove_entry(key);
                c.bump_epoch(key);
            }
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyIntent {
    /// Number of compute nodes in this intent (excluding the final aggregate).
    #[getter]
    fn step_count(&self) -> usize {
        self.nodes.lock().unwrap().len().saturating_sub(1)
    }

    fn __repr__(&self) -> String {
        let n = self.nodes.lock().unwrap().len();
        let t = self.last_tokens.lock().unwrap().len();
        format!("Intent(nodes={n}, tracked={t})")
    }

    /// Signal the currently running execution of this intent to cancel
    /// cooperatively.
    fn cancel(&self) {
        self.cancel_flag
            .lock()
            .unwrap()
            .store(true, Ordering::SeqCst);
    }

    /// Diff this intent against ``new_intent`` and update internal
    /// state, evicting stale cache entries.
    ///
    /// For each node whose ``version_token`` changed (or that was
    /// removed), the corresponding cache entry is evicted and its
    /// epoch bumped. Transitive dependents are invalidated too.
    /// Unchanged nodes keep their cache entries.
    ///
    /// After ``update``, the old intent holds the new node list and
    /// can be executed with :func:`run_intent`.
    #[pyo3(signature = (new_intent, pipeline=None))]
    fn update(&self, new_intent: &PyIntent, pipeline: Option<&PyPipeline>) {
        let cache = match pipeline {
            Some(p) => p.cache_handle(),
            None => default_cache_handle(),
        };

        let new_nodes = new_intent
            .nodes
            .lock()
            .unwrap()
            .drain(..)
            .collect::<Vec<_>>();
        let new_tokens = Self::extract_tokens(&new_nodes);
        let dependents = Self::build_dependents(&new_nodes);

        let old_tokens = self.last_tokens.lock().unwrap();
        let changed: Vec<String> = old_tokens
            .iter()
            .filter_map(|(key, old_token)| match new_tokens.get(key) {
                Some(new_token) if new_token != old_token => Some(key.clone()),
                _ => None,
            })
            .collect();
        let removed: Vec<String> = old_tokens
            .keys()
            .filter(|k| !new_tokens.contains_key(*k))
            .cloned()
            .collect();
        drop(old_tokens);

        let to_evict = Self::propagate(changed, &dependents);
        Self::evict_keys(&cache, &to_evict);

        let removed_evict = Self::propagate(removed, &dependents);
        Self::evict_keys(&cache, &removed_evict);

        // Transfer the cancel flag so future intent.cancel() sets the
        // correct flag that the swapped-in nodes' callbacks reference.
        {
            let new_flag = new_intent.cancel_flag.lock().unwrap();
            *self.cancel_flag.lock().unwrap() = new_flag.clone();
        }

        *self.last_tokens.lock().unwrap() = new_tokens;
        *self.nodes.lock().unwrap() = new_nodes;
    }

    /// Manually invalidate specific node keys and their transitive
    /// dependents.
    ///
    /// Cache entries for each key (and every node that depends on
    /// them, transitively) are evicted and their epochs bumped.
    /// The next :func:`run_intent` call recomputes them.
    ///
    /// This is the escape hatch for cases where node content changed
    /// without a ``version_token`` change — e.g. an in-place raster
    /// pixel edit.
    #[pyo3(signature = (keys, pipeline=None))]
    fn invalidate(&self, keys: Vec<String>, pipeline: Option<&PyPipeline>) {
        let cache = match pipeline {
            Some(p) => p.cache_handle(),
            None => default_cache_handle(),
        };

        let nodes = self.nodes.lock().unwrap();
        let dependents = Self::build_dependents(&nodes);
        drop(nodes);

        let to_evict = Self::propagate(keys, &dependents);
        Self::evict_keys(&cache, &to_evict);
    }
}

/// Convert a Plan and Part into an executable Intent.
#[gen_stub_pyfunction(module = "raygeo.cnc.execution.intent")]
#[pyfunction]
fn create_intent(plan: &PyPlan, part: &PyPart, generation_id: u64) -> PyIntent {
    let nodes = intent::create_intent(&plan.inner, &part.inner, generation_id);
    let last_tokens = PyIntent::extract_tokens(&nodes);
    PyIntent {
        nodes: Arc::new(Mutex::new(nodes)),
        last_tokens: Mutex::new(last_tokens),
        cancel_flag: Mutex::new(Arc::new(AtomicBool::new(false))),
    }
}

/// Build an Intent from a list of raw :class:`~raygeo.pipeline.request.NodeRequest` objects.
///
/// Useful for callers that construct their own node list without going
/// through the Plan API.
#[gen_stub_pyfunction(module = "raygeo.cnc.execution.intent")]
#[pyfunction]
fn create_intent_from_nodes(
    py: Python<'_>,
    nodes: Vec<Py<PyNodeRequest>>,
) -> PyResult<PyIntent> {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let core_nodes: Vec<NodeRequest> = nodes
        .iter()
        .map(|n| convert_node_request(py, &n.borrow(py), &cancel_flag))
        .collect::<PyResult<_>>()?;
    let last_tokens = PyIntent::extract_tokens(&core_nodes);
    Ok(PyIntent {
        nodes: Arc::new(Mutex::new(core_nodes)),
        last_tokens: Mutex::new(last_tokens),
        cancel_flag: Mutex::new(cancel_flag),
    })
}

/// Run an Intent through the pipeline, consuming the node list.
///
/// Returns the final aggregated :class:`~raygeo.ops.Ops` (all steps
/// linked with safe-Z travel).  ``on_completed`` is invoked for each
/// completed node (including the aggregate) for progress monitoring.
#[gen_stub_pyfunction(module = "raygeo.cnc.execution.intent")]
#[pyfunction]
#[pyo3(signature = (intent, on_completed=None, on_batch_progress=None, pipeline=None))]
fn run_intent(
    py: Python<'_>,
    intent: &PyIntent,
    on_completed: Option<Py<PyAny>>,
    on_batch_progress: Option<Py<PyAny>>,
    pipeline: Option<&PyPipeline>,
) -> PyResult<Py<PyOps>> {
    let cache = pipeline.map(|p| p.cache_handle());

    let nodes = intent.nodes.lock().unwrap().drain(..).collect::<Vec<_>>();

    let on_completed_cb = on_completed;
    let on_batch = on_batch_progress.map(|cb| {
        Arc::new(move |frac: f64, msg: String| {
            Python::attach(|py| {
                let _ = cb.call1(py, (frac, msg));
            });
        }) as Arc<dyn Fn(f64, String) + Send + Sync + 'static>
    });

    // Capture the aggregate node's Ops so we can return it.
    let result_ops: Arc<Mutex<Option<crate::ops::Ops>>> =
        Arc::new(Mutex::new(None));
    let result_ops_capture = result_ops.clone();

    py.detach(move || -> PyResult<()> {
        let pipeline = match cache {
            Some(c) => CorePipeline::with_cache(c),
            None => CorePipeline::default(),
        };
        pipeline
            .execute(
                nodes,
                move |node: CompletedNode| {
                    // Capture the aggregate output.
                    if let Some(ref output) = node.output {
                        if let Some(agg) =
                            output.downcast_ref::<AggregateOutput>()
                        {
                            *result_ops_capture.lock().unwrap() =
                                Some(agg.ops.clone());
                        }
                    }
                    if let Some(ref cb) = on_completed_cb {
                        Python::attach(|py| {
                            let py_node = completed_node_from_core(py, node);
                            let _ = cb.call1(py, (py_node,));
                        });
                    }
                },
                on_batch,
            )
            .map_err(|_| {
                crate::python::errors::PipelineCancelled::new_err(
                    "pipeline was cancelled",
                )
            })?;
        Ok(())
    })?;

    let ops = result_ops.lock().unwrap().take().ok_or_else(|| {
        PyRuntimeError::new_err("pipeline produced no aggregate output")
    })?;
    Py::new(py, PyOps { inner: ops })
}

pub(crate) fn register(exec_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = exec_mod.py();
    let m = PyModule::new(py, "intent")?;
    m.add_class::<PyIntent>()?;
    m.add_function(wrap_pyfunction!(create_intent, m.clone())?)?;
    m.add_function(wrap_pyfunction!(create_intent_from_nodes, m.clone())?)?;
    m.add_function(wrap_pyfunction!(run_intent, m.clone())?)?;
    exec_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.execution.intent", &m)?;

    Ok(())
}
