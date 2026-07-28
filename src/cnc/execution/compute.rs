use std::any::Any;
use std::sync::Arc;

use crate::cnc::execution::callbacks::OpsCallbacksAdapter;
use crate::cnc::execution::callbacks::ScaledCallbacks;
use crate::ops::assembly::{AssembleCtx, Assembler, AssemblyOutput, Tracelet};
use crate::ops::part::{FaceState, Part, StockRegion};
use crate::ops::state::State;
use crate::ops::transform::{apply_transformers, Transformer};
use crate::pipeline::cache::CacheKey;
use crate::pipeline::compute::{Compute, ComputeCtx};
use crate::types::Polygon;

pub struct AssemblerCompute {
    pub assembler: Arc<dyn Assembler>,
    pub part: Part,
    pub face_id: String,
    pub transformers: Vec<Box<dyn Transformer>>,
    pub cut_state: State,
    /// Keys of upstream compute nodes whose `cleared_fragments`
    /// should be restored into this node's face before assembly.
    pub state_source_keys: Vec<String>,
    /// When set, temporarily replace the face's stock region with
    /// this boundary + islands before running, then restore the
    /// original after.  This lets each step see a different pocket
    /// subset while still sharing the same face's cleared area.
    pub region_boundary: Option<(Polygon, Vec<Polygon>)>,
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

        // Emit cut-state commands (SET_POWER, etc.) before the
        // assembler runs so they appear at the start of the ops.
        trace.apply_state(&self.cut_state);

        // Temporarily replace the face's stock region with a
        // per-step boundary if one is set.  This lets each step
        // see a different pocket subset (e.g. a single region)
        // while sharing the same face's cleared area.
        let saved_region = self.region_boundary.as_ref().map(|(bnd, isls)| {
            let saved = face.stock_region.clone();
            face.stock_region = StockRegion::new(bnd.clone(), isls.clone());
            saved
        });

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

        // Restore the original stock region.
        if let Some(saved) = saved_region {
            assemble_ctx.face.stock_region = saved;
        }

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
        let _ = self.part.face(&self.face_id);
        Some(CacheKey::new(tag))
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
            .unwrap_or_else(|| assembly.clone());
        if let Some(frags) = &restored.cleared_fragments {
            let face = self.part.face_mut(&self.face_id);
            face.cleared.set_fragments(frags.clone());
        }
        Ok(Box::new(restored))
    }

    fn prepare_cache_entry(
        &self,
        output: &(dyn Any + Send + Sync),
    ) -> Option<(Box<dyn Any + Send + Sync>, usize)> {
        let assembly = output.downcast_ref::<AssemblyOutput>()?;
        let face = self.part.face(&self.face_id);
        let cleared_fragments = face.map(|f| f.cleared.fragments().to_vec());
        let mut with_fragments = assembly.clone();
        with_fragments.cleared_fragments = cleared_fragments;
        let cached = self
            .assembler
            .store_cache(&with_fragments)
            .unwrap_or(with_fragments);
        let ops_heap = cached.ops.heap_size();
        let fragments_heap = cached.cleared_fragments.as_ref().map_or(0, |f| {
            let buf = f.capacity() * std::mem::size_of::<Polygon>();
            let vertices: usize = f.iter().map(|p| p.capacity()).sum::<usize>()
                * std::mem::size_of::<glam::DVec2>();
            buf + vertices
        });
        Some((
            Box::new(cached),
            std::mem::size_of::<AssemblyOutput>() + ops_heap + fragments_heap,
        ))
    }

    fn source_keys(&self) -> Vec<String> {
        self.state_source_keys.clone()
    }

    fn name(&self) -> &str {
        self.assembler.name()
    }
}
