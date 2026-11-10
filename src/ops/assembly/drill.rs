//! Hole-making cycles: plain drilling, pecking (full retract and
//! chip-break), spot drilling, and countersinking.
//!
//! Every cycle is expanded into explicit moves — rapid to the safe
//! plane, rapid to the rapid plane above the surface, feed plunges at
//! the plunge feed, rapid retracts, dwell at the bottom — so any
//! encoder and the time estimator see ordinary motion. The whole cycle
//! is wrapped in a [`SectionType::DrillCycle`] section, letting
//! encoders or downstream consumers substitute a canned cycle
//! (G81/G82/G73/G83) for the expanded moves.
//!
//! Plunges are `LineTo` (cutting) at the plunge feed, rapids/retracts
//! are `MoveTo` (travel) — matching the ops-level convention that a
//! plunge is a vertical feed move.
use prof_macros::prof;

use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::shape::polygon::get_circle_polygon;
use crate::geo::types::{Point, Point3D};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::{AssembleCtx, Assembler, Tracelet};
use crate::ops::enums::SectionType;
use crate::ops::part::FaceState;
use crate::ops::state::State;
use crate::ops::types::{OpNode, ToolPose};

/// Clearance the rapid re-entry stops above the previous peck bottom.
/// Shared with the tapping assembler, whose pecks re-enter the same way.
pub(crate) const REENTRY_CLEARANCE: f64 = 0.5;

/// Peck retract strategy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PeckMode {
    /// Retract fully to the rapid plane after each peck, clearing the
    /// flutes from the hole (G83 semantics).
    FullRetract,
    /// Retract only a small amount to break the chip (G73 semantics).
    ChipBreak,
}

/// Peck cycle parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PeckParams {
    pub mode: PeckMode,
    /// Axial depth per peck, measured from the surface.
    pub increment: f64,
    /// Chip-break retract distance above the current depth (mm);
    /// only used with [`PeckMode::ChipBreak`].
    pub retract_amount: f64,
}

/// Spec for the drilling assembler: one cycle at one hole.
///
/// `target_z` is the **full-diameter depth**; the tip descends deeper
/// by the tip compensation `(diameter / 2) / tan(tip_angle / 2)`.
#[derive(Clone, Debug, PartialEq)]
pub struct DrillSpec {
    pub center: Point,
    /// Drill (and hole) diameter; the cleared disk is cut at this
    /// diameter.
    pub diameter: f64,
    /// Point angle of the drill in degrees (e.g. 118, 140, 90 for
    /// spot drills), used for tip compensation.
    pub tip_angle: f64,
    /// Surface Z the hole starts at (toolpath convention: 0.0).
    pub z_surface: f64,
    /// Full-diameter depth of the hole.
    pub target_z: f64,
    /// Rapid plane just above the surface; peck retracts stop here.
    pub rapid_z: f64,
    pub safe_z: f64,
    /// Peck parameters; `None` drills in a single plunge.
    pub peck: Option<PeckParams>,
    /// Dwell at the bottom of every plunge (ms).
    pub dwell_ms: f64,
    /// Marks the cycle as a spot drill: emitted moves are identical to
    /// a plain drill, only `name()` / `PlanStep.kind` differ.
    pub spot: bool,
    /// When true (the default), `target_z` is the full-diameter depth
    /// and the tip descends deeper by the tip compensation. Tools whose
    /// tip defines the final diameter (countersinks) set this false:
    /// `target_z` then is the depth the tip reaches.
    pub tip_compensate: bool,
}

impl DrillSpec {
    /// Depth the drill tip reaches, including tip compensation when
    /// enabled.
    pub fn tip_z(&self) -> f64 {
        if self.tip_compensate {
            self.target_z - self.tip_length()
        } else {
            self.target_z
        }
    }

    /// Length of the conical drill tip below `target_z`.
    pub fn tip_length(&self) -> f64 {
        let half_angle = (self.tip_angle.to_radians()) / 2.0;
        (self.diameter / 2.0) / half_angle.tan()
    }
}

impl Assembler for DrillSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "drill: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = generate_drill(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "drill: done");
        Ok(meta)
    }

    fn name(&self) -> &str {
        match (self.spot, self.peck.map(|p| p.mode)) {
            (true, _) => "spot_drill",
            (false, None) => "plain_drill",
            (false, Some(PeckMode::FullRetract)) => "peck_drill",
            (false, Some(PeckMode::ChipBreak)) => "chip_break_drill",
        }
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Generate one drilling cycle: the expanded explicit moves of a
/// plain, peck or chip-break cycle at [`DrillSpec::center`], wrapped
/// in a `DRILL_CYCLE` section, and cuts the hole disk from the face's
/// cleared area.
#[prof]
pub fn generate_drill(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &DrillSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    validate_drill(opts)?;
    let tip_z = opts.tip_z();

    let start = ToolPose {
        pos: Point3D::new(opts.center.x, opts.center.y, opts.safe_z),
        heading: 0.0,
    };

    trace.ops_section_start(SectionType::DrillCycle, "drill", None)?;
    trace.apply_state(cut_state);

    // Rapid approach down to the rapid plane above the surface.
    trace.move_to(opts.center.x, opts.center.y, opts.safe_z, None);
    trace.move_to(opts.center.x, opts.center.y, opts.rapid_z, None);

    match opts.peck {
        None => plunge_to(trace, opts, cut_state, tip_z),
        Some(peck) => peck_to(trace, opts, cut_state, tip_z, peck),
    }

    // Full retract out of the hole.
    trace.move_to(opts.center.x, opts.center.y, opts.safe_z, None);
    trace.ops_section_end(SectionType::DrillCycle, None)?;

    let disk = vec![get_circle_polygon(opts.center, opts.diameter / 2.0, 64)];
    face.cleared.cut(&disk);

    Ok(AssemblyMeta {
        start,
        end: ToolPose {
            pos: Point3D::new(opts.center.x, opts.center.y, opts.safe_z),
            heading: 0.0,
        },
    })
}

/// Single plunge to `tip_z` at the plunge feed, with dwell.
fn plunge_to(
    trace: &mut Tracelet,
    opts: &DrillSpec,
    cut_state: &State,
    tip_z: f64,
) {
    feed_plunge(trace, opts, cut_state, tip_z);
}

/// Peck to `tip_z` in increments, retracting per [`PeckMode`].
fn peck_to(
    trace: &mut Tracelet,
    opts: &DrillSpec,
    cut_state: &State,
    tip_z: f64,
    peck: PeckParams,
) {
    let depth = opts.z_surface - tip_z;
    let n_pecks = (depth / peck.increment).ceil().max(1.0) as usize;
    let mut previous_bottom = opts.z_surface;
    for k in 1..=n_pecks {
        let bottom = (opts.z_surface - peck.increment * k as f64).max(tip_z);
        match peck.mode {
            PeckMode::ChipBreak => {
                // Rapid back down to the previous bottom; the hole is
                // already cut to there, so the rapid is safe.
                if k > 1 {
                    trace.move_to(
                        opts.center.x,
                        opts.center.y,
                        previous_bottom,
                        None,
                    );
                }
                feed_plunge(trace, opts, cut_state, bottom);
                // Small rapid retract to break the chip.
                let break_z = (bottom + peck.retract_amount).min(opts.rapid_z);
                trace.move_to(opts.center.x, opts.center.y, break_z, None);
            }
            PeckMode::FullRetract => {
                // Rapid back down to just above the previous bottom
                // (clearance avoids rapiding into the un-cut floor).
                if k > 1 {
                    let reentry =
                        (previous_bottom + REENTRY_CLEARANCE).min(opts.rapid_z);
                    trace.move_to(opts.center.x, opts.center.y, reentry, None);
                }
                feed_plunge(trace, opts, cut_state, bottom);
                // Full retract to the rapid plane to clear the flutes.
                trace.move_to(opts.center.x, opts.center.y, opts.rapid_z, None);
            }
        }
        previous_bottom = bottom;
    }
}

/// One feed plunge to `z`, at the plunge feed, with dwell at the
/// bottom. Switches the feed rate in and restores the cut feed after.
fn feed_plunge(
    trace: &mut Tracelet,
    opts: &DrillSpec,
    cut_state: &State,
    z: f64,
) {
    let plunge_feed = cut_state.plunge_feed.or(cut_state.feed_rate);
    if plunge_feed != cut_state.feed_rate {
        trace.set_feed_rate(plunge_feed.unwrap_or(0));
    }
    trace.line_to(opts.center.x, opts.center.y, z, None);
    if plunge_feed != cut_state.feed_rate {
        if let Some(fr) = cut_state.feed_rate {
            trace.set_feed_rate(fr);
        }
    }
    if opts.dwell_ms > 0.0 {
        trace.push_raw(OpNode::dwell(opts.dwell_ms));
    }
}

/// Spec for the countersinking assembler: one conical plunge at one
/// hole.
///
/// Plunging a conical countersink tool of `tip_angle` to the derived
/// depth produces a chamfer of `target_diameter` at the surface.
#[derive(Clone, Debug, PartialEq)]
pub struct CountersinkSpec {
    pub center: Point,
    /// Included angle of the countersink cone in degrees (e.g. 90).
    pub tip_angle: f64,
    /// Chamfer diameter produced at the surface.
    pub target_diameter: f64,
    pub z_surface: f64,
    pub rapid_z: f64,
    pub safe_z: f64,
    pub dwell_ms: f64,
}

impl Assembler for CountersinkSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "countersink: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = generate_countersink(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "countersink: done");
        Ok(meta)
    }

    fn name(&self) -> &str {
        "countersink"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Generate one countersink cycle: a plain plunge to the depth that
/// produces `target_diameter` with the given cone angle, wrapped in a
/// `DRILL_CYCLE` section.
#[prof]
pub fn generate_countersink(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &CountersinkSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    if opts.tip_angle <= 0.0 || opts.tip_angle >= 180.0 {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "tip_angle must be within (0, 180) degrees, got {}",
            opts.tip_angle
        )));
    }
    if opts.target_diameter <= 0.0 {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "target_diameter must be above 0, got {}",
            opts.target_diameter
        )));
    }
    let half_angle = opts.tip_angle.to_radians() / 2.0;
    let depth = (opts.target_diameter / 2.0) / half_angle.tan();
    let drill = DrillSpec {
        center: opts.center,
        // The cone's full diameter at the surface.
        diameter: opts.target_diameter,
        tip_angle: opts.tip_angle,
        z_surface: opts.z_surface,
        // The cone tip defines the depth: no extra tip compensation.
        target_z: opts.z_surface - depth,
        rapid_z: opts.rapid_z,
        safe_z: opts.safe_z,
        peck: None,
        dwell_ms: opts.dwell_ms,
        spot: false,
        tip_compensate: false,
    };
    generate_drill(face, trace, &drill, cut_state)
}

/// Validate the drill spec's geometry parameters.
fn validate_drill(opts: &DrillSpec) -> RaygeoResult<()> {
    if opts.diameter <= 0.0 {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "diameter must be above 0, got {}",
            opts.diameter
        )));
    }
    if opts.tip_angle <= 0.0 || opts.tip_angle >= 180.0 {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "tip_angle must be within (0, 180) degrees, got {}",
            opts.tip_angle
        )));
    }
    if opts.target_z >= opts.z_surface {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "target_z ({}) must be below z_surface ({})",
            opts.target_z, opts.z_surface
        )));
    }
    if opts.rapid_z < opts.z_surface {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "rapid_z ({}) must be at or above z_surface ({})",
            opts.rapid_z, opts.z_surface
        )));
    }
    if opts.safe_z < opts.rapid_z {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "safe_z ({}) must be at or above rapid_z ({})",
            opts.safe_z, opts.rapid_z
        )));
    }
    if let Some(peck) = &opts.peck {
        if peck.increment <= 0.0 {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "peck increment must be above 0, got {}",
                peck.increment
            )));
        }
        if peck.mode == PeckMode::ChipBreak && peck.retract_amount <= 0.0 {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "chip-break retract_amount must be above 0, got {}",
                peck.retract_amount
            )));
        }
    }
    Ok(())
}
