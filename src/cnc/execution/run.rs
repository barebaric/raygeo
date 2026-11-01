//! One-call execution of a planned pocket.
//!
//! [`execute_nodes`] is THE execution semantics of the cnc stack: build the
//! intent nodes, run them on a pipeline, deliver the aggregate. Both API
//! surfaces delegate to it — Python's `run_intent` and the Rust
//! [`run_plan`] — so there is exactly one behaviour, not two.

use std::sync::{Arc, Mutex};

use crate::cnc::execution::intent::create_intent;
use crate::cnc::execution::specs::AggregateOutput;
use crate::cnc::plan::plan::Plan;
use crate::error::{RaygeoError, RaygeoResult};
use crate::ops::container::Ops;
use crate::ops::part::Part;
use crate::pipeline::cache::Cache;
use crate::pipeline::completed::{CompletedNode, PipelineError};
use crate::pipeline::pipeline::Pipeline;

/// What running a plan produces: the linked toolpath and, when the plan's
/// machine parameters allow it, the estimated execution time (seconds).
#[derive(Debug, Clone)]
pub struct PlanOutput {
    pub ops: Ops,
    pub time_estimate: Option<f64>,
}

/// Executes pre-built pipeline nodes and captures the aggregate output.
///
/// A step that fails fails the run: the returned error carries the step's
/// error, and no partial toolpath is produced. `on_completed` (optional)
/// fires for every finished node; `on_batch_progress` and `cache` are the
/// same hooks [`Pipeline::execute`] offers.
pub fn execute_nodes(
    nodes: Vec<crate::pipeline::request::NodeRequest>,
    on_completed: Option<impl Fn(CompletedNode) + Send + Sync + 'static>,
    on_batch_progress: Option<Arc<dyn Fn(f64, String) + Send + Sync>>,
    cache: Option<Arc<Mutex<Cache>>>,
) -> RaygeoResult<PlanOutput> {
    let result: Arc<Mutex<Option<Result<PlanOutput, String>>>> =
        Arc::new(Mutex::new(None));
    let sink = result.clone();
    let capture = move |node: CompletedNode| {
        let mut slot = match sink.lock() {
            Ok(slot) => slot,
            Err(_) => return,
        };
        if slot.is_none() {
            let taken = if let Some(error) = &node.error {
                Some(Err(match error {
                    PipelineError::Cancelled => "cancelled".to_string(),
                    other => format!("{other:?}"),
                }))
            } else if let Some(output) = &node.output {
                output.downcast_ref::<AggregateOutput>().map(|aggregate| {
                    Ok(PlanOutput {
                        ops: aggregate.ops.clone(),
                        time_estimate: aggregate.time_estimate,
                    })
                })
            } else {
                None
            };
            *slot = taken;
        }
        drop(slot);
        if let Some(cb) = &on_completed {
            cb(node);
        }
    };

    let pipeline = match cache {
        Some(cache) => Pipeline::with_cache(cache),
        None => Pipeline::default(),
    };
    pipeline
        .execute(nodes, capture, on_batch_progress)
        .map_err(|_| RaygeoError::Cancelled)?;

    let taken = result
        .lock()
        .map_err(|_| RaygeoError::InternalError("pipeline panicked".into()))?
        .take();
    match taken {
        Some(Ok(output)) => Ok(output),
        Some(Err(message)) if message == "cancelled" => {
            Err(RaygeoError::Cancelled)
        }
        Some(Err(message)) => Err(RaygeoError::InternalError(format!(
            "a clearing step failed: {message}"
        ))),
        None => Err(RaygeoError::InternalError(
            "the pipeline produced no aggregate output".into(),
        )),
    }
}

/// Runs a [`Plan`] against its [`Part`] and returns the linked toolpath.
/// The one-call entry point for Rust embedders; see
/// `docs/embedding-rust.md`.
pub fn run_plan(plan: &Plan, part: &Part) -> RaygeoResult<PlanOutput> {
    let nodes = create_intent(plan, part, 0);
    if nodes.is_empty() {
        return Err(RaygeoError::InternalError("the plan has no steps".into()));
    }
    execute_nodes(nodes, None::<fn(CompletedNode)>, None, None)
}
