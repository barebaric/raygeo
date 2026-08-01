//! Intent API — convert `Plan`s into executable `NodeRequest`s.
//!
//! [`create_intent`] takes a descriptive [`Plan`] and a [`Part`],
//! shares each step's assembler spec by `Arc::clone`, wraps them in
//! [`AssemblerCompute`] nodes, and appends a final [`OpsAggregate`]
//! with `LinkMode::Sequential` for safe-Z travel.  The result is a
//! `Vec<NodeRequest>` — the Intent — suitable for
//! `pipeline::execute_stages`.

use std::sync::Arc;

use crate::cnc::execution::aggregate::OpsAggregate;
use crate::cnc::execution::compute::AssemblerCompute;
use crate::cnc::execution::specs::{
    AggregateGroup, AggregateInput, AggregateSpec, LinkMode, MachineParams,
};
use crate::cnc::plan::plan::Plan;
use crate::ops::part::Part;
use crate::ops::state::State;
use crate::pipeline::callbacks::NoCallbacks;
use crate::pipeline::request::NodeRequest;
use crate::pipeline::stage::StageSpec;

const IDENTITY_4X4: [[f64; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Convert a [`Plan`] into executable [`NodeRequest`]s (the Intent).
///
/// Produces one `NodeRequest` per step (with `StageSpec::Compute`
/// wrapping an [`AssemblerCompute`] built from the step's spec),
/// threads cleared state between consecutive steps via
/// `state_source_keys`, and appends a final `StageSpec::Aggregate`
/// with `LinkMode::Sequential` that links all steps with safe-Z
/// travel moves.
pub fn create_intent(
    plan: &Plan,
    part: &Part,
    generation_id: u64,
) -> Vec<NodeRequest> {
    let mut nodes: Vec<NodeRequest> = Vec::with_capacity(plan.steps.len() + 1);
    let mut prev_key: Option<String> = None;

    for (i, step) in plan.steps.iter().enumerate() {
        let name = step.spec.name();
        let key = format!("step:{i}:{name}");

        let state_source_keys = prev_key
            .as_ref()
            .map(|k| vec![k.clone()])
            .unwrap_or_default();

        let assembler = Arc::clone(&step.spec);
        let compute = AssemblerCompute {
            assembler,
            part: part.clone(),
            face_id: step.face_id.clone(),
            transformers: Vec::new(),
            cut_state: State::default(),
            state_source_keys,
            region_boundary: step.region_boundary.clone(),
            profile: false,
        };

        nodes.push(NodeRequest::new(
            key.clone(),
            generation_id,
            0,
            StageSpec::Compute {
                compute_fn: Box::new(compute),
            },
            Box::new(NoCallbacks),
        ));

        prev_key = Some(key);
    }

    if nodes.is_empty() {
        return nodes;
    }

    let step_keys: Vec<String> = nodes.iter().map(|n| n.key.clone()).collect();
    let aggregate = OpsAggregate {
        spec: AggregateSpec {
            wrap_start: Vec::new(),
            groups: vec![AggregateGroup {
                start_markers: Vec::new(),
                inputs: step_keys
                    .iter()
                    .map(|k| AggregateInput {
                        source_key: k.clone(),
                        placement_matrix: IDENTITY_4X4,
                        uid: String::new(),
                        target_dimensions: (0.0, 0.0),
                    })
                    .collect(),
                end_markers: Vec::new(),
                link_mode: LinkMode::Sequential {
                    safe_z: plan.safe_z,
                },
            }],
            wrap_end: Vec::new(),
            machine: MachineParams::default(),
            transformers: Vec::new(),
        },
    };

    nodes.push(NodeRequest::new(
        format!("aggregate:{}", generation_id),
        generation_id,
        0,
        StageSpec::Aggregate {
            aggregate_fn: Box::new(aggregate),
        },
        Box::new(NoCallbacks),
    ));

    nodes
}
