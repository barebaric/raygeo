use std::any::Any;

use crate::cnc::execution::callbacks::OpsCallbacksAdapter;
use crate::cnc::execution::callbacks::ScaledCallbacks;
use crate::ops::assembly::{AssembleCtx, Assembler, AssemblyOutput, Tracelet};
use crate::ops::part::{FaceState, Part};
use crate::ops::state::State;
use crate::ops::transform::{
    apply_transformers, combine_cache_hashes, Transformer,
};
use crate::pipeline::cache::CacheKey;
use crate::pipeline::compute::{Compute, ComputeCtx};

pub struct AssemblerCompute {
    pub assembler: Box<dyn Assembler>,
    pub part: Part,
    pub face_id: String,
    pub transformers: Vec<Box<dyn Transformer>>,
    pub cut_state: State,
    /// Keys of upstream compute nodes whose `cleared_fragments`
    /// should be restored into this node's face before assembly.
    pub state_source_keys: Vec<String>,
}

impl Compute for AssemblerCompute {
    fn run(
        &mut self,
        ctx: &mut ComputeCtx,
    ) -> Result<Box<dyn Any + Send + Sync>, String> {
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }

        let mut trace = Tracelet::new();
        let face_id = self.face_id.clone();
        let size_mm = self.part.size_mm;
        let pixels_per_mm = self.part.pixels_per_mm;
        let image_source = self.part.image_source.as_deref();
        let face = self
            .part
            .faces
            .entry(face_id)
            .or_insert_with(|| FaceState::new(None));

        // Thread cleared state from upstream source nodes.
        for source_key in &self.state_source_keys {
            if let Some(dep) = ctx.deps.get(source_key) {
                if let Some(dep_output) = dep.downcast_ref::<AssemblyOutput>() {
                    if let Some(frags) = &dep_output.cleared_fragments {
                        if !frags.is_empty() {
                            face.cleared.set_fragments(frags.clone());
                        }
                    }
                }
            }
        }

        let adapter = OpsCallbacksAdapter {
            inner: ctx.callbacks,
        };
        let mut assemble_ctx = AssembleCtx {
            face,
            trace: &mut trace,
            state: &self.cut_state,
            callbacks: &adapter,
            size_mm,
            pixels_per_mm,
            image_source,
        };
        let meta = self.assembler.assemble(&mut assemble_ctx)?;

        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }

        let mut ops = trace.into_ops();

        if !self.transformers.is_empty() {
            let scaled = ScaledCallbacks::new(&adapter, 0.8, 0.2);
            apply_transformers(&mut ops, &mut self.transformers, &scaled)
                .map_err(|_| "cancelled".to_string())?;
        }

        ctx.callbacks.report_progress(1.0, "compute: done");

        let source_dimensions =
            if self.part.size_mm.0 > 0.0 && self.part.size_mm.1 > 0.0 {
                Some(self.part.size_mm)
            } else {
                None
            };

        let cleared_fragments = self.part.face(&self.face_id).and_then(|f| {
            let frags = f.cleared.fragments();
            if frags.is_empty() {
                None
            } else {
                Some(frags.to_vec())
            }
        });

        let output = AssemblyOutput {
            ops,
            is_scalable: self.assembler.is_scalable(),
            source_dimensions,
            cleared_fragments,
            meta,
        };
        Ok(Box::new(output))
    }

    fn cache_key(&self, tag: &str) -> Option<CacheKey> {
        let face = self.part.face(&self.face_id)?;
        let assembler_hash = self.assembler.cache_key_for_face(face)?;
        let transformer_hashes: Vec<u64> =
            self.transformers.iter().map(|t| t.cache_key()).collect();
        let combined =
            combine_cache_hashes(assembler_hash, &transformer_hashes);
        if self.state_source_keys.is_empty() {
            return Some(CacheKey::new(tag, combined));
        }
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        combined.hash(&mut h);
        self.state_source_keys.hash(&mut h);
        Some(CacheKey::new(tag, h.finish()))
    }

    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, String> {
        let assembly =
            cached.downcast_ref::<AssemblyOutput>().ok_or_else(|| {
                "cache type mismatch: expected AssemblyOutput".to_string()
            })?;
        let restored = self
            .assembler
            .restore_cache(assembly)
            .ok_or_else(|| "cache restore returned None".to_string())?;
        if let Some(frags) = &restored.cleared_fragments {
            let face = self.part.face_mut(&self.face_id);
            face.cleared.set_fragments(frags.clone());
        }
        Ok(Box::new(restored))
    }

    fn prepare_cache_entry(
        &self,
        output: &(dyn Any + Send + Sync),
    ) -> Option<Box<dyn Any + Send + Sync>> {
        let assembly = output.downcast_ref::<AssemblyOutput>()?;
        let face = self.part.face(&self.face_id);
        let cleared_fragments = face.map(|f| f.cleared.fragments().to_vec());
        let mut with_fragments = assembly.clone();
        with_fragments.cleared_fragments = cleared_fragments;
        if let Some(cached) = self.assembler.store_cache(&with_fragments) {
            return Some(Box::new(cached));
        }
        None
    }

    fn source_keys(&self) -> Vec<String> {
        self.state_source_keys.clone()
    }

    fn name(&self) -> &'static str {
        self.assembler.name()
    }
}
