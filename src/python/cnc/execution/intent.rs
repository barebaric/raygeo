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
use crate::python::cnc::execution::converter::completed_node_from_core;
use crate::python::cnc::plan::PyPlan;
use crate::python::ops::container::PyOps;
use crate::python::ops::part::part::PyPart;

/// An executable Intent produced by [`create_intent`].
///
/// Holds the raw [`NodeRequest`]s inside a shared container so that
/// [`run_intent`] can move them out at execution time.
#[gen_stub_pyclass(module = "raygeo.cnc.execution.intent")]
#[pyclass(
    name = "Intent",
    module = "raygeo.cnc.execution.intent",
    skip_from_py_object
)]
#[derive(Debug)]
pub struct PyIntent {
    nodes: Arc<Mutex<Vec<NodeRequest>>>,
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
        format!("Intent(nodes={})", n)
    }
}

/// Convert a Plan and Part into an executable Intent.
#[gen_stub_pyfunction(module = "raygeo.cnc.execution.intent")]
#[pyfunction]
fn create_intent(plan: &PyPlan, part: &PyPart, generation_id: u64) -> PyIntent {
    let nodes = intent::create_intent(&plan.inner, &part.inner, generation_id);
    PyIntent {
        nodes: Arc::new(Mutex::new(nodes)),
    }
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
    pipeline: Option<&crate::python::pipeline::execute::PyPipeline>,
) -> PyResult<Py<PyOps>> {
    let cache = pipeline.and_then(|_p| None::<Arc<Mutex<Cache>>>);

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
            .map_err(|_| PyRuntimeError::new_err("pipeline was cancelled"))?;
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
    m.add_function(wrap_pyfunction!(run_intent, m.clone())?)?;
    exec_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.execution.intent", &m)?;

    Ok(())
}
