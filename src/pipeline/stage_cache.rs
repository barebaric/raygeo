//! Cache-facing adapter over the pipeline stage traits.
//!
//! [`Compute`] and [`Aggregate`] expose the same cache methods with
//! identical signatures, so the dispatch sequence (read cache → run →
//! write cache) is written once here against a private [`StageCache`]
//! adapter and shared by both stage kinds.

use std::any::Any;
use std::sync::{Arc, Mutex};

use crate::pipeline::aggregate::{Aggregate, AggregateCtx, DepMap};
use crate::pipeline::cache::{Cache, CacheKey};
use crate::pipeline::callbacks::Callbacks;
use crate::pipeline::completed::PipelineError;
use crate::pipeline::compute::{Compute, ComputeCtx};

/// Cache-facing adapter over the compute and aggregate stages.
pub(crate) trait StageCache {
    fn cache_key(&self, tag: &str) -> Option<CacheKey>;
    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError>;
    fn run(
        &mut self,
        callbacks: &dyn Callbacks,
        deps: &DepMap,
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError>;
    fn prepare_cache_entry(
        &self,
        out: &(dyn Any + Send + Sync),
    ) -> Option<(Box<dyn Any + Send + Sync>, usize)>;
}

impl StageCache for Box<dyn Compute> {
    fn cache_key(&self, tag: &str) -> Option<CacheKey> {
        (**self).cache_key(tag)
    }

    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
        (**self).restore_from_cache(cached)
    }

    fn run(
        &mut self,
        callbacks: &dyn Callbacks,
        deps: &DepMap,
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
        let mut ctx = ComputeCtx { callbacks, deps };
        (**self).run(&mut ctx)
    }

    fn prepare_cache_entry(
        &self,
        out: &(dyn Any + Send + Sync),
    ) -> Option<(Box<dyn Any + Send + Sync>, usize)> {
        (**self).prepare_cache_entry(out)
    }
}

impl StageCache for Box<dyn Aggregate> {
    fn cache_key(&self, tag: &str) -> Option<CacheKey> {
        (**self).cache_key(tag)
    }

    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
        (**self).restore_from_cache(cached)
    }

    fn run(
        &mut self,
        callbacks: &dyn Callbacks,
        deps: &DepMap,
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
        let mut ctx = AggregateCtx::new(callbacks);
        (**self).run(&mut ctx, deps)
    }

    fn prepare_cache_entry(
        &self,
        out: &(dyn Any + Send + Sync),
    ) -> Option<(Box<dyn Any + Send + Sync>, usize)> {
        (**self).prepare_cache_entry(out)
    }
}

/// Try to restore the stage output from the cache.
///
/// Returns ``Ok(None)`` when the stage should run: the node is not
/// cacheable, has no cache key, the entry is missing, or restoring it
/// fails. A poisoned cache lock is an error (as before).
fn try_read_cache(
    cache: &Arc<Mutex<Cache>>,
    cacheable: bool,
    cache_key: &Option<CacheKey>,
    stage: &mut dyn StageCache,
) -> Result<Option<Box<dyn Any + Send + Sync>>, PipelineError> {
    if !cacheable {
        return Ok(None);
    }
    let Some(key) = cache_key else {
        return Ok(None);
    };
    let mut c = cache.lock().map_err(|_| PipelineError::CacheLockPoisoned)?;
    let Some(cached) = c.get(key) else {
        return Ok(None);
    };
    match stage.restore_from_cache(&**cached) {
        Ok(out) => Ok(Some(out)),
        Err(_) => Ok(None),
    }
}

/// Store the stage output into the cache.
///
/// Silently skips when the node is not cacheable, the run failed, there
/// is no cache key or entry, or the cache lock is poisoned (as before).
/// A store that would exceed the byte budget is an error.
fn store_result(
    cache: &Arc<Mutex<Cache>>,
    cacheable: bool,
    cache_key: Option<CacheKey>,
    result: &Result<Box<dyn Any + Send + Sync>, PipelineError>,
    stage: &dyn StageCache,
) -> Result<(), PipelineError> {
    if !cacheable {
        return Ok(());
    }
    let Ok(ref out) = result else {
        return Ok(());
    };
    let Some(key) = cache_key else {
        return Ok(());
    };
    let Some((entry, size)) = stage.prepare_cache_entry(out.as_ref()) else {
        return Ok(());
    };
    let Ok(mut c) = cache.lock() else {
        return Ok(());
    };
    let node_tag = key.tag.clone();
    if c.insert(key, entry, size) {
        Ok(())
    } else {
        Err(PipelineError::CacheBudgetExceeded {
            node_key: node_tag,
            size,
            budget: c.budget_bytes(),
        })
    }
}

/// Run a stage through the cache sequence (read → run → write).
pub(crate) fn dispatch_cached(
    stage: &mut dyn StageCache,
    callbacks: &dyn Callbacks,
    cache: &Arc<Mutex<Cache>>,
    node_key: &str,
    deps: &DepMap,
    cacheable: bool,
) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
    let cache_key = stage.cache_key(node_key);
    if let Some(out) = try_read_cache(cache, cacheable, &cache_key, stage)? {
        return Ok(out);
    }
    let result = stage.run(callbacks, deps);
    store_result(cache, cacheable, cache_key, &result, stage)?;
    result
}
