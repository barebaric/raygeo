use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::pipeline::cache::CacheKey;
use crate::pipeline::callbacks::Callbacks;

pub type DepMap = HashMap<String, Arc<dyn Any + Send + Sync>>;

pub struct ComputeCtx<'a> {
    pub callbacks: &'a dyn Callbacks,
    pub deps: &'a DepMap,
}

impl<'a> ComputeCtx<'a> {
    pub fn new(callbacks: &'a dyn Callbacks, deps: &'a DepMap) -> Self {
        ComputeCtx { callbacks, deps }
    }
}

pub trait Compute: Send + Sync {
    fn run(
        &mut self,
        ctx: &mut ComputeCtx,
    ) -> Result<Box<dyn Any + Send + Sync>, String>;

    fn source_keys(&self) -> Vec<String> {
        Vec::new()
    }

    fn cache_key(&self, _tag: &str) -> Option<CacheKey> {
        None
    }

    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, String> {
        let _ = cached;
        Err("not cached".into())
    }

    fn prepare_cache_entry(
        &self,
        _output: &(dyn Any + Send + Sync),
    ) -> Option<Box<dyn Any + Send + Sync>> {
        None
    }

    fn name(&self) -> &str;
}
