//! Workplan step type and executor.
//!
//! [`WorkplanStep`] is the shared, serialisable description of one
//! operation in a CNC workplan. Each variant owns the knowledge of how
//! to invoke its underlying ops-layer assembler via [`execute`].
//!
//! [`Workplan`] is the explicit executor struct that captures plan-time
//! context (`pocket_boundary`, `islands`, `safe_z`) and provides an
//! [`execute`](Workplan::execute) method that manages the shared
//! [`ClearedArea`], runs each step, and links passes with safe-Z travel
//! moves.
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
use crate::ops::assembly::result::AssemblyResult;
use crate::ops::assembly::slot::{self, SlotOptions};
use crate::ops::assembly::spiral::{self, SpiralOptions};
use crate::ops::assembly::toroid::{self, ToroidalClearOptions};
use crate::ops::assembly::wavefront::{self, AdaptiveWavefrontOptions};
use crate::ops::container::Ops;
use crate::ops::cut::{ClearedArea, ToolPose};
use crate::ops::state::State;
use crate::types::{Point, Point3D, Polygon};

#[derive(Clone, Debug)]
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

/// Plan-time context and executor for a sequence of [`WorkplanStep`]s.
///
/// Captures `pocket_boundary`, `islands`, and `safe_z` at plan time;
/// [`execute`](Workplan::execute) takes only the runtime tool states.
#[derive(Clone, Debug)]
pub struct Workplan {
    pub steps: Vec<WorkplanStep>,
    pub pocket_boundary: Polygon,
    pub islands: Vec<Polygon>,
    pub safe_z: f64,
}

impl Workplan {
    /// Create a new empty workplan.
    pub fn new(
        pocket_boundary: Polygon,
        islands: Vec<Polygon>,
        safe_z: f64,
    ) -> Self {
        Workplan {
            steps: Vec::new(),
            pocket_boundary,
            islands,
            safe_z,
        }
    }

    /// Append builder output steps.
    pub fn extend(&mut self, steps: &[WorkplanStep]) {
        self.steps.extend(steps.iter().cloned());
    }

    /// Execute all steps, linking passes with safe-Z travel moves.
    ///
    /// Each step's assembler is dispatched in order with the shared
    /// [`ClearedArea`]. Between passes the tool retracts to `safe_z`,
    /// travels XY, and plunges — all under `travel_state`. A final
    /// lift to `safe_z` is appended if the last pass ends below it.
    #[prof]
    pub fn execute(
        &self,
        cut_state: &State,
        travel_state: &State,
    ) -> RaygeoResult<AssemblyResult> {
        let mut cleared =
            ClearedArea::new(&self.pocket_boundary, &self.islands);
        let mut passes: Vec<AssemblyResult> = Vec::new();

        for step in &self.steps {
            let r = step.execute(&mut cleared, cut_state, travel_state)?;
            passes.push(r);
        }

        if passes.is_empty() {
            return Ok(AssemblyResult {
                ops: Ops::new(),
                cleared_polygons: cleared.fragments().to_vec(),
                start: ToolPose {
                    pos: Point3D::ZERO,
                    heading: 0.0,
                },
                end: ToolPose {
                    pos: Point3D::ZERO,
                    heading: 0.0,
                },
            });
        }

        // ── Link passes with travel moves ──────────────────────────
        let mut ops = Ops::new();
        let mut prev_end = passes[0].end;

        ops.extend(&passes[0].ops);

        for pass in &passes[1..] {
            let entry = pass.start.pos;
            let entry_z = pass_start_z(pass);

            ops.apply_state(travel_state);
            ops.move_to(prev_end.pos.x, prev_end.pos.y, self.safe_z, None);
            if (entry.x - prev_end.pos.x).abs() > 1e-12
                || (entry.y - prev_end.pos.y).abs() > 1e-12
            {
                ops.move_to(entry.x, entry.y, self.safe_z, None);
            }
            if (entry_z - self.safe_z).abs() > 1e-12 {
                ops.move_to(entry.x, entry.y, entry_z, None);
            }

            ops.extend(&pass.ops);
            prev_end = pass.end;
        }

        // ── Final lift ─────────────────────────────────────────────
        if prev_end.pos.z < self.safe_z - 1e-12 {
            ops.apply_state(travel_state);
            ops.move_to(prev_end.pos.x, prev_end.pos.y, self.safe_z, None);
            prev_end.pos.z = self.safe_z;
        }

        let mut result = AssemblyResult {
            ops,
            cleared_polygons: passes
                .iter()
                .flat_map(|p| p.cleared_polygons.iter().cloned())
                .collect(),
            start: passes[0].start,
            end: prev_end,
        };
        result.cleared_polygons = cleared.fragments().to_vec();
        Ok(result)
    }
}

fn pass_start_z(result: &AssemblyResult) -> f64 {
    for i in 0..result.ops.len() {
        if result.ops.is_cutting(i) || result.ops.is_travel(i) {
            return result.ops.endpoint(i).z;
        }
    }
    0.0
}
