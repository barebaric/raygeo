use std::sync::Arc;

use crate::pipeline::cache::Cache;
use crate::pipeline::completed::CompletedNode;
use crate::pipeline::execute::{execute_stages, Cancelled};
use crate::pipeline::request::NodeRequest;

pub struct Pipeline {
    cache: Arc<std::sync::Mutex<Cache>>,
}

impl std::fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let used = self.cache.lock().map(|c| c.used_bytes()).unwrap_or(0);
        let budget = self.cache.lock().map(|c| c.budget_bytes()).unwrap_or(0);
        f.debug_struct("Pipeline")
            .field("cache_used_bytes", &used)
            .field("cache_budget_bytes", &budget)
            .finish()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Pipeline::new(256 * 1024 * 1024)
    }
}

impl Pipeline {
    pub fn new(budget_bytes: usize) -> Self {
        Pipeline {
            cache: Arc::new(std::sync::Mutex::new(Cache::new(budget_bytes))),
        }
    }

    pub fn execute(
        &self,
        nodes: Vec<NodeRequest>,
        on_completed: impl Fn(CompletedNode) + Send + Sync + 'static,
        on_batch_progress: Option<
            Arc<dyn Fn(f64, String) + Send + Sync + 'static>,
        >,
    ) -> Result<(), Cancelled> {
        execute_stages(nodes, on_completed, on_batch_progress, &self.cache)
    }

    pub fn clear_cache(&self) {
        if let Ok(mut c) = self.cache.lock() {
            c.clear();
        }
    }

    pub fn clear_cache_prefix(&self, prefix: &str) {
        if let Ok(mut c) = self.cache.lock() {
            c.clear_prefix(prefix);
        }
    }

    pub fn cache_used_bytes(&self) -> usize {
        self.cache.lock().map(|c| c.used_bytes()).unwrap_or(0)
    }

    pub fn cache_budget_bytes(&self) -> usize {
        self.cache.lock().map(|c| c.budget_bytes()).unwrap_or(0)
    }

    /// Provide a handle to the inner cache so the Python execute hook
    /// can share this Pipeline's cache instead of using a default one.
    pub fn cache_handle(&self) -> Arc<std::sync::Mutex<Cache>> {
        Arc::clone(&self.cache)
    }

    /// Build a Pipeline that shares the supplied cache.
    pub fn with_cache(cache: Arc<std::sync::Mutex<Cache>>) -> Self {
        Pipeline { cache }
    }
}
