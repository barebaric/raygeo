use crate::pipeline::aggregate::Aggregate;
use crate::pipeline::compute::Compute;

pub enum StageSpec {
    Compute { compute_fn: Box<dyn Compute> },
    Aggregate { aggregate_fn: Box<dyn Aggregate> },
}

impl StageSpec {
    pub fn source_keys(&self) -> Vec<String> {
        match self {
            StageSpec::Compute { compute_fn } => compute_fn.source_keys(),
            StageSpec::Aggregate { aggregate_fn } => aggregate_fn.source_keys(),
        }
    }
}

impl std::fmt::Debug for StageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StageSpec::Compute { compute_fn } => f
                .debug_struct("Compute")
                .field("compute_fn", &compute_fn.name())
                .finish(),
            StageSpec::Aggregate { aggregate_fn } => f
                .debug_struct("Aggregate")
                .field("aggregate_fn", &aggregate_fn.name())
                .finish(),
        }
    }
}
