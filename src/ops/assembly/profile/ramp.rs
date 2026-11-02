//! Ramp entry along the profile loop.
//!
//! A small entry assembler the plan layer composes before a profile step:
//! it follows the same exact tool-center loop the profile pass will walk —
//! round as many laps as the drop takes at the requested angle — so the
//! tool enters the material gradually and the wall is never nicked by a
//! vertical bore. Arc pieces come out as helical arc moves; the descent
//! runs at the plunge feed.

use prof_macros::prof;

use crate::error::RaygeoResult;
use crate::geo::types::Point3D;
use crate::ops::assembly::profile::exact::{classify_loops, ExactLoop};
use crate::ops::assembly::profile::ProfileKind;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::trace_utils as tu;
use crate::ops::assembly::{AssembleCtx, Assembler, Tracelet};
use crate::ops::part::FaceState;
use crate::ops::state::State;
use crate::ops::types::{CutDirection, ToolPose};

/// Most laps the ramp rounds the loop before falling back to a straight
/// plunge (a loop too short for the drop at the requested angle).
const MAX_LAPS: usize = 500;

/// Spec for the profile ramp-entry assembler.
///
/// Derives the same tool-center loop the profile pass walks — same offset
/// parameters, same largest-loop choice — and ramps down it from
/// `z_start` to `z_end`. Compose it before a
/// [`ProfileSpec`](super::ProfileSpec) step with matching parameters.
#[derive(Clone, Debug)]
pub struct ProfileRampSpec {
    /// Which profile the ramp leads into; picks the offset side and the
    /// island handling of the loop derivation.
    pub kind: ProfileKind,
    pub tool_radius: f64,
    pub wall_margin: f64,
    pub stock_to_leave: f64,
    /// Steepest descent along the loop (radians).
    pub ramp_angle: f64,
    pub z_start: f64,
    pub z_end: f64,
    pub cut_direction: CutDirection,
    /// Boundary-following tolerance of the loop derivation.
    pub tolerance: f64,
}

impl Assembler for ProfileRampSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "profile_ramp: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = ramp_entry(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "profile_ramp: done");
        Ok(meta)
    }

    fn name(&self) -> &str {
        "profile_ramp"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Emits the ramp descent along the profile loop.
#[prof]
pub(crate) fn ramp_entry(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &ProfileRampSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let zero = || AssemblyMeta {
        start: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
        end: ToolPose {
            pos: Point3D::ZERO,
            heading: 0.0,
        },
    };
    let Some(geometry) = face.geometry.clone() else {
        // No curve-true geometry: the loop the profile pass will walk is
        // unknown here, so there is nothing to ramp along.
        return Ok(zero());
    };
    let offset_dist = opts.tool_radius + opts.wall_margin + opts.stock_to_leave;
    let offset = match opts.kind {
        ProfileKind::Outer => offset_dist,
        ProfileKind::Inner => -offset_dist,
    };
    let keep_outer_only = opts.kind == ProfileKind::Outer;
    let Some((mut loops, _islands)) =
        classify_loops(&geometry, offset, opts.tolerance, keep_outer_only)
    else {
        return Ok(zero());
    };
    loops.truncate(1);
    let Some((loop_, _)) = loops.pop() else {
        return Ok(zero());
    };

    let start = loop_.start();
    let forward = opts.cut_direction == CutDirection::Ccw;
    let heading = loop_.heading_at_start(forward);
    let drop = opts.z_start - opts.z_end;
    let needed = if opts.ramp_angle > 1e-9 && drop > 1e-9 {
        drop / opts.ramp_angle.tan()
    } else {
        0.0
    };
    let laps = (needed / loop_.perimeter()).ceil();
    if !(laps.is_finite()) || laps < 1.0 || laps as usize > MAX_LAPS {
        // Nowhere to ramp: descend straight, at the plunge feed.
        let plunge_feed = cut_state.plunge_feed.or(cut_state.feed_rate);
        if plunge_feed != cut_state.feed_rate {
            trace.set_feed_rate(plunge_feed.unwrap_or(0));
        }
        trace.move_to(start.x, start.y, opts.z_start, None);
        trace.line_to(start.x, start.y, opts.z_end, None);
        if plunge_feed != cut_state.feed_rate {
            if let Some(fr) = cut_state.feed_rate {
                trace.set_feed_rate(fr);
            }
        }
        let end = ToolPose {
            pos: Point3D::new(start.x, start.y, opts.z_end),
            heading,
        };
        return Ok(AssemblyMeta { start: end, end });
    }

    trace.move_to(start.x, start.y, opts.z_start, None);
    trace.apply_state(cut_state);
    let plunge_feed = cut_state.plunge_feed.or(cut_state.feed_rate);
    if plunge_feed != cut_state.feed_rate {
        trace.set_feed_rate(plunge_feed.unwrap_or(0));
    }
    loop_.emit_ramp(trace, laps as usize, opts.z_start, opts.z_end, forward);
    if plunge_feed != cut_state.feed_rate {
        if let Some(fr) = cut_state.feed_rate {
            trace.set_feed_rate(fr);
        }
    }
    trace.cut(
        tu::tool_snapshot(
            Point3D::new(start.x, start.y, opts.z_end),
            heading,
            Point3D::new(start.x, start.y, opts.z_start),
        ),
        None,
    );

    // The swept band counts as cleared, so a following roughing pass sees
    // the ramped stretch the way the profile engine reports its own steps.
    mark_band_cleared(face, &loop_, opts.tool_radius, opts.tolerance);

    let end = ToolPose {
        pos: Point3D::new(start.x, start.y, opts.z_end),
        heading,
    };
    Ok(AssemblyMeta { start: end, end })
}

/// Marks the tool-center loop's swept band as cleared.
fn mark_band_cleared(
    face: &mut FaceState,
    loop_: &ExactLoop,
    tool_radius: f64,
    tolerance: f64,
) {
    let mut g = crate::geo::geometry::Geometry::new();
    let mut first = true;
    for (start, command) in loop_.spans() {
        if first {
            g.move_to(start.x, start.y, start.z);
            first = false;
        }
        match command {
            crate::geo::types::Command::Line { end } => {
                g.line_to(end.x, end.y, end.z);
            }
            crate::geo::types::Command::Arc {
                end,
                center_offset,
                normal,
            } => {
                g.arc_to(
                    end.x,
                    end.y,
                    center_offset.x,
                    center_offset.y,
                    normal.z < 0.0,
                    end.z,
                );
            }
            crate::geo::types::Command::Bezier {
                end,
                control1,
                control2,
            } => {
                g.bezier_to(*control1, *control2, *end);
            }
            crate::geo::types::Command::Move { end } => {
                g.line_to(end.x, end.y, end.z);
            }
        }
    }
    let polys = g.to_polygons(tolerance.max(1e-6));
    let band: Vec<crate::geo::types::Polygon> = polys
        .iter()
        .flat_map(|p| {
            crate::geo::shape::polygon::offset_polygon(
                p,
                tool_radius,
                crate::geo::shape::polygon::JoinStyle::Round,
            )
        })
        .collect();
    if !band.is_empty() {
        face.cleared.cut(&band);
    }
}
