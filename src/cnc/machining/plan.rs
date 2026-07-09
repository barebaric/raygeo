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
use crate::ops::assembly::result::{emit_trace_events, AssemblyResult};
use crate::ops::assembly::slot::{self, SlotOptions};
use crate::ops::assembly::spiral::{self, SpiralOptions};
use crate::ops::assembly::toroid::{self, ToroidalClearOptions};
use crate::ops::assembly::wavefront::{self, AdaptiveWavefrontOptions};
use crate::ops::container::Ops;
use crate::ops::cut::{ClearedArea, ToolPose};
use crate::ops::state::State;
use crate::trace::Tracer;
use crate::trace_types::{
    EventKind, Meta, MetaValue, MoveKind, ProgressSnapshot, ToolSnapshot,
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
                    trace: None,
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
        cut_state: &State,
        travel_state: &State,
        trace: Option<PathBuf>,
    ) -> RaygeoResult<AssemblyResult> {
        let mut tracer = Tracer::open(trace);

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
        let mut cleared =
            ClearedArea::new(&self.pocket_boundary, &self.islands);
        let mut ops = Ops::new();
        let mut prev_end: Option<ToolPose> = None;
        let mut first_start: Option<ToolPose> = None;

        for (i, step) in self.steps.iter().enumerate() {
            let r = step.execute(&mut cleared, cut_state, travel_state)?;

            if first_start.is_none() {
                first_start = Some(r.start);
            }

            if let Some(pe) = prev_end {
                emit_link(
                    &mut ops,
                    &mut tracer,
                    root,
                    travel_state,
                    pe,
                    r.start.pos,
                    self.safe_z,
                    pass_start_z(&r),
                );
            }

            let step_attrs = r.trace.as_ref().and_then(|t| t.attrs.clone());
            let step_span = tracer.enter(
                root,
                step.assembler(),
                &format!("#{} {}", i, step.label()),
                step_attrs,
            );

            match &r.trace {
                Some(t) => {
                    emit_trace_events(
                        &mut tracer,
                        step_span,
                        step.assembler(),
                        &t.events,
                    );
                }
                None => {
                    tracer.init(
                        step_span,
                        step.assembler(),
                        ToolSnapshot {
                            pos_x: r.start.pos.x,
                            pos_y: r.start.pos.y,
                            pos_z: r.start.pos.z,
                            heading: r.start.heading,
                            prev_x: r.start.pos.x,
                            prev_y: r.start.pos.y,
                            prev_z: r.start.pos.z,
                        },
                        ProgressSnapshot {
                            step_idx: 0,
                            ops_len: 0,
                            status: 0,
                        },
                        None,
                    );
                    replay_ops(
                        &mut tracer,
                        step_span,
                        step.assembler(),
                        &r.ops,
                        r.start,
                    );
                    tracer.event(
                        step_span,
                        step.assembler(),
                        EventKind::Exit,
                        Some(ToolSnapshot {
                            pos_x: r.end.pos.x,
                            pos_y: r.end.pos.y,
                            pos_z: r.end.pos.z,
                            heading: r.end.heading,
                            prev_x: r.end.pos.x,
                            prev_y: r.end.pos.y,
                            prev_z: r.end.pos.z,
                        }),
                        None,
                    );
                }
            }
            tracer.exit(step_span, step.assembler());

            ops.extend(&r.ops);
            prev_end = Some(r.end);
        }

        if first_start.is_none() {
            tracer.exit(root, "workplan");
            tracer.finish();
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
                trace: None,
            });
        }

        let mut pe = prev_end.unwrap();

        // ── Final lift ─────────────────────────────────────────────
        if pe.pos.z < self.safe_z - 1e-12 {
            let lift_span = tracer.enter(root, "workplan", "final_lift", None);
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
                ProgressSnapshot {
                    step_idx: 0,
                    ops_len: 0,
                    status: 0,
                },
                None,
            );

            ops.apply_state(travel_state);
            ops.move_to(pe.pos.x, pe.pos.y, self.safe_z, None);
            let lift_pos = Point3D::new(pe.pos.x, pe.pos.y, self.safe_z);
            tracer.move_point(
                lift_span,
                "workplan",
                MoveKind::Travel,
                ToolSnapshot {
                    pos_x: lift_pos.x,
                    pos_y: lift_pos.y,
                    pos_z: lift_pos.z,
                    heading: pe.heading,
                    prev_x: pe.pos.x,
                    prev_y: pe.pos.y,
                    prev_z: pe.pos.z,
                },
                None,
                None,
            );

            tracer.event(
                lift_span,
                "workplan",
                EventKind::Exit,
                Some(ToolSnapshot {
                    pos_x: lift_pos.x,
                    pos_y: lift_pos.y,
                    pos_z: lift_pos.z,
                    heading: pe.heading,
                    prev_x: pe.pos.x,
                    prev_y: pe.pos.y,
                    prev_z: pe.pos.z,
                }),
                None,
            );
            tracer.exit(lift_span, "workplan");

            pe.pos.z = self.safe_z;
        }

        let result = AssemblyResult {
            ops,
            cleared_polygons: cleared.fragments().to_vec(),
            start: first_start.unwrap(),
            end: pe,
            trace: None,
        };

        tracer.exit(root, "workplan");
        tracer.finish();
        Ok(result)
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

/// Walk the Ops commands and emit trace move events.
///
/// Each move event records the destination endpoint as `pos` and the
/// origin as `prev`, with heading recomputed from the vector connecting
/// them.
fn replay_ops(
    tracer: &mut Tracer,
    span: u32,
    source: &str,
    ops: &Ops,
    start: ToolPose,
) {
    let mut pos = start.pos;
    let mut heading = start.heading;
    let mut step_idx: u32 = 0;

    for node in &ops.commands {
        if !node.is_moving() {
            continue;
        }
        let endpoint = node.end_point();
        let kind = match &node.category {
            crate::ops::types::OpCategory::Moving { cmd, .. } => match cmd {
                crate::ops::types::MoveCmd::MoveTo => MoveKind::Travel,
                crate::ops::types::MoveCmd::LineTo
                | crate::ops::types::MoveCmd::ArcTo { .. }
                | crate::ops::types::MoveCmd::BezierTo { .. }
                | crate::ops::types::MoveCmd::QuadraticBezierTo { .. }
                | crate::ops::types::MoveCmd::ScanLine { .. } => MoveKind::Cut,
            },
            _ => MoveKind::Cut,
        };

        let dx = endpoint.x - pos.x;
        let dy = endpoint.y - pos.y;
        if dx.abs() > 1e-12 || dy.abs() > 1e-12 {
            heading = dy.atan2(dx);
        }

        tracer.move_point(
            span,
            source,
            kind,
            ToolSnapshot {
                pos_x: endpoint.x,
                pos_y: endpoint.y,
                pos_z: endpoint.z,
                heading,
                prev_x: pos.x,
                prev_y: pos.y,
                prev_z: pos.z,
            },
            Some(ProgressSnapshot {
                step_idx,
                ops_len: ops.len() as u32,
                status: 0,
            }),
            None,
        );

        pos = endpoint;
        step_idx += 1;
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

/// Emit a link span with retract/XY-travel/plunge moves between two passes.
///
/// The tool moves from `from` to `(to.x, to.y, entry_z)` via safe-Z
/// retract, optional XY travel, and optional plunge.
#[allow(clippy::too_many_arguments)]
fn emit_link(
    ops: &mut Ops,
    tracer: &mut Tracer,
    parent_span: u32,
    travel_state: &State,
    from: ToolPose,
    to: Point3D,
    safe_z: f64,
    entry_z: f64,
) {
    let link_span = tracer.enter(parent_span, "workplan", "link", None);
    tracer.init(
        link_span,
        "workplan",
        ToolSnapshot {
            pos_x: from.pos.x,
            pos_y: from.pos.y,
            pos_z: from.pos.z,
            heading: from.heading,
            prev_x: from.pos.x,
            prev_y: from.pos.y,
            prev_z: from.pos.z,
        },
        ProgressSnapshot {
            step_idx: 0,
            ops_len: 0,
            status: 0,
        },
        None,
    );

    let mut cur_pos = from.pos;
    let mut cur_heading = from.heading;

    ops.apply_state(travel_state);
    ops.move_to(from.pos.x, from.pos.y, safe_z, None);
    let retract_pos = Point3D::new(from.pos.x, from.pos.y, safe_z);
    tracer.move_point(
        link_span,
        "workplan",
        MoveKind::Travel,
        ToolSnapshot {
            pos_x: retract_pos.x,
            pos_y: retract_pos.y,
            pos_z: retract_pos.z,
            heading: cur_heading,
            prev_x: cur_pos.x,
            prev_y: cur_pos.y,
            prev_z: cur_pos.z,
        },
        None,
        None,
    );
    cur_pos = retract_pos;

    if (to.x - from.pos.x).abs() > 1e-12 || (to.y - from.pos.y).abs() > 1e-12 {
        ops.move_to(to.x, to.y, safe_z, None);
        let travel_pos = Point3D::new(to.x, to.y, safe_z);
        let dx = travel_pos.x - cur_pos.x;
        let dy = travel_pos.y - cur_pos.y;
        let heading = dy.atan2(dx);
        tracer.move_point(
            link_span,
            "workplan",
            MoveKind::Travel,
            ToolSnapshot {
                pos_x: travel_pos.x,
                pos_y: travel_pos.y,
                pos_z: travel_pos.z,
                heading,
                prev_x: cur_pos.x,
                prev_y: cur_pos.y,
                prev_z: cur_pos.z,
            },
            None,
            None,
        );
        cur_pos = travel_pos;
        cur_heading = heading;
    }

    if (entry_z - safe_z).abs() > 1e-12 {
        ops.move_to(to.x, to.y, entry_z, None);
        let plunge_pos = Point3D::new(to.x, to.y, entry_z);
        tracer.move_point(
            link_span,
            "workplan",
            MoveKind::Plunge,
            ToolSnapshot {
                pos_x: plunge_pos.x,
                pos_y: plunge_pos.y,
                pos_z: plunge_pos.z,
                heading: cur_heading,
                prev_x: cur_pos.x,
                prev_y: cur_pos.y,
                prev_z: cur_pos.z,
            },
            None,
            None,
        );
        cur_pos = plunge_pos;
    }

    tracer.event(
        link_span,
        "workplan",
        EventKind::Exit,
        Some(ToolSnapshot {
            pos_x: cur_pos.x,
            pos_y: cur_pos.y,
            pos_z: cur_pos.z,
            heading: cur_heading,
            prev_x: cur_pos.x,
            prev_y: cur_pos.y,
            prev_z: cur_pos.z,
        }),
        None,
    );
    tracer.exit(link_span, "workplan");
}
