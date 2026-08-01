use std::any::Any;
use std::sync::Arc;

use crate::cnc::execution::callbacks::OpsCallbacksAdapter;
use crate::cnc::execution::callbacks::ScaledCallbacks;
use crate::ops::assembly::{
    AssembleCtx, Assembler, AssemblyMeta, AssemblyOutput, AssemblyWarning,
    AssemblyWarningKind, Tracelet,
};
use crate::ops::part::{FaceState, Part, StockRegion};
use crate::ops::state::State;
use crate::ops::transform::{apply_transformers, Transformer};
use crate::ops::types::ToolPose;
use crate::pipeline::cache::CacheKey;
use crate::pipeline::completed::PipelineError;
use crate::pipeline::compute::{Compute, ComputeCtx};
use crate::prof::prof_report;
use crate::types::{Point3D, Polygon};

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
    /// Print a profiling report to stdout once the node's faces have
    /// all been assembled.  Emitted from `run()` so it runs on the
    /// same rayon worker that executed the assemblers, where the
    /// thread-local profiler data lives.
    pub profile: bool,
}

impl Compute for AssemblerCompute {
    fn run(
        &mut self,
        ctx: &mut ComputeCtx,
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
        if ctx.callbacks.is_cancelled() {
            return Err(PipelineError::Cancelled);
        }

        let size_mm = self.part.size_mm;
        let pixels_per_mm = self.part.pixels_per_mm;
        let image_source = self.part.image_source.as_deref();

        // Determine target faces. An explicit (non-empty) `face_id`
        // processes only that face (the historical single-face
        // behaviour, including upstream state threading). An empty
        // `face_id` iterates every face of the part — the multi-face
        // path introduced for multi-pocket parts.
        let explicit_face = !self.face_id.is_empty();
        let face_ids: Vec<String> = if explicit_face {
            vec![self.face_id.clone()]
        } else {
            self.part.faces.keys().cloned().collect()
        };

        let adapter = OpsCallbacksAdapter {
            inner: ctx.callbacks,
        };

        let mut trace = Tracelet::new();
        // Emit cut-state commands (SET_POWER, etc.) once, before the
        // first assembler runs, so they appear at the start of the ops.
        trace.apply_state(&self.cut_state);

        // Thread cleared state from upstream source nodes into the
        // target face, exactly as the historic single-face path did.
        //
        // `AssemblyOutput.cleared_fragments` is a flat, unattributed
        // list, so predecessors can only be restored to one specific
        // face: the explicit `face_id` when set, otherwise the default
        // face `""`. (For a single-face part this is identical to the
        // old behaviour; for a multi-face part the default face — the
        // largest pocket — is the natural threading target.)
        if !self.state_source_keys.is_empty() {
            let target = if explicit_face {
                self.face_id.clone()
            } else {
                String::new()
            };
            let face = self
                .part
                .faces
                .entry(target)
                .or_insert_with(|| FaceState::new(None));
            for source_key in &self.state_source_keys {
                if let Some(dep) = ctx.deps.get(source_key) {
                    if let Some(dep_output) =
                        dep.downcast_ref::<AssemblyOutput>()
                    {
                        if let Some(frags) = &dep_output.cleared_fragments {
                            if !frags.is_empty() {
                                face.cleared.set_fragments(frags.clone());
                            }
                        }
                    }
                }
            }
        }

        let mut warnings: Vec<AssemblyWarning> = Vec::new();
        let mut combined_start: Option<ToolPose> = None;
        let mut combined_end: Option<ToolPose> = None;
        let mut processed_face_ids: Vec<String> = Vec::new();

        for fid in face_ids {
            // Lazy-init the face (matches the single-face `entry`
            // behaviour for an unknown id).
            let face = self
                .part
                .faces
                .entry(fid.clone())
                .or_insert_with(|| FaceState::new(None));

            // Temporarily replace the face's stock region with a
            // per-step boundary if one is set.  This lets each step
            // see a different pocket subset (e.g. a single region)
            // while sharing the same face's cleared area.
            let saved_region =
                self.region_boundary.as_ref().map(|(bnd, isls)| {
                    let saved = face.stock_region.clone();
                    face.stock_region =
                        StockRegion::new(bnd.clone(), isls.clone());
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
                face_id: fid.clone(),
                region_boundary: self.region_boundary.clone(),
                warnings: &mut warnings,
            };
            let result = self.assembler.assemble(&mut assemble_ctx);

            // Restore the original stock region before looking at the
            // result, so an error still leaves the face intact.
            if let Some(saved) = saved_region {
                assemble_ctx.face.stock_region = saved;
            }

            match result {
                Ok(meta) => {
                    if combined_start.is_none() {
                        combined_start = Some(meta.start);
                    }
                    combined_end = Some(meta.end);
                    processed_face_ids.push(fid);
                }
                Err(e) if e == "cancelled" => {
                    return Err(PipelineError::Cancelled);
                }
                Err(e) => {
                    // Don't fail the whole part yet — warn and continue
                    // to the next face. Partial ops already emitted into
                    // the shared trace are kept. If *every* attempted face
                    // fails, the loop-bottom check below turns this into a
                    // hard error so the pipeline's failure cascade still
                    // fires (existing `test_pipeline_failure_propagation`
                    // contract). Recovery is only for partial success.
                    warnings.push(AssemblyWarning {
                        kind: AssemblyWarningKind::FaceFailed,
                        face_id: fid.clone(),
                        region: None,
                        detail: e,
                    });
                }
            }
        }

        if ctx.callbacks.is_cancelled() {
            return Err(PipelineError::Cancelled);
        }

        // Every attempted face failed (no successful `Ok(meta)`): surface
        // a hard error instead of an empty success, so the scheduler
        // reattaches this node with `error` and `output = None` and
        // propagates the synthetic "upstream failed" to dependents.
        // Matches the pre-multi-face behaviour of `assemble()?`.
        if processed_face_ids.is_empty() && !warnings.is_empty() {
            let detail = warnings
                .iter()
                .map(|w| w.detail.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(PipelineError::Other(detail));
        }

        let mut ops = trace.into_ops();

        if !self.transformers.is_empty() {
            let scaled = ScaledCallbacks::new(&adapter, 0.8, 0.2);
            apply_transformers(&mut ops, &mut self.transformers, &scaled)
                .map_err(|_| PipelineError::Cancelled)?;
        }

        ctx.callbacks.report_progress(1.0, "compute: done");

        let source_dimensions =
            if self.part.size_mm.0 > 0.0 && self.part.size_mm.1 > 0.0 {
                Some(self.part.size_mm)
            } else {
                None
            };

        // Collect cleared fragments from every face that was actually
        // processed (single-face mode: just that face; multi-face mode:
        // the union across all faces). Empty when nothing was cleared.
        let mut cleared_fragments: Vec<Polygon> = Vec::new();
        for fid in &processed_face_ids {
            if let Some(f) = self.part.face(fid) {
                let frags = f.cleared.fragments();
                if !frags.is_empty() {
                    cleared_fragments.extend(frags.iter().cloned());
                }
            }
        }
        let cleared_fragments = if cleared_fragments.is_empty() {
            None
        } else {
            Some(cleared_fragments)
        };

        let meta = AssemblyMeta {
            start: combined_start.unwrap_or(ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            }),
            end: combined_end.unwrap_or(ToolPose {
                pos: Point3D::ZERO,
                heading: 0.0,
            }),
        };

        let output = AssemblyOutput {
            ops,
            is_scalable: self.assembler.is_scalable(),
            source_dimensions,
            cleared_fragments,
            meta,
            warnings,
        };
        if self.profile {
            prof_report();
        }
        Ok(Box::new(output))
    }

    fn cache_key(&self, tag: &str) -> Option<CacheKey> {
        let _ = self.part.face(&self.face_id);
        Some(CacheKey::new(tag))
    }

    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
        let output =
            cached.downcast_ref::<AssemblyOutput>().ok_or_else(|| {
                PipelineError::Other(
                    "cache type mismatch: expected AssemblyOutput".into(),
                )
            })?;
        let restored = self
            .assembler
            .restore_cache(output)
            .unwrap_or_else(|| output.clone());
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
        let struct_size = std::mem::size_of::<AssemblyOutput>();
        let fragments_heap = cached.cleared_fragments.as_ref().map_or(0, |f| {
            let buf = f.len() * std::mem::size_of::<Polygon>();
            let vertices: usize = f.iter().map(|p| p.len()).sum::<usize>()
                * std::mem::size_of::<glam::DVec2>();
            buf + vertices
        });
        let total = struct_size + ops_heap + fragments_heap;
        Some((Box::new(cached), total))
    }

    fn source_keys(&self) -> Vec<String> {
        self.state_source_keys.clone()
    }

    fn name(&self) -> &str {
        self.assembler.name()
    }
}
