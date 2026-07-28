use std::any::Any;
use std::fmt;
use std::sync::Arc;

/// Typed pipeline execution error.
#[derive(Debug, Clone)]
pub enum PipelineError {
    /// Node was cancelled (normal during rapid rebuilds).
    Cancelled,
    /// A dependency of this node failed.
    UpstreamFailed,
    /// The cache budget does not allow storing this node's output.
    CacheBudgetExceeded {
        node_key: String,
        size: usize,
        budget: usize,
    },
    /// Any other execution failure.
    Other(String),
}

impl PipelineError {
    /// Machine-readable error kind string (no UUIDs or variable data).
    pub fn kind(&self) -> &'static str {
        match self {
            PipelineError::Cancelled => "cancelled",
            PipelineError::UpstreamFailed => "upstream_failed",
            PipelineError::CacheBudgetExceeded { .. } => {
                "cache_budget_exceeded"
            }
            PipelineError::Other(_) => "other",
        }
    }
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineError::Cancelled => write!(f, "cancelled"),
            PipelineError::UpstreamFailed => write!(f, "upstream failed"),
            PipelineError::CacheBudgetExceeded {
                node_key,
                size,
                budget,
            } => {
                write!(
                    f,
                    "Cache budget exceeded: node '{node_key}' requires {size} bytes \
                     but the cache budget is {budget} bytes. Reduce scene complexity \
                     or increase the cache budget.",
                )
            }
            PipelineError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<String> for PipelineError {
    fn from(msg: String) -> Self {
        PipelineError::Other(msg)
    }
}

pub struct CompletedNode {
    pub key: String,
    pub generation_id: u64,
    pub output: Option<Arc<dyn Any + Send + Sync>>,
    pub error: Option<PipelineError>,
}

impl CompletedNode {
    pub fn ok(
        key: String,
        generation_id: u64,
        output: Arc<dyn Any + Send + Sync>,
    ) -> Self {
        CompletedNode {
            key,
            generation_id,
            output: Some(output),
            error: None,
        }
    }

    pub fn err(key: String, generation_id: u64, error: PipelineError) -> Self {
        CompletedNode {
            key,
            generation_id,
            output: None,
            error: Some(error),
        }
    }
}
