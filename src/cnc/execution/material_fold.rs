//! Material fold compute node — folds upstream ``AssemblyOutput``'s
//! ``material_effects`` against one stock into a ``MaterialState``
//! snapshot. See ``ops::material::fold`` for the kernel; this is the
//! ``pipeline::Compute`` glue that runs it inside ``run_intent``'s
//! ``rayon::scope``, alongside assembler compute nodes.

use std::any::Any;

use crate::ops::assembly::AssemblyOutput;
use crate::ops::material::fold;
use crate::ops::material::spec::MaterialFoldSpec;
use crate::ops::material::state::MaterialState;
use crate::pipeline::cache::CacheKey;
use crate::pipeline::completed::PipelineError;
use crate::pipeline::compute::{Compute, ComputeCtx};

/// A ``Compute`` implementation that folds upstream material effects
/// into one stock's ``MaterialState``.
///
/// The spec is a *template* at build time: ``entries[i].effects`` is
/// empty, and is populated from each entry's upstream
/// ``AssemblyOutput.material_effects`` in :meth:`Compute::run`. The
/// fold kernel skips entries whose effects are still empty, so
/// upstreams that produced ``None`` are naturally ignored.
pub struct MaterialFoldCompute {
    /// The full fold spec (stock shape, entries with placements,
    /// grid budget). ``entries[i].effects`` is empty at build time
    /// and populated from upstream outputs in :meth:`Compute::run`.
    pub spec: MaterialFoldSpec,
    /// Source keys of upstream compute nodes whose
    /// ``AssemblyOutput.material_effects`` this fold consumes. Used
    /// by :meth:`Compute::source_keys` for dependency wiring.
    pub source_keys: Vec<String>,
}

impl Compute for MaterialFoldCompute {
    fn run(
        &mut self,
        ctx: &mut ComputeCtx,
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
        if ctx.callbacks.is_cancelled() {
            return Err(PipelineError::Cancelled);
        }
        // Populate each entry's effects from upstream outputs.
        for entry in &mut self.spec.entries {
            if let Some(dep) = ctx.deps.get(&entry.source_key) {
                if let Some(upstream) = dep.downcast_ref::<AssemblyOutput>() {
                    if let Some(effects) = &upstream.material_effects {
                        entry.effects = effects.clone();
                    }
                }
            }
        }
        let state = fold::fold_effects(&self.spec)
            .map_err(|e| PipelineError::Other(e.to_string()))?;
        ctx.callbacks.report_progress(1.0, "fold: done");
        Ok(Box::new(state))
    }

    fn cache_key(&self, tag: &str) -> Option<CacheKey> {
        Some(CacheKey::new(tag))
    }

    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
        let state =
            cached.downcast_ref::<MaterialState>().ok_or_else(|| {
                PipelineError::Other(
                    "cache type mismatch: expected MaterialState".into(),
                )
            })?;
        Ok(Box::new(state.clone()))
    }

    fn prepare_cache_entry(
        &self,
        output: &(dyn Any + Send + Sync),
    ) -> Option<(Box<dyn Any + Send + Sync>, usize)> {
        let state = output.downcast_ref::<MaterialState>()?;
        let voids_heap = state
            .void_polygons
            .iter()
            .map(|p| p.len() * std::mem::size_of::<crate::geo::types::Point>())
            .sum::<usize>();
        let surface_heap =
            state.surface_map.as_ref().map_or(0, |c| c.data.len());
        let depth_heap = state.depth_field.as_ref().map_or(0, |c| c.data.len());
        let total = std::mem::size_of::<MaterialState>()
            + voids_heap
            + surface_heap
            + depth_heap;
        Some((Box::new(state.clone()), total))
    }

    fn source_keys(&self) -> Vec<String> {
        self.source_keys.clone()
    }

    fn name(&self) -> &str {
        "material_fold"
    }
}
