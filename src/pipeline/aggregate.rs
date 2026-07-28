use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::pipeline::cache::CacheKey;
use crate::pipeline::callbacks::Callbacks;
use crate::pipeline::completed::PipelineError;

pub type DepMap = HashMap<String, Arc<dyn Any + Send + Sync>>;

pub struct AggregateCtx<'a> {
    pub callbacks: &'a dyn Callbacks,
}

impl<'a> AggregateCtx<'a> {
    pub fn new(callbacks: &'a dyn Callbacks) -> Self {
        AggregateCtx { callbacks }
    }
}

pub trait Aggregate: Send + Sync {
    fn run(
        &mut self,
        ctx: &mut AggregateCtx,
        deps: &DepMap,
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError>;

    fn source_keys(&self) -> Vec<String>;

    fn cache_key(&self, _tag: &str) -> Option<CacheKey> {
        None
    }

    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
        let _ = cached;
        Err(PipelineError::Other("not cached".into()))
    }

    fn prepare_cache_entry(
        &self,
        _output: &(dyn Any + Send + Sync),
    ) -> Option<(Box<dyn Any + Send + Sync>, usize)> {
        None
    }

    fn name(&self) -> &str;
}
