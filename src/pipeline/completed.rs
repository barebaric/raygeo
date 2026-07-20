use std::any::Any;
use std::sync::Arc;

pub struct CompletedNode {
    pub key: String,
    pub generation_id: u64,
    pub output: Option<Arc<dyn Any + Send + Sync>>,
    pub error: Option<String>,
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

    pub fn err(key: String, generation_id: u64, error: String) -> Self {
        CompletedNode {
            key,
            generation_id,
            output: None,
            error: Some(error),
        }
    }
}
