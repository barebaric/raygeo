use std::any::Any;
use std::sync::Arc;

use glam::{DMat4, DVec4};

use crate::cnc::execution::specs::{
    AggregateOutput, MachineTransformSpec, RotaryMappingSpec,
};
use crate::ops::assembly::AssemblyOutput;
use crate::ops::axis::Axis;
use crate::ops::container::Ops;
use crate::ops::types::{MarkerCmd, MoveCmd, OpCategory};
use crate::pipeline::cache::CacheKey;
use crate::pipeline::compute::{Compute, ComputeCtx};
use crate::types::Point3D;

pub struct MachineTransformCompute {
    pub spec: MachineTransformSpec,
}

impl MachineTransformCompute {
    fn apply_rotary_mapping(&self, ops: &mut Ops) {
        let layer_map: std::collections::HashMap<&str, &RotaryMappingSpec> =
            self.spec
                .rotary_mappings
                .iter()
                .map(|rm| (rm.layer_uid.as_str(), rm))
                .collect();
        if layer_map.is_empty() {
            return;
        }

        let mut current_layer: Option<Arc<str>> = None;
        let mut i = 0;
        while i < ops.commands.len() {
            // Track layer markers.
            if let OpCategory::Marker(cmd) = &ops.commands[i].category {
                match cmd {
                    MarkerCmd::LayerStart(uid) => {
                        current_layer = Some(uid.clone())
                    }
                    MarkerCmd::LayerEnd(_) => current_layer = None,
                    _ => {}
                }
                i += 1;
                continue;
            }

            if !ops.commands[i].is_moving() {
                i += 1;
                continue;
            }

            let layer_uid = match &current_layer {
                Some(uid) => uid.as_ref(),
                None => {
                    i += 1;
                    continue;
                }
            };
            let rm = match layer_map.get(layer_uid) {
                Some(rm) => rm,
                None => {
                    i += 1;
                    continue;
                }
            };

            // Snapshot extra_axes before the mutable borrow.
            let mut ea_vec: Vec<(Axis, f64)> = ops.commands[i]
                .extra_axes()
                .map(|ea| ea.to_vec())
                .unwrap_or_default();
            let rotary_axis =
                Axis::from_str_name(&rm.rotary_axis).unwrap_or(Axis::A);

            // Mutate endpoint and control points.
            {
                let node = &mut ops.commands[i];
                if let OpCategory::Moving { end, cmd } = &mut node.category {
                    let degrees = mu_to_degrees(
                        end.y,
                        rm.diameter,
                        rm.gear_ratio,
                        rm.reverse,
                    );

                    ea_vec.retain(|(a, _)| *a != rotary_axis);
                    ea_vec.push((rotary_axis, degrees));

                    end.y = rm.axis_position_3d[1] + end.x * rm.cylinder_dir[1];

                    if let Some(ref replaced) = rm.replaced_axis {
                        zero_axis(end, replaced);
                    }

                    match cmd {
                        MoveCmd::BezierTo { control1, control2 } => {
                            control1.y = mu_to_degrees(
                                control1.y,
                                rm.diameter,
                                rm.gear_ratio,
                                rm.reverse,
                            );
                            control2.y = mu_to_degrees(
                                control2.y,
                                rm.diameter,
                                rm.gear_ratio,
                                rm.reverse,
                            );
                        }
                        MoveCmd::QuadraticBezierTo { control } => {
                            control.y = mu_to_degrees(
                                control.y,
                                rm.diameter,
                                rm.gear_ratio,
                                rm.reverse,
                            );
                        }
                        MoveCmd::ArcTo { center, .. } => {
                            center.y = mu_to_degrees(
                                center.y,
                                rm.diameter,
                                rm.gear_ratio,
                                rm.reverse,
                            );
                        }
                        _ => {}
                    }
                }
            }

            // Set extra_axes (mutable borrow released).
            if ea_vec.is_empty() {
                ops.commands[i].clear_extra_axes();
            } else {
                ops.commands[i].set_extra_axes(Arc::from(ea_vec));
            }

            i += 1;
        }

        ops.invalidate_time_cache();
    }

    fn apply_axis_replacement(&self, ops: &mut Ops) {
        let layer_map: std::collections::HashMap<&str, &RotaryMappingSpec> =
            self.spec
                .rotary_mappings
                .iter()
                .filter(|rm| rm.replaced_axis.is_some())
                .map(|rm| (rm.layer_uid.as_str(), rm))
                .collect();
        if layer_map.is_empty() {
            return;
        }

        let mut current_layer: Option<Arc<str>> = None;
        let mut i = 0;
        while i < ops.commands.len() {
            if let OpCategory::Marker(cmd) = &ops.commands[i].category {
                match cmd {
                    MarkerCmd::LayerStart(uid) => {
                        current_layer = Some(uid.clone())
                    }
                    MarkerCmd::LayerEnd(_) => current_layer = None,
                    _ => {}
                }
                i += 1;
                continue;
            }

            if !ops.commands[i].is_moving() {
                i += 1;
                continue;
            }

            let layer_uid = match &current_layer {
                Some(uid) => uid.as_ref(),
                None => {
                    i += 1;
                    continue;
                }
            };
            let rm = match layer_map.get(layer_uid) {
                Some(rm) => rm,
                None => {
                    i += 1;
                    continue;
                }
            };

            let rotary_axis =
                Axis::from_str_name(&rm.rotary_axis).unwrap_or(Axis::A);

            // Read degrees from extra_axes before mutable borrow.
            let degrees: Option<f64> =
                ops.commands[i].extra_axes().and_then(|ea| {
                    ea.iter().find(|(a, _)| *a == rotary_axis).map(|(_, v)| *v)
                });

            let Some(degrees) = degrees else {
                i += 1;
                continue;
            };

            let scaled =
                degrees_to_scaled_mu(degrees, rm.mu_per_rotation, rm.reverse);
            let mut ea_vec: Vec<(Axis, f64)> = ops.commands[i]
                .extra_axes()
                .map(|ea| ea.to_vec())
                .unwrap_or_default();

            // Mutate endpoint and control points.
            {
                let node = &mut ops.commands[i];
                if let OpCategory::Moving { end, cmd } = &mut node.category {
                    if let Some(ref replaced) = rm.replaced_axis {
                        set_axis_value(end, replaced, scaled);
                    }

                    ea_vec.retain(|(a, _)| *a != rotary_axis);

                    match cmd {
                        MoveCmd::BezierTo { control1, control2 } => {
                            if let Some(ref replaced) = rm.replaced_axis {
                                let v1 = degrees_to_scaled_mu(
                                    control1.y,
                                    rm.mu_per_rotation,
                                    rm.reverse,
                                );
                                let v2 = degrees_to_scaled_mu(
                                    control2.y,
                                    rm.mu_per_rotation,
                                    rm.reverse,
                                );
                                if replaced == "Y" {
                                    control1.y = v1;
                                    control2.y = v2;
                                } else {
                                    set_axis_value(control1, replaced, v1);
                                    set_axis_value(control2, replaced, v2);
                                    control1.y = 0.0;
                                    control2.y = 0.0;
                                }
                            }
                        }
                        MoveCmd::QuadraticBezierTo { control } => {
                            if let Some(ref replaced) = rm.replaced_axis {
                                let v = degrees_to_scaled_mu(
                                    control.y,
                                    rm.mu_per_rotation,
                                    rm.reverse,
                                );
                                if replaced == "Y" {
                                    control.y = v;
                                } else {
                                    set_axis_value(control, replaced, v);
                                    control.y = 0.0;
                                }
                            }
                        }
                        MoveCmd::ArcTo { center, .. } => {
                            if let Some(ref _replaced) = rm.replaced_axis {
                                center.y = degrees_to_scaled_mu(
                                    center.y,
                                    rm.mu_per_rotation,
                                    rm.reverse,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Set extra_axes (mutable borrow released).
            if ea_vec.is_empty() {
                ops.commands[i].clear_extra_axes();
            } else {
                ops.commands[i].set_extra_axes(Arc::from(ea_vec));
            }

            i += 1;
        }

        ops.invalidate_time_cache();
    }
}

impl Compute for MachineTransformCompute {
    fn run(
        &mut self,
        ctx: &mut ComputeCtx,
    ) -> Result<Box<dyn Any + Send + Sync>, String> {
        let upstream =
            ctx.deps.get(&self.spec.source_key).ok_or_else(|| {
                format!("missing dependency: {}", self.spec.source_key)
            })?;

        let agg_ops = upstream
            .downcast_ref::<AssemblyOutput>()
            .map(|a| &a.ops)
            .or_else(|| {
                upstream.downcast_ref::<AggregateOutput>().map(|a| &a.ops)
            })
            .ok_or_else(|| {
                format!("cannot get Ops from dep: {}", self.spec.source_key)
            })?;

        let time_estimate = upstream
            .downcast_ref::<AggregateOutput>()
            .and_then(|a| a.time_estimate);

        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }

        let mut ops = agg_ops.clone();

        // 1. Linearize curves.
        if self.spec.linearize_curves {
            ops.linearize_curves();
        }

        // 2. Per-layer rotary mapping (Y→degrees, world-space).
        self.apply_rotary_mapping(&mut ops);

        // 3. World→machine + WCS + Z-flip.
        let mut combined = DMat4::IDENTITY;
        let w2m = array_to_dmat4(&self.spec.world_to_machine);
        if !is_identity(&w2m) {
            combined = w2m * combined;
        }

        // Per-layer WCS via translate_layers (subtracts offsets).
        if !self.spec.layer_wcs_offsets.is_empty() {
            let default = self.spec.default_wcs_offset;
            let offsets: Vec<(String, (f64, f64, f64))> = self
                .spec
                .layer_wcs_offsets
                .iter()
                .map(|(uid, off)| (uid.clone(), (off[0], off[1], off[2])))
                .collect();
            ops.translate_layers(
                (default[0], default[1], default[2]),
                Some(&offsets),
            );
        } else {
            let (ox, oy, oz) = (
                self.spec.default_wcs_offset[0],
                self.spec.default_wcs_offset[1],
                self.spec.default_wcs_offset[2],
            );
            if ox != 0.0 || oy != 0.0 || oz != 0.0 {
                let offset_matrix = DMat4::from_cols(
                    DVec4::new(1.0, 0.0, 0.0, 0.0),
                    DVec4::new(0.0, 1.0, 0.0, 0.0),
                    DVec4::new(0.0, 0.0, 1.0, 0.0),
                    DVec4::new(-ox, -oy, -oz, 1.0),
                );
                combined = offset_matrix * combined;
            }
        }

        // Z-flip.
        if self.spec.reverse_z {
            let z_flip = DMat4::from_cols(
                DVec4::new(1.0, 0.0, 0.0, 0.0),
                DVec4::new(0.0, 1.0, 0.0, 0.0),
                DVec4::new(0.0, 0.0, -1.0, 0.0),
                DVec4::new(0.0, 0.0, 0.0, 1.0),
            );
            combined = z_flip * combined;
        }

        if !is_identity(&combined) {
            ops.transform(combined);
        }

        // 4. AXIS_REPLACEMENT degrees→scaled-mu (per-layer, machine-space).
        self.apply_axis_replacement(&mut ops);

        Ok(Box::new(AggregateOutput { ops, time_estimate }))
    }

    fn source_keys(&self) -> Vec<String> {
        vec![self.spec.source_key.clone()]
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

    fn name(&self) -> &str {
        "machine_transform"
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn mu_to_degrees(
    mu: f64,
    diameter: f64,
    gear_ratio: f64,
    reverse: bool,
) -> f64 {
    if diameter <= 0.0 {
        return 0.0;
    }
    let circumference = diameter * std::f64::consts::PI;
    let mut degrees = (mu / circumference) * 360.0 * gear_ratio;
    if reverse {
        degrees = -degrees;
    }
    degrees
}

fn degrees_to_scaled_mu(
    degrees: f64,
    mu_per_rotation: f64,
    reverse: bool,
) -> f64 {
    if mu_per_rotation <= 0.0 {
        return degrees;
    }
    let mut scaled = degrees * mu_per_rotation / 360.0;
    if reverse {
        scaled = -scaled;
    }
    scaled
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

fn zero_axis(end: &mut Point3D, axis: &str) {
    match axis {
        "X" => end.x = 0.0,
        "Y" => end.y = 0.0,
        "Z" => end.z = 0.0,
        _ => {}
    }
}

fn set_axis_value(end: &mut Point3D, axis: &str, value: f64) {
    match axis {
        "X" => end.x = value,
        "Y" => end.y = value,
        "Z" => end.z = value,
        // Non-XYZ axes (A/B/C/U) are replaced axes that map to the Y
        // position in the 3-axis coordinate system, matching the
        // Python KinematicMapping.degrees_to_scaled_mu_pass behaviour
        // where _AXIS_TO_INDEX.get(non_xyz, 1) defaults to index 1 (Y).
        _ => end.y = value,
    }
}
