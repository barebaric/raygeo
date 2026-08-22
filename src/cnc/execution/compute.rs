use std::any::Any;
use std::sync::Arc;

use crate::cnc::execution::callbacks::OpsCallbacksAdapter;
use crate::cnc::execution::callbacks::ScaledCallbacks;
use crate::geo::types::{Point3D, Polygon};
use crate::ops::assembly::{
    AssembleCtx, Assembler, AssemblyMeta, AssemblyOutput, AssemblyWarning,
    AssemblyWarningKind, Tracelet,
};
use crate::ops::material::spec::LaserPhysics;
use crate::ops::part::{FaceState, Part, StockRegion};
use crate::ops::state::State;
use crate::ops::transform::{apply_transformers, Transformer};
use crate::ops::types::ToolPose;
use crate::pipeline::cache::CacheKey;
use crate::pipeline::completed::PipelineError;
use crate::pipeline::compute::{Compute, ComputeCtx, DepMap};
use crate::prof::prof_report;

pub struct AssemblerCompute {
    pub assembler: Arc<dyn Assembler>,
    pub part: Part,
    pub face_id: String,
    pub transformers: Vec<Box<dyn Transformer>>,
    pub cut_state: State,
    /// Physical laser parameters for the burn fluence model. Defaults
    /// to a neutral fallback when the caller does not configure it.
    pub laser: LaserPhysics,
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

/// Outcome of running the assembler over every target face.
struct FaceRunResult {
    combined_start: Option<ToolPose>,
    combined_end: Option<ToolPose>,
    processed_face_ids: Vec<String>,
    warnings: Vec<AssemblyWarning>,
    material_effects: Vec<crate::ops::material::MaterialEffect>,
}

impl AssemblerCompute {
    /// Determine the target faces.
    ///
    /// A non-empty `face_id` processes only that face; an empty one
    /// processes every face of the part.
    fn resolve_face_ids(&self) -> (bool, Vec<String>) {
        let explicit_face = !self.face_id.is_empty();
        let face_ids = if explicit_face {
            vec![self.face_id.clone()]
        } else {
            self.part.face_ids_ordered()
        };
        (explicit_face, face_ids)
    }

    /// Thread cleared state from upstream source nodes into the target
    /// face.
    ///
    /// `AssemblyOutput.cleared_fragments` is a flat, unattributed list,
    /// so predecessors can only be restored to one specific face: the
    /// explicit `face_id` when set, otherwise the default face `""`
    /// (the largest pocket, the natural threading target).
    fn thread_upstream_state(&mut self, deps: &DepMap, target: &str) {
        if self.state_source_keys.is_empty() {
            return;
        }
        let face = self
            .part
            .faces
            .entry(target.to_string())
            .or_insert_with(|| FaceState::new(None));
        for source_key in &self.state_source_keys {
            if let Some(dep) = deps.get(source_key) {
                if let Some(dep_output) = dep.downcast_ref::<AssemblyOutput>() {
                    if let Some(frags) = &dep_output.cleared_fragments {
                        if !frags.is_empty() {
                            face.cleared.set_fragments(frags.clone());
                        }
                    }
                }
            }
        }
    }

    /// Assemble a single face.
    ///
    /// Lazy-inits the face (creating a fresh `FaceState` for an unknown
    /// id), temporarily replaces the face's stock region with a per-step
    /// boundary if one is set, runs the assembler, and restores the
    /// original region before returning.  This lets each step see a
    /// different pocket subset while sharing the same face's cleared
    /// area.
    #[allow(clippy::too_many_arguments)]
    fn assemble_face(
        &mut self,
        trace: &mut Tracelet,
        adapter: &OpsCallbacksAdapter,
        size_mm: (f64, f64),
        pixels_per_mm: Option<(f64, f64)>,
        fid: &str,
        warnings: &mut Vec<AssemblyWarning>,
        material_effects: &mut Vec<crate::ops::material::MaterialEffect>,
    ) -> Result<AssemblyMeta, String> {
        let image_source = self.part.image_source.as_deref();
        let face = self
            .part
            .faces
            .entry(fid.to_string())
            .or_insert_with(|| FaceState::new(None));

        let saved_region = self.region_boundary.as_ref().map(|(bnd, isls)| {
            let saved = face.stock_region.clone();
            face.stock_region = StockRegion::new(bnd.clone(), isls.clone());
            saved
        });

        let mut assemble_ctx = AssembleCtx {
            face,
            trace,
            state: &self.cut_state,
            callbacks: adapter,
            size_mm,
            pixels_per_mm,
            image_source,
            face_id: fid.to_string(),
            region_boundary: self.region_boundary.clone(),
            warnings,
            material_effects,
            laser: self.laser,
        };
        let result = self.assembler.assemble(&mut assemble_ctx);

        // Restore the original stock region before looking at the result,
        // so an error still leaves the face intact.
        if let Some(saved) = saved_region {
            assemble_ctx.face.stock_region = saved;
        }

        result
    }

    /// Run the assembler over every target face, folding the results
    /// into combined start/end poses, the processed face ids, and the
    /// accumulated warnings.
    fn run_assemblers(
        &mut self,
        trace: &mut Tracelet,
        adapter: &OpsCallbacksAdapter,
        size_mm: (f64, f64),
        pixels_per_mm: Option<(f64, f64)>,
        face_ids: &[String],
    ) -> Result<FaceRunResult, PipelineError> {
        let mut combined_start: Option<ToolPose> = None;
        let mut combined_end: Option<ToolPose> = None;
        let mut processed_face_ids: Vec<String> = Vec::new();
        let mut warnings: Vec<AssemblyWarning> = Vec::new();
        let mut material_effects: Vec<crate::ops::material::MaterialEffect> =
            Vec::new();

        for fid in face_ids {
            let result = self.assemble_face(
                trace,
                adapter,
                size_mm,
                pixels_per_mm,
                fid,
                &mut warnings,
                &mut material_effects,
            );
            match result {
                Ok(meta) => {
                    if combined_start.is_none() {
                        combined_start = Some(meta.start);
                    }
                    combined_end = Some(meta.end);
                    processed_face_ids.push(fid.clone());
                }
                Err(e) if e == "cancelled" => {
                    return Err(PipelineError::Cancelled);
                }
                Err(e) => {
                    // Don't fail the whole part yet — warn and continue
                    // to the next face. Partial ops already emitted into
                    // the shared trace are kept. If *every* attempted
                    // face fails, the all-failed check in `run` below
                    // turns this into a hard error so the pipeline's
                    // failure cascade still fires (existing
                    // `test_pipeline_failure_propagation` contract).
                    // Recovery is only for partial success.
                    warnings.push(AssemblyWarning {
                        kind: AssemblyWarningKind::FaceFailed,
                        face_id: fid.clone(),
                        region: None,
                        detail: e,
                    });
                }
            }
        }

        Ok(FaceRunResult {
            combined_start,
            combined_end,
            processed_face_ids,
            warnings,
            material_effects,
        })
    }

    /// Collect cleared fragments from every face that was actually
    /// processed (single-face mode: just that face; multi-face mode:
    /// the union across all faces). Empty when nothing was cleared.
    fn collect_cleared_fragments(
        &self,
        processed_face_ids: &[String],
    ) -> Option<Vec<Polygon>> {
        let mut cleared_fragments: Vec<Polygon> = Vec::new();
        for fid in processed_face_ids {
            if let Some(f) = self.part.face(fid) {
                let frags = f.cleared.fragments();
                if !frags.is_empty() {
                    cleared_fragments.extend(frags.iter().cloned());
                }
            }
        }
        if cleared_fragments.is_empty() {
            None
        } else {
            Some(cleared_fragments)
        }
    }
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

        let (explicit_face, face_ids) = self.resolve_face_ids();

        let adapter = OpsCallbacksAdapter {
            inner: ctx.callbacks,
        };

        let mut trace = Tracelet::new();
        // Emit cut-state commands (SET_POWER, etc.) once, before the
        // first assembler runs, so they appear at the start of the ops.
        trace.apply_state(&self.cut_state);

        let target = if explicit_face {
            self.face_id.clone()
        } else {
            String::new()
        };
        self.thread_upstream_state(ctx.deps, &target);

        let result = self.run_assemblers(
            &mut trace,
            &adapter,
            size_mm,
            pixels_per_mm,
            &face_ids,
        )?;
        let FaceRunResult {
            combined_start,
            combined_end,
            processed_face_ids,
            warnings,
            material_effects,
        } = result;

        if ctx.callbacks.is_cancelled() {
            return Err(PipelineError::Cancelled);
        }

        // Every attempted face failed (no successful `Ok(meta)`): surface
        // a hard error instead of an empty success, so the scheduler
        // reattaches this node with `error` and `output = None` and
        // propagates the synthetic "upstream failed" to dependents.
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

        let cleared_fragments =
            self.collect_cleared_fragments(&processed_face_ids);

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
            material_effects: if material_effects.is_empty() {
                None
            } else {
                Some(material_effects)
            },
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
        let effects_heap = cached.material_effects.as_ref().map_or(0, |e| {
            e.len()
                * std::mem::size_of::<crate::ops::material::MaterialEffect>()
                + e.iter().map(|fx| fx.heap_size()).sum::<usize>()
        });
        let total = struct_size + ops_heap + fragments_heap + effects_heap;
        Some((Box::new(cached), total))
    }

    fn source_keys(&self) -> Vec<String> {
        self.state_source_keys.clone()
    }

    fn name(&self) -> &str {
        self.assembler.name()
    }
}
