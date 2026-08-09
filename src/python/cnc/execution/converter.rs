use std::any::Any;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3::types::PyAny;

use crate::cnc::execution::aggregate::OpsAggregate;
use crate::cnc::execution::compute::AssemblerCompute;
use crate::cnc::execution::encode::EncoderCompute;
use crate::cnc::execution::machine_transform::MachineTransformCompute;
use crate::cnc::execution::specs::AggregateOutput;
use crate::ops::assembly::Assembler;
use crate::ops::assembly::AssemblyOutput;
use crate::ops::convert::EncodeOutput;
use crate::ops::part::Part;
use crate::pipeline::cache::Cache;
use crate::pipeline::request::NodeRequest as CoreNodeRequest;

type ExecuteFn = dyn Fn(
        Python<'_>,
        Vec<Py<PyNodeRequest>>,
        Py<PyAny>,
        Option<Py<PyAny>>,
        Option<Arc<Mutex<Cache>>>,
    ) -> PyResult<()>
    + Send
    + Sync;
use crate::pipeline::stage::StageSpec as CoreStageSpec;
use crate::python::cnc::execution::specs::PyComputePayload;
use crate::python::cnc::execution::specs::PyEncodeSpec;
use crate::python::cnc::execution::specs::PyMachineTransformSpec;
use crate::python::ops::assembly::PyAssemblyOutput;
use crate::python::ops::convert::PyEncodeOutput;
use crate::python::pipeline::callbacks::PyTaskCallbacks;
use crate::python::pipeline::completed::PyCompletedNode;
use crate::python::pipeline::request::PyNodeRequest;
use crate::python::pipeline::stage::PyStageSpec;

// ── StageSpec conversion ──────────────────────────────────────────

fn convert_stage(
    py: Python<'_>,
    spec: &PyStageSpec,
) -> PyResult<CoreStageSpec> {
    Ok(match spec {
        PyStageSpec::Compute {
            part,
            params,
            face_id,
        } => {
            let part_any = part.bind(py);
            let part_bound = part_any
                .cast::<crate::python::ops::part::part::PyPart>()
                .map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err("part is not a Part")
                })?;
            let mut part_ref = part_bound.borrow_mut();
            let part_inner = std::mem::replace(
                &mut part_ref.inner,
                Part::new(None, (0.0, 0.0)),
            );
            drop(part_ref);

            let params_any = params.bind(py);
            let params_bound =
                params_any.cast::<PyComputePayload>().map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err(
                        "params is not a ComputePayload",
                    )
                })?;
            let params_ref = params_bound.borrow();
            let assembler_any = params_ref.assembler.bind(py);
            let assembler = assembler_any
                .cast::<crate::python::ops::assembly::PyAssembler>()
                .map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err(
                        "assembler is not an Assembler",
                    )
                })?;
            let assembler: Arc<dyn Assembler> =
                Arc::from(assembler.borrow().into_core(py)?);
            let transformers =
                params_ref
                    .transformers
                    .iter()
                    .map(|t| {
                        crate::python::ops::transform::extract_transformer(
                            t.bind(py),
                        )
                    })
                    .collect::<PyResult<
                        Vec<Box<dyn crate::ops::transform::Transformer>>,
                    >>()?;
            let state_source_keys = params_ref.state_source_keys.clone();
            let profile = params_ref.profile;
            let cut_state = crate::ops::state::State {
                power: params_ref.power,
                feed_rate: if params_ref.cut_speed > 0 {
                    Some(params_ref.cut_speed)
                } else {
                    None
                },
                active_head_uid: params_ref.head_uid.clone(),
                ..Default::default()
            };
            drop(params_ref);

            let compute = AssemblerCompute {
                assembler,
                part: part_inner,
                face_id: face_id.clone(),
                transformers,
                cut_state,
                state_source_keys,
                region_boundary: None,
                profile,
            };
            CoreStageSpec::Compute {
                compute_fn: Box::new(compute),
            }
        }
        PyStageSpec::Aggregate { spec } => {
            let spec_any = spec.bind(py);
            let spec_bound = spec_any
                .cast::<crate::python::cnc::execution::specs::PyAggregateSpec>()
                .map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err(
                        "spec is not an AggregateSpec",
                    )
                })?;
            let agg_spec = spec_bound.borrow().to_core(py)?;
            CoreStageSpec::Aggregate {
                aggregate_fn: Box::new(OpsAggregate { spec: agg_spec }),
            }
        }
    })
}

// ── EncodeSpec conversion ─────────────────────────────────────────

fn convert_encode_spec(
    py: Python<'_>,
    encode_spec: &PyEncodeSpec,
) -> PyResult<CoreStageSpec> {
    let encoder = encode_spec.encoder.borrow(py).into_core(py)?;
    let compute = EncoderCompute {
        encoder,
        source_key: encode_spec.source_key.clone(),
    };
    Ok(CoreStageSpec::Compute {
        compute_fn: Box::new(compute),
    })
}

// ── MachineTransformSpec conversion ───────────────────────────────

fn convert_machine_transform_spec(
    py: Python<'_>,
    spec: &PyMachineTransformSpec,
) -> PyResult<CoreStageSpec> {
    let compute = MachineTransformCompute {
        spec: spec.to_core(py),
    };
    Ok(CoreStageSpec::Compute {
        compute_fn: Box::new(compute),
    })
}

// ── NodeRequest conversion ────────────────────────────────────────

pub(crate) fn convert_node_request(
    py: Python<'_>,
    req: &PyNodeRequest,
    cancel_flag: &Arc<AtomicBool>,
) -> PyResult<CoreNodeRequest> {
    let stage_any = req.stage.bind(py);

    // Try pipeline StageSpec first (Compute / Aggregate).
    if let Ok(stage_bound) = stage_any.cast::<PyStageSpec>() {
        let stage = stage_bound.borrow();
        let callbacks = PyTaskCallbacks::new(
            req.on_progress.clone(),
            req.on_cancelled.clone(),
            req.on_chunk.clone(),
            Arc::clone(cancel_flag),
        );
        return Ok(CoreNodeRequest::new(
            req.key.clone(),
            req.generation_id,
            req.version_token,
            convert_stage(py, &stage)?,
            Box::new(callbacks),
            req.cacheable,
        ));
    }

    // Try EncodeSpec.
    if let Ok(enc_bound) = stage_any.cast::<PyEncodeSpec>() {
        let stage = convert_encode_spec(py, &enc_bound.borrow())?;
        let callbacks = PyTaskCallbacks::new(
            req.on_progress.clone(),
            req.on_cancelled.clone(),
            req.on_chunk.clone(),
            Arc::clone(cancel_flag),
        );
        return Ok(CoreNodeRequest::new(
            req.key.clone(),
            req.generation_id,
            req.version_token,
            stage,
            Box::new(callbacks),
            req.cacheable,
        ));
    }

    // Try MachineTransformSpec (wrapped as StageSpec.Compute).
    if let Ok(mt_bound) = stage_any.cast::<PyMachineTransformSpec>() {
        let mt_spec = mt_bound.borrow();
        let stage = convert_machine_transform_spec(py, &mt_spec)?;
        let callbacks = PyTaskCallbacks::new(
            req.on_progress.clone(),
            req.on_cancelled.clone(),
            req.on_chunk.clone(),
            Arc::clone(cancel_flag),
        );
        return Ok(CoreNodeRequest::new(
            req.key.clone(),
            req.generation_id,
            req.version_token,
            stage,
            Box::new(callbacks),
            req.cacheable,
        ));
    }

    Err(pyo3::exceptions::PyTypeError::new_err(
        "stage is not a StageSpec, EncodeSpec, or MachineTransformSpec",
    ))
}

// ── Output conversion (Arc<dyn Any> → Python object) ──────────────

fn any_to_py(
    py: Python<'_>,
    output: Arc<dyn Any + Send + Sync>,
) -> Option<Py<PyAny>> {
    if let Some(assembly) = output.downcast_ref::<AssemblyOutput>() {
        let py_ao = PyAssemblyOutput {
            inner: assembly.clone(),
        };
        return Bound::new(py, py_ao).ok().map(|o| o.into_any().unbind());
    }
    if output.downcast_ref::<AggregateOutput>().is_some() {
        let py_ao =
            crate::python::cnc::execution::specs::PyAggregateOutput::from_arc(
                output, py,
            );
        return Bound::new(py, py_ao).ok().map(|o| o.into_any().unbind());
    }
    if output.downcast_ref::<EncodeOutput>().is_some() {
        let py_eo = PyEncodeOutput::from_arc(output);
        return Bound::new(py, py_eo).ok().map(|o| o.into_any().unbind());
    }
    Some(py.None())
}

use crate::python::pipeline::completed::PyErrorKind;

// ── CompletedNode conversion ──────────────────────────────────────

pub(crate) fn completed_node_from_core(
    py: Python<'_>,
    node: crate::pipeline::completed::CompletedNode,
) -> PyCompletedNode {
    let error_kind = node.error.as_ref().map(|e| match e {
        crate::pipeline::completed::PipelineError::Cancelled => {
            PyErrorKind::Cancelled
        }
        crate::pipeline::completed::PipelineError::UpstreamFailed => {
            PyErrorKind::UpstreamFailed
        }
        crate::pipeline::completed::PipelineError::CacheBudgetExceeded {
            ..
        } => PyErrorKind::CacheBudgetExceeded,
        crate::pipeline::completed::PipelineError::CacheLockPoisoned => {
            PyErrorKind::CacheLockPoisoned
        }
        crate::pipeline::completed::PipelineError::Other(_) => {
            PyErrorKind::Other
        }
    });
    PyCompletedNode {
        key: node.key,
        generation_id: node.generation_id,
        output: node.output.and_then(|arc| any_to_py(py, arc)),
        error: node.error.map(|e| e.to_string()),
        error_kind,
    }
}

// ── Execute hook ──────────────────────────────────────────────────

pub fn create_execute_hook() -> Box<ExecuteFn> {
    Box::new(
        move |py: Python<'_>,
              nodes: Vec<Py<PyNodeRequest>>,
              on_completed: Py<PyAny>,
              on_batch_progress: Option<Py<PyAny>>,
              cache: Option<Arc<Mutex<Cache>>>| {
            // Nodes created via the bare execute() path don't belong
            // to an Intent, so they get a dummy never-cancelled flag.
            let default_flag = Arc::new(AtomicBool::new(false));
            let core_nodes: Vec<CoreNodeRequest> = nodes
                .iter()
                .map(|n| convert_node_request(py, &n.borrow(py), &default_flag))
                .collect::<PyResult<_>>()?;

            let on_completed_cb = Arc::new(on_completed);
            let on_batch = on_batch_progress.map(|cb| {
                Arc::new(move |frac: f64, msg: String| {
                    Python::attach(|py| {
                        let _ = cb.call1(py, (frac, msg));
                    });
                })
                    as Arc<dyn Fn(f64, String) + Send + Sync + 'static>
            });

            py.detach(move || {
                let pipeline = match cache {
                    Some(c) => {
                        crate::pipeline::pipeline::Pipeline::with_cache(c)
                    }
                    None => crate::pipeline::pipeline::Pipeline::default(),
                };
                pipeline
                    .execute(
                        core_nodes,
                        move |node| {
                            Python::attach(|py| {
                                let py_node =
                                    completed_node_from_core(py, node);
                                let _ = on_completed_cb.call1(py, (py_node,));
                            });
                        },
                        on_batch,
                    )
                    .map_err(|_| {
                        pyo3::exceptions::PyRuntimeError::new_err(
                            "pipeline was cancelled",
                        )
                    })
            })
        },
    )
}
