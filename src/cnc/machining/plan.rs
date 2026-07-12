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

use std::path::PathBuf;

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::algo::helix::HelixDirection;
use crate::geo::algo::ramp::RampStyle;
use crate::ops::assembly::adaptive::{self, AdaptiveClearingOptions};
use crate::ops::assembly::helix::{self, HelixOptions};
use crate::ops::assembly::profile::{self, ProfileInnerOptions};
use crate::ops::assembly::ramp::{self, RampOptions};
use crate::ops::assembly::result::{emit_trace_events, AssemblyMeta};
use crate::ops::assembly::slot::{self, SlotOptions};
use crate::ops::assembly::spiral::{self, SpiralOptions};
use crate::ops::assembly::toroid::{self, ToroidalClearOptions};
use crate::ops::assembly::wavefront::{self, AdaptiveWavefrontOptions};
use crate::ops::assembly::Tracelet;
use crate::ops::cut::{Part, ToolPose};
use crate::ops::state::State;
use crate::trace::Tracer;
use crate::trace_types::{
    EventKind, Meta, MetaValue, ProgressSnapshot, ToolSnapshot,
};
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
        part: Part,
        tool_radius: f64,
        step_over: f64,
        step_length: f64,
        target_z: f64,
        safe_z: f64,
        max_deflection_deg: f64,
        wall_margin: f64,
        area_tolerance: f64,
        angular_step: f64,
        start_pos: Option<Point3D>,
        start_heading: Option<f64>,
    },
    ProfileInner {
        part: Part,
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
        part: Part,
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
    /// Entry-style steps (helix, spiral, ramp, toroid, slot) deposit their
    /// swept polygons into `part.cleared` internally.  The stateful
    /// strategies (adaptive clearing, profiling, wavefront) borrow
    /// `part.cleared` directly and mutate it from within the assembler.
    pub fn execute(
        &self,
        trace: &mut Tracelet,
        part: &mut Part,
        cut_state: &State,
        travel_state: &State,
    ) -> RaygeoResult<AssemblyMeta> {
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
                let opts = HelixOptions {
                    center: *center,
                    start_radius: *helix_r,
                    z_start: *z_start,
                    z_end: *z_end,
                    pitch: *pitch,
                    direction: *direction,
                    angular_step: *angular_step,
                };
                helix::generate_helix(part, trace, &opts, cut_state)
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
                let opts = SpiralOptions {
                    center: *center,
                    z: *z,
                    start_radius: *start_radius,
                    end_radius: *end_radius,
                    revolutions: *revolutions,
                    direction: *direction,
                    angular_step: *angular_step,
                    start_angle: *start_angle,
                };
                spiral::generate_spiral(part, trace, &opts, cut_state)
            }
            WorkplanStep::RampEntry {
                start,
                end,
                z_start,
                z_end,
                max_ramp_angle_deg,
                lateral_amplitude,
            } => {
                let opts = RampOptions {
                    start: *start,
                    end: *end,
                    z_start: *z_start,
                    z_end: *z_end,
                    max_ramp_angle_deg: *max_ramp_angle_deg,
                    style: RampStyle::ZigZag,
                    lateral_amplitude: *lateral_amplitude,
                };
                ramp::generate_ramp(part, trace, &opts, cut_state)
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
                let mut full_carrier = carrier.clone();
                if let Some(&first) = carrier.first() {
                    if let Some(plunge) = part.cleared.find_plunge_point(
                        &part.stock_region,
                        first,
                        *tool_radius,
                        *tool_radius * 3.0,
                    ) {
                        full_carrier.insert(0, plunge);
                    }
                }
                let opts = ToroidalClearOptions {
                    carrier: full_carrier,
                    start: *start,
                    target_z: *target_z,
                    tool_radius: *tool_radius,
                    step_over: *step_over,
                    max_ramp_angle_deg: *max_ramp_angle_deg,
                    direction: *direction,
                    angular_step: *angular_step,
                };
                toroid::generate_toroidal_clear(part, trace, &opts, cut_state)
            }
            WorkplanStep::Slot {
                carrier,
                tool_radius,
                target_z,
            } => {
                let mut full_carrier = carrier.clone();
                if let Some(&first) = carrier.first() {
                    if let Some(plunge) = part.cleared.find_plunge_point(
                        &part.stock_region,
                        first,
                        *tool_radius,
                        *tool_radius * 3.0,
                    ) {
                        full_carrier.insert(0, plunge);
                    }
                }
                let opts = SlotOptions {
                    carrier: full_carrier,
                    tool_radius: *tool_radius,
                    target_z: *target_z,
                };
                slot::generate_slot(part, trace, &opts, cut_state)
            }
            WorkplanStep::AdaptiveClear {
                part: step_part,
                tool_radius,
                step_over,
                step_length,
                target_z,
                safe_z,
                max_deflection_deg,
                wall_margin,
                area_tolerance,
                start_pos,
                start_heading,
                ..
            } => {
                let (boundary, islands) = step_part.extract_boundary();
                let boundary = boundary.unwrap_or_default();
                let opts = AdaptiveClearingOptions {
                    tool_radius: *tool_radius,
                    step_over: *step_over,
                    step_length: *step_length,
                    target_z: *target_z,
                    safe_z: *safe_z,
                    max_deflection_deg: *max_deflection_deg,
                    wall_margin: *wall_margin,
                    area_tolerance: *area_tolerance,
                    start_pos: *start_pos,
                    start_heading: *start_heading,
                    ..Default::default()
                };
                let saved_region = std::mem::replace(
                    &mut part.stock_region,
                    crate::ops::cut::StockRegion::new(
                        boundary.clone(),
                        islands.clone(),
                    ),
                );
                let result =
                    adaptive::adaptive_clearing(part, trace, &opts, cut_state);
                part.stock_region = saved_region;
                result
            }
            WorkplanStep::ProfileInner {
                part: step_part,
                tool_radius,
                step_over,
                step_length,
                target_z,
                safe_z,
                wall_margin,
                stock_to_leave,
            } => {
                let (boundary, islands) = step_part.extract_boundary();
                let boundary = boundary.unwrap_or_default();
                let opts = ProfileInnerOptions {
                    tool_radius: *tool_radius,
                    step_over: *step_over,
                    step_length: *step_length,
                    target_z: *target_z,
                    safe_z: *safe_z,
                    wall_margin: *wall_margin,
                    stock_to_leave: *stock_to_leave,
                    ..Default::default()
                };
                let saved_region = std::mem::replace(
                    &mut part.stock_region,
                    crate::ops::cut::StockRegion::new(
                        boundary.clone(),
                        islands.clone(),
                    ),
                );
                let result =
                    profile::profile_inner(part, trace, &opts, cut_state);
                part.stock_region = saved_region;
                result
            }
            WorkplanStep::Wavefront {
                part: step_part,
                tool_radius,
                step_over,
                z,
                area_tolerance,
                precision,
            } => {
                let (boundary, islands) = step_part.extract_boundary();
                let boundary = boundary.unwrap_or_default();
                let opts = AdaptiveWavefrontOptions {
                    tool_radius: *tool_radius,
                    step_over: *step_over,
                    z: *z,
                    area_tolerance: *area_tolerance,
                    precision: *precision,
                };
                let saved_region = std::mem::replace(
                    &mut part.stock_region,
                    crate::ops::cut::StockRegion::new(
                        boundary.clone(),
                        islands.clone(),
                    ),
                );
                let result = wavefront::adaptive_wavefronts(
                    part, trace, &opts, cut_state,
                );
                part.stock_region = saved_region;
                result
            }
            WorkplanStep::Retract { safe_z } => {
                let pos = Point3D::new(0.0, 0.0, *safe_z);
                trace.apply_state(travel_state);
                trace.move_to(pos.x, pos.y, *safe_z, None);
                Ok(AssemblyMeta {
                    start: ToolPose { pos, heading: 0.0 },
                    end: ToolPose { pos, heading: 0.0 },
                })
            }
        }
    }

    /// Short machine name for the assembler that produced this step.
    pub fn assembler(&self) -> &'static str {
        match self {
            WorkplanStep::HelixPlunge { .. } => "helix",
            WorkplanStep::FlatSpiral { .. } => "spiral",
            WorkplanStep::RampEntry { .. } => "ramp",
            WorkplanStep::ToroidalClear { .. } => "toroid",
            WorkplanStep::Slot { .. } => "slot",
            WorkplanStep::AdaptiveClear { .. } => "adaptive",
            WorkplanStep::ProfileInner { .. } => "profile",
            WorkplanStep::Wavefront { .. } => "wavefront",
            WorkplanStep::Retract { .. } => "retract",
        }
    }

    /// Short human label (variant name).
    pub fn label(&self) -> String {
        match self {
            WorkplanStep::HelixPlunge { .. } => "HelixPlunge".to_string(),
            WorkplanStep::FlatSpiral { .. } => "FlatSpiral".to_string(),
            WorkplanStep::RampEntry { .. } => "RampEntry".to_string(),
            WorkplanStep::ToroidalClear { .. } => "ToroidalClear".to_string(),
            WorkplanStep::Slot { .. } => "Slot".to_string(),
            WorkplanStep::AdaptiveClear { .. } => "AdaptiveClear".to_string(),
            WorkplanStep::ProfileInner { .. } => "ProfileInner".to_string(),
            WorkplanStep::Wavefront { .. } => "Wavefront".to_string(),
            WorkplanStep::Retract { .. } => "Retract".to_string(),
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

    /// Create a new empty workplan from a [`Part`](crate::ops::cut::Part).
    ///
    /// Extracts the boundary polygon and islands from `part.geometry`.
    /// Returns `None` if the part has no extractable boundary geometry.
    pub fn from_part(
        part: &crate::ops::cut::Part,
        safe_z: f64,
    ) -> Option<Self> {
        let (boundary, islands) = part.extract_boundary();
        Some(Workplan {
            steps: Vec::new(),
            pocket_boundary: boundary?,
            islands,
            safe_z,
        })
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
    ///
    /// When `trace` is `Some(path)`, a structured span/event trace file
    /// is written alongside the normal machining output.
    #[prof]
    pub fn execute(
        &self,
        trace: &mut Tracelet,
        cut_state: &State,
        travel_state: &State,
        trace_path: Option<PathBuf>,
    ) -> RaygeoResult<AssemblyMeta> {
        let mut tracer = Tracer::open(trace_path);

        // ── Root span ──────────────────────────────────────────────
        let mut steps_meta = Vec::new();
        for (i, step) in self.steps.iter().enumerate() {
            steps_meta.push(MetaValue::Str(format!("#{} {}", i, step.label())));
        }
        let mut attrs: Meta = Meta::new();
        attrs.insert("safe_z".into(), MetaValue::F64(self.safe_z));
        attrs.insert("steps".into(), MetaValue::List(steps_meta));
        attrs.insert("boundary".into(), polygon_to_meta(&self.pocket_boundary));
        attrs.insert(
            "islands".into(),
            MetaValue::List(self.islands.iter().map(polygon_to_meta).collect()),
        );
        let root = tracer.enter(0, "workplan", "Workplan", Some(attrs));

        // ── Execute steps with interleaved links ───────────────────
        let mut part = Part::from_polygons(
            &self.pocket_boundary,
            &self.islands,
            (0.0, 0.0),
        );
        let mut prev_end: Option<ToolPose> = None;
        let mut first_start: Option<ToolPose> = None;

        for (i, step) in self.steps.iter().enumerate() {
            trace.emit_step_start(i, &step.label());

            // Run step in temp tracelet
            let mut temp = Tracelet::new();
            temp.begin_section(step.assembler(), &step.label());
            let meta =
                step.execute(&mut temp, &mut part, cut_state, travel_state)?;
            let step_events = temp.drain();
            let step_attrs = temp.attrs().cloned();
            let step_ops = temp.into_ops();

            if first_start.is_none() {
                first_start = Some(meta.start);
            }

            // Emit link BEFORE step ops (link goes to main trace, step ops follow)
            if let Some(pe) = prev_end {
                let entry_z = pass_start_z_ops(&step_ops);
                trace.begin_section("workplan", "link");
                emit_link(
                    trace,
                    travel_state,
                    pe,
                    meta.start.pos,
                    self.safe_z,
                    entry_z,
                );
                let link_events = trace.drain();
                let link_span = tracer.enter(root, "workplan", "link", None);
                emit_trace_events(
                    &mut tracer,
                    link_span,
                    "workplan",
                    &link_events,
                );
                tracer.exit(link_span, "workplan");
            }

            // Push step's ops into main trace (in correct order, after link)
            for node in step_ops.commands {
                trace.push_raw(node);
            }

            // Feed step's events to Tracer
            let step_span = tracer.enter(
                root,
                step.assembler(),
                &format!("#{} {}", i, step.label()),
                step_attrs,
            );
            emit_trace_events(
                &mut tracer,
                step_span,
                step.assembler(),
                &step_events,
            );
            tracer.exit(step_span, step.assembler());

            trace.emit_step_end(i);
            prev_end = Some(meta.end);
        }

        // Final lift
        let mut pe = match prev_end {
            Some(e) => e,
            None => {
                tracer.exit(root, "workplan");
                tracer.finish();
                trace.finish();
                return Ok(AssemblyMeta {
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
        };

        if pe.pos.z < self.safe_z - 1e-12 {
            let lift_span = tracer.enter(root, "workplan", "final_lift", None);
            trace.begin_section("workplan", "final_lift");
            trace.apply_state(travel_state);
            trace.move_to(pe.pos.x, pe.pos.y, self.safe_z, None);
            let lift_events = trace.drain();
            // Emit minimal init/move/exit for the lift
            tracer.init(
                lift_span,
                "workplan",
                ToolSnapshot {
                    pos_x: pe.pos.x,
                    pos_y: pe.pos.y,
                    pos_z: pe.pos.z,
                    heading: pe.heading,
                    prev_x: pe.pos.x,
                    prev_y: pe.pos.y,
                    prev_z: pe.pos.z,
                },
                ProgressSnapshot::default(),
                None,
            );
            emit_trace_events(&mut tracer, lift_span, "workplan", &lift_events);
            tracer.event(
                lift_span,
                "workplan",
                EventKind::Exit,
                Some(ToolSnapshot {
                    pos_x: pe.pos.x,
                    pos_y: pe.pos.y,
                    pos_z: self.safe_z,
                    heading: pe.heading,
                    prev_x: pe.pos.x,
                    prev_y: pe.pos.y,
                    prev_z: pe.pos.z,
                }),
                None,
                None,
            );
            tracer.exit(lift_span, "workplan");
            pe.pos.z = self.safe_z;
        }

        trace.finish();
        tracer.exit(root, "workplan");
        tracer.finish();

        Ok(AssemblyMeta {
            start: first_start.unwrap(),
            end: pe,
        })
    }
}

/// Convert a polygon to a MetaValue list of [x, y] pairs.
fn polygon_to_meta(poly: &Polygon) -> MetaValue {
    MetaValue::List(
        poly.iter()
            .map(|p| {
                MetaValue::List(vec![MetaValue::F64(p.x), MetaValue::F64(p.y)])
            })
            .collect(),
    )
}

fn pass_start_z_ops(ops: &crate::ops::container::Ops) -> f64 {
    for i in 0..ops.len() {
        if ops.is_cutting(i) || ops.is_travel(i) {
            return ops.endpoint(i).z;
        }
    }
    0.0
}

/// Emit travel moves for a link between two passes.
///
/// Writes retract/XY-travel/plunge moves to the Tracelet. The caller
/// is responsible for tracing (begin_section/drain/emit_trace_events).
fn emit_link(
    trace: &mut Tracelet,
    travel_state: &State,
    from: ToolPose,
    to: Point3D,
    safe_z: f64,
    entry_z: f64,
) {
    trace.apply_state(travel_state);
    trace.move_to(from.pos.x, from.pos.y, safe_z, None);
    if (to.x - from.pos.x).abs() > 1e-12 || (to.y - from.pos.y).abs() > 1e-12 {
        trace.move_to(to.x, to.y, safe_z, None);
    }
    if (entry_z - safe_z).abs() > 1e-12 {
        trace.move_to(to.x, to.y, entry_z, None);
    }
}
