use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::pipeline::callbacks::Callbacks;

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
    ) -> Result<Box<dyn Any + Send + Sync>, String>;

    fn source_keys(&self) -> Vec<String>;

    fn name(&self) -> &'static str;
}
