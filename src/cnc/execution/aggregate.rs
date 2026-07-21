use std::any::Any;
use std::collections::HashSet;

use glam::DMat4;

use crate::cnc::execution::callbacks::{OpsCallbacksAdapter, ScaledCallbacks};
use crate::cnc::execution::specs::{
    AggregateGroup, AggregateInput, AggregateOutput, AggregateSpec, LinkMode,
    Marker,
};
use crate::ops::assembly::AssemblyOutput;
use crate::ops::container::Ops;
use crate::ops::transform::apply_transformers;
use crate::pipeline::aggregate::{Aggregate, AggregateCtx, DepMap};
use crate::pipeline::cache::CacheKey;
use crate::types::Point3D;

pub struct OpsAggregate {
    pub spec: AggregateSpec,
}

impl OpsAggregate {
    fn emit_marker(ops: &mut Ops, marker: &Marker) {
        match marker {
            Marker::JobStart => ops.job_start(),
            Marker::JobEnd => ops.job_end(),
            Marker::LayerStart { uid } => ops.layer_start(uid),
            Marker::LayerEnd { uid } => ops.layer_end(uid),
            Marker::WorkpieceStart { uid } => ops.workpiece_start(uid),
            Marker::WorkpieceEnd { uid } => ops.workpiece_end(uid),
        }
    }

    fn pull_input(
        deps: &DepMap,
        input: &AggregateInput,
    ) -> Result<Ops, String> {
        let upstream = deps.get(&input.source_key).ok_or_else(|| {
            format!("missing dependency: {}", input.source_key)
        })?;

        let ops = if let Some(assembly) =
            upstream.downcast_ref::<AssemblyOutput>()
        {
            let mut cloned = assembly.ops.copy();
            if assembly.is_scalable {
                if let Some((src_w, src_h)) = assembly.source_dimensions {
                    let (tgt_w, tgt_h) = input.target_dimensions;
                    if tgt_w > 0.0
                        && tgt_h > 0.0
                        && (src_w - tgt_w).abs() > 1e-9
                        && (src_h - tgt_h).abs() > 1e-9
                    {
                        let sx = tgt_w / src_w;
                        let sy = tgt_h / src_h;
                        let s = sx.min(sy);
                        if (s - 1.0).abs() > 1e-9 {
                            let scale =
                                DMat4::from_scale(glam::DVec3::new(s, s, 1.0));
                            cloned.transform(scale);
                        }
                    }
                }
            }
            let m = array_to_dmat4(&input.placement_matrix);
            if !is_identity(&m) {
                cloned.transform(m);
            }
            cloned
        } else if let Some(agg) = upstream.downcast_ref::<AggregateOutput>() {
            agg.ops.copy()
        } else {
            return Err(format!(
                "cannot aggregate output for dep: {}",
                input.source_key
            ));
        };
        Ok(ops)
    }

    fn pull_meta_pos(
        deps: &DepMap,
        input: &AggregateInput,
    ) -> Option<(Point3D, Point3D)> {
        let meta = deps
            .get(&input.source_key)?
            .downcast_ref::<AssemblyOutput>()
            .map(|ao| ao.meta.clone())?;
        Some((
            world_pos(meta.start.pos, &input.placement_matrix),
            world_pos(meta.end.pos, &input.placement_matrix),
        ))
    }

    fn emit_link(ops: &mut Ops, from: Point3D, to: Point3D, safe_z: f64) {
        let entry_z = to.z;
        // Retract
        ops.move_to(from.x, from.y, safe_z, None);
        // XY travel (if XY differs)
        if (to.x - from.x).abs() > 1e-12 || (to.y - from.y).abs() > 1e-12 {
            ops.move_to(to.x, to.y, safe_z, None);
        }
        // Plunge
        if (entry_z - safe_z).abs() > 1e-12 {
            ops.move_to(to.x, to.y, entry_z, None);
        }
    }

    fn emit_group(
        deps: &DepMap,
        ops: &mut Ops,
        callbacks: &dyn crate::ops::callbacks::Callbacks,
        group: &AggregateGroup,
    ) -> Result<(), String> {
        for marker in &group.start_markers {
            Self::emit_marker(ops, marker);
        }

        let mut prev_end_world: Option<Point3D> = None;
        for input in &group.inputs {
            if callbacks.is_cancelled() {
                return Err("cancelled".to_string());
            }

            // Look up world-space start/end positions for this input
            let world_positions = Self::pull_meta_pos(deps, input);

            // Emit link before this input (skip first)
            if let Some(pe) = prev_end_world {
                if let LinkMode::Sequential { safe_z } = &group.link_mode {
                    if let Some((start_world, _)) = world_positions {
                        Self::emit_link(ops, pe, start_world, *safe_z);
                    }
                }
            }

            let upstream = Self::pull_input(deps, input)?;

            // Capture world-space end position for next link
            if let Some((_, end_world)) = world_positions {
                prev_end_world = Some(end_world);
            }

            ops.extend(&upstream);
        }

        // Final lift after last input
        if let Some(pe) = prev_end_world {
            if let LinkMode::Sequential { safe_z } = &group.link_mode {
                if pe.z < *safe_z - 1e-12 {
                    ops.move_to(pe.x, pe.y, *safe_z, None);
                }
            }
        }

        for marker in &group.end_markers {
            Self::emit_marker(ops, marker);
        }
        Ok(())
    }
}

impl Aggregate for OpsAggregate {
    fn run(
        &mut self,
        ctx: &mut AggregateCtx,
        deps: &DepMap,
    ) -> Result<Box<dyn Any + Send + Sync>, String> {
        let adapter = OpsCallbacksAdapter {
            inner: ctx.callbacks,
        };
        let total_groups = self.spec.groups.len() as f64;
        let mut ops = Ops::new();

        for marker in &self.spec.wrap_start {
            Self::emit_marker(&mut ops, marker);
        }

        for (i, group) in self.spec.groups.iter().enumerate() {
            if ctx.callbacks.is_cancelled() {
                return Err("cancelled".to_string());
            }
            let frac = if total_groups > 0.0 {
                i as f64 / total_groups
            } else {
                0.0
            };
            ctx.callbacks.report_progress(
                frac,
                &format!(
                    "aggregate: group {}/{}",
                    i + 1,
                    self.spec.groups.len()
                ),
            );
            Self::emit_group(deps, &mut ops, &adapter, group)?;
        }

        for marker in &self.spec.wrap_end {
            Self::emit_marker(&mut ops, marker);
        }

        if !self.spec.transformers.is_empty() {
            if ctx.callbacks.is_cancelled() {
                return Err("cancelled".to_string());
            }
            let scaled = ScaledCallbacks::new(&adapter, 0.8, 0.2);
            apply_transformers(&mut ops, &mut self.spec.transformers, &scaled)
                .map_err(|_| "cancelled".to_string())?;
        }

        let time_estimate = {
            let mp = &self.spec.machine;
            if mp.default_feed_rate > 0.0 || mp.default_rapid_rate > 0.0 {
                let mut ops_for_time = ops.copy();
                Some(ops_for_time.estimate_time(
                    mp.default_feed_rate,
                    mp.default_rapid_rate,
                    mp.acceleration,
                ))
            } else {
                None
            }
        };

        ctx.callbacks.report_progress(1.0, "aggregate: done");
        Ok(Box::new(AggregateOutput { ops, time_estimate }))
    }

    fn source_keys(&self) -> Vec<String> {
        let mut keys = HashSet::new();
        for group in &self.spec.groups {
            for input in &group.inputs {
                keys.insert(input.source_key.clone());
            }
        }
        keys.into_iter().collect()
    }

    fn cache_key(&self, tag: &str) -> Option<CacheKey> {
        Some(CacheKey::new(tag))
    }

    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, String> {
        let output =
            cached.downcast_ref::<AggregateOutput>().ok_or_else(|| {
                "cache type mismatch: expected AggregateOutput".to_string()
            })?;
        Ok(Box::new(output.clone()))
    }

    fn prepare_cache_entry(
        &self,
        output: &(dyn Any + Send + Sync),
    ) -> Option<Box<dyn Any + Send + Sync>> {
        let output = output.downcast_ref::<AggregateOutput>()?;
        Some(Box::new(output.clone()))
    }

    fn name(&self) -> &'static str {
        "aggregate"
    }
}

fn array_to_dmat4(arr: &[[f64; 4]; 4]) -> DMat4 {
    DMat4::from_cols_array_2d(&[
        [arr[0][0], arr[1][0], arr[2][0], arr[3][0]],
        [arr[0][1], arr[1][1], arr[2][1], arr[3][1]],
        [arr[0][2], arr[1][2], arr[2][2], arr[3][2]],
        [arr[0][3], arr[1][3], arr[2][3], arr[3][3]],
    ])
}

fn is_identity(m: &DMat4) -> bool {
    let id = DMat4::IDENTITY;
    m.abs_diff_eq(id, 1e-12)
}

/// Transform a point from assembler-local space to world space using the
/// input's placement matrix. Returns the point unchanged when the matrix
/// is identity.
fn world_pos(p: Point3D, placement: &[[f64; 4]; 4]) -> Point3D {
    let m = array_to_dmat4(placement);
    if is_identity(&m) {
        p
    } else {
        m.transform_point3(p)
    }
}
