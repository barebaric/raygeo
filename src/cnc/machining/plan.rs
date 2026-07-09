//! Workplan step type and executor.
//!
//! [`WorkplanStep`] is the shared, serialisable description of one
//! operation in a CNC workplan. Each variant owns the knowledge of how
//! to invoke its underlying ops-layer assembler via [`execute`], so the
//! executor ([`execute_workplan`]) stays a dumb loop that only manages
//! the shared [`ClearedArea`] and chains results.
//!
//! [`execute`]: WorkplanStep::execute

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::algo::ramp::RampStyle;
use crate::ops::assembly::adaptive::{self, AdaptiveClearingOptions};
use crate::ops::assembly::helix::{self, HelixOptions};
use crate::ops::assembly::profile::{self, ProfileInnerOptions};
use crate::ops::assembly::ramp::{self, RampOptions};
use crate::ops::assembly::result::{chain, AssemblyResult};
use crate::ops::assembly::slot::{self, SlotOptions};
use crate::ops::assembly::spiral::{self, SpiralOptions};
use crate::ops::assembly::toroid::{self, ToroidalClearOptions};
use crate::ops::assembly::wavefront::{self, AdaptiveWavefrontOptions};
use crate::ops::container::Ops;
use crate::ops::cut::{ClearedArea, ToolPose};
use crate::ops::state::State;
use crate::types::{Point, Point3D, Polygon};

pub enum WorkplanStep {
    HelixPlunge {
        center: Point,
        helix_r: f64,
        z_start: f64,
        z_end: f64,
        pitch: f64,
        direction: HelixDirection,
        angular_step: f64,
    },
    FlatSpiral {
        center: Point,
        z: f64,
        start_radius: f64,
        end_radius: f64,
        revolutions: f64,
        direction: HelixDirection,
        angular_step: f64,
        start_angle: f64,
    },
    RampEntry {
        start: Point,
        end: Point,
        z_start: f64,
        z_end: f64,
        max_ramp_angle_deg: f64,
        lateral_amplitude: f64,
    },
    ToroidalClear {
        carrier: Vec<Point>,
        start: Point3D,
        target_z: f64,
        tool_radius: f64,
        step_over: f64,
        max_ramp_angle_deg: f64,
        direction: HelixDirection,
        angular_step: f64,
    },
    Slot {
        carrier: Vec<Point>,
        tool_radius: f64,
        target_z: f64,
    },
    AdaptiveClear {
        pocket_boundary: Polygon,
        islands: Vec<Polygon>,
        tool_radius: f64,
        step_over: f64,
        step_length: f64,
        target_z: f64,
        safe_z: f64,
        max_deflection_deg: f64,
        wall_margin: f64,
        area_tolerance: f64,
        angular_step: f64,
    },
    ProfileInner {
        boundary: Polygon,
        islands: Vec<Polygon>,
        tool_radius: f64,
        step_over: f64,
        step_length: f64,
        target_z: f64,
        safe_z: f64,
        wall_margin: f64,
        stock_to_leave: f64,
    },
    /// Inside-out wavefront expansion from the current cleared area.
    Wavefront {
        pocket_boundary: Polygon,
        islands: Vec<Polygon>,
        tool_radius: f64,
        step_over: f64,
        z: f64,
        area_tolerance: f64,
        precision: f64,
    },
    Retract {
        safe_z: f64,
    },
}

impl WorkplanStep {
    /// Invoke this step's assembler.
    ///
    /// Each variant knows how to call its own minimal ops-layer
    /// assembler. Entry-style steps (helix, spiral, ramp, toroid, slot)
    /// deposit their swept polygons into `cleared` via
    /// [`ClearedArea::cut`]; the stateful strategies (adaptive clearing,
    /// profiling, wavefront) borrow `cleared` directly and mutate it
    /// from within the assembler.
    pub fn execute(
        &self,
        cleared: &mut ClearedArea,
        cut_state: &State,
        travel_state: &State,
    ) -> RaygeoResult<AssemblyResult> {
        match self {
            WorkplanStep::HelixPlunge {
                center,
                helix_r,
                z_start,
                z_end,
                pitch,
                direction,
                angular_step,
            } => {
                let r = helix::generate_helix(
                    &HelixOptions {
                        center: *center,
                        start_radius: *helix_r,
                        z_start: *z_start,
                        z_end: *z_end,
                        pitch: *pitch,
                        direction: *direction,
                        angular_step: *angular_step,
                    },
                    cut_state,
                )?;
                cleared.cut(&r.cleared_polygons);
                Ok(r)
            }
            WorkplanStep::FlatSpiral {
                center,
                z,
                start_radius,
                end_radius,
                revolutions,
                direction,
                angular_step,
                start_angle,
            } => {
                let r = spiral::generate_spiral(
                    &SpiralOptions {
                        center: *center,
                        z: *z,
                        start_radius: *start_radius,
                        end_radius: *end_radius,
                        revolutions: *revolutions,
                        direction: *direction,
                        angular_step: *angular_step,
                        start_angle: *start_angle,
                    },
                    cut_state,
                )?;
                cleared.cut(&r.cleared_polygons);
                Ok(r)
            }
            WorkplanStep::RampEntry {
                start,
                end,
                z_start,
                z_end,
                max_ramp_angle_deg,
                lateral_amplitude,
            } => {
                let r = ramp::generate_ramp(
                    &RampOptions {
                        start: *start,
                        end: *end,
                        z_start: *z_start,
                        z_end: *z_end,
                        max_ramp_angle_deg: *max_ramp_angle_deg,
                        style: RampStyle::ZigZag,
                        lateral_amplitude: *lateral_amplitude,
                    },
                    cut_state,
                )?;
                cleared.cut(&r.cleared_polygons);
                Ok(r)
            }
            WorkplanStep::ToroidalClear {
                carrier,
                start,
                target_z,
                tool_radius,
                step_over,
                max_ramp_angle_deg,
                direction,
                angular_step,
            } => {
                let r = toroid::generate_toroidal_clear(
                    &ToroidalClearOptions {
                        carrier: carrier.clone(),
                        start: *start,
                        target_z: *target_z,
                        tool_radius: *tool_radius,
                        step_over: *step_over,
                        max_ramp_angle_deg: *max_ramp_angle_deg,
                        direction: *direction,
                        angular_step: *angular_step,
                    },
                    cut_state,
                )?;
                cleared.cut(&r.cleared_polygons);
                Ok(r)
            }
            WorkplanStep::Slot {
                carrier,
                tool_radius,
                target_z,
            } => {
                let r = slot::generate_slot(
                    &SlotOptions {
                        carrier: carrier.clone(),
                        tool_radius: *tool_radius,
                        target_z: *target_z,
                    },
                    cut_state,
                )?;
                cleared.cut(&r.cleared_polygons);
                Ok(r)
            }
            WorkplanStep::AdaptiveClear {
                pocket_boundary,
                islands,
                tool_radius,
                step_over,
                step_length,
                target_z,
                safe_z,
                max_deflection_deg,
                wall_margin,
                area_tolerance,
                ..
            } => adaptive::adaptive_clearing(
                cleared,
                &AdaptiveClearingOptions {
                    pocket_boundary: pocket_boundary.clone(),
                    islands: islands.clone(),
                    tool_radius: *tool_radius,
                    step_over: *step_over,
                    step_length: *step_length,
                    target_z: *target_z,
                    safe_z: *safe_z,
                    max_deflection_deg: *max_deflection_deg,
                    wall_margin: *wall_margin,
                    area_tolerance: *area_tolerance,
                    ..Default::default()
                },
                cut_state,
            ),
            WorkplanStep::ProfileInner {
                boundary,
                islands,
                tool_radius,
                step_over,
                step_length,
                target_z,
                safe_z,
                wall_margin,
                stock_to_leave,
            } => profile::profile_inner(
                cleared,
                &ProfileInnerOptions {
                    boundary: boundary.clone(),
                    islands: islands.clone(),
                    tool_radius: *tool_radius,
                    step_over: *step_over,
                    step_length: *step_length,
                    target_z: *target_z,
                    safe_z: *safe_z,
                    wall_margin: *wall_margin,
                    stock_to_leave: *stock_to_leave,
                    ..Default::default()
                },
                cut_state,
            ),
            WorkplanStep::Wavefront {
                pocket_boundary,
                islands,
                tool_radius,
                step_over,
                z,
                area_tolerance,
                precision,
            } => wavefront::adaptive_wavefronts(
                cleared,
                &AdaptiveWavefrontOptions {
                    pocket_boundary: pocket_boundary.clone(),
                    islands: islands.clone(),
                    tool_radius: *tool_radius,
                    step_over: *step_over,
                    z: *z,
                    area_tolerance: *area_tolerance,
                    precision: *precision,
                },
                cut_state,
            ),
            WorkplanStep::Retract { safe_z } => {
                let pos = Point3D::new(0.0, 0.0, *safe_z);
                let mut ops = Ops::new();
                ops.apply_state(travel_state);
                ops.move_to(pos.x, pos.y, *safe_z, None);
                Ok(AssemblyResult {
                    ops,
                    cleared_polygons: vec![],
                    start: ToolPose { pos, heading: 0.0 },
                    end: ToolPose { pos, heading: 0.0 },
                })
            }
        }
    }
}

/// Execute a workplan: dispatch each step to its assembler and chain the
/// results into one [`AssemblyResult`].
///
/// `pocket_boundary` and `islands` describe the stock and seed the
/// shared [`ClearedArea`]. `cut_state` is applied to cutting moves;
/// `travel_state` is applied to retract/travel moves (e.g.
/// [`WorkplanStep::Retract`]).
#[prof]
pub fn execute_workplan(
    steps: &[WorkplanStep],
    pocket_boundary: &Polygon,
    islands: &[Polygon],
    cut_state: &State,
    travel_state: &State,
) -> RaygeoResult<AssemblyResult> {
    let mut cleared = ClearedArea::new(pocket_boundary, islands);
    let mut acc: Option<AssemblyResult> = None;

    for step in steps {
        let step_result =
            step.execute(&mut cleared, cut_state, travel_state)?;
        acc = Some(match acc.take() {
            Some(a) => chain(a, step_result),
            None => step_result,
        });
    }

    let mut result = acc.unwrap_or_else(|| AssemblyResult {
        ops: Ops::new(),
        cleared_polygons: vec![],
        start: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
        end: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
    });
    // The shared cleared area is the authoritative record of what was
    // removed; prefer it over the per-step polygon concatenation.
    result.cleared_polygons = cleared.fragments().to_vec();
    Ok(result)
}
