//! Rigid tapping: one internal thread per cycle by synchronous
//! spindle-fed tapping.
//!
//! The tap rapids to the pilot bore, and descends with the spindle
//! synchronized to the Z feed — the moves travel `pitch` millimetres
//! per spindle revolution, i.e. at `pitch × rpm` mm/min. At the
//! bottom the spindle reverses and the synchronized retract up to the
//! rapid plane unscrews the tap; then the cutting direction is
//! restored and the tap rapids out to the safe plane.
//!
//! A right-hand thread taps clockwise and retracts counter-clockwise
//! (G84 semantics); a left-hand thread is the mirror (G74).
//!
//! The whole cycle is expanded into explicit moves wrapped in a
//! [`SectionType::TappingCycle`] section, letting encoders or
//! downstream consumers substitute a canned cycle (G84/G74) for the
//! expanded moves. The synchronization itself travels as state
//! commands — `SetSpindleDirection` and `SetSyncPitch` — so the moves
//! stay ordinary `LineTo` motion whose feed the time estimator and
//! encoders derive from `pitch × rpm`.
//!
//! The pilot bore is assumed pre-drilled to the tap drill diameter;
//! optional pecking fully retracts to the rapid plane between
//! increments to clear chips (G84 Q semantics).
use prof_macros::prof;

use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::shape::polygon::get_circle_polygon;
use crate::geo::types::{Point, Point3D};
use crate::ops::assembly::drill::REENTRY_CLEARANCE;
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::thread_mill::ThreadHandedness;
use crate::ops::assembly::{AssembleCtx, Assembler, Tracelet};
use crate::ops::enums::SectionType;
use crate::ops::part::FaceState;
use crate::ops::state::{SpindleDirection, State};
use crate::ops::types::{OpNode, ToolPose};

/// Spec for the tapping assembler: one rigid-tapping cycle at one
/// pilot bore.
#[derive(Clone, Debug, PartialEq)]
pub struct TapSpec {
    pub center: Point,
    /// Nominal (major) diameter of the thread; the cleared disk is cut
    /// at this diameter.
    pub nominal_diameter: f64,
    /// Thread pitch (lead): axial travel per spindle revolution.
    pub pitch: f64,
    pub handedness: ThreadHandedness,
    /// Surface Z the hole starts at (toolpath convention: 0.0).
    pub z_surface: f64,
    /// Depth of the thread (full depth, no tip compensation — a tap
    /// has a full-diameter cutting end beyond its chamfer).
    pub target_z: f64,
    /// Rapid plane just above the surface; synchronized retracts and
    /// peck re-entries stop here.
    pub rapid_z: f64,
    pub safe_z: f64,
    /// Peck increment (axial depth per peck); `None` taps in a single
    /// synchronized pass.
    pub peck: Option<f64>,
    /// Dwell at the bottom of the hole before the spindle reversal (ms).
    pub dwell_ms: f64,
}

impl TapSpec {
    /// Validate the spec. Also run by the tapping planner so bad
    /// thread geometry is rejected at plan time.
    pub fn validate(&self) -> RaygeoResult<()> {
        if self.nominal_diameter <= 0.0 {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "nominal_diameter must be above 0, got {}",
                self.nominal_diameter
            )));
        }
        if self.pitch <= 0.0 {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "pitch must be above 0, got {}",
                self.pitch
            )));
        }
        if self.target_z >= self.z_surface {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "target_z ({}) must be below z_surface ({})",
                self.target_z, self.z_surface
            )));
        }
        if self.rapid_z < self.z_surface {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "rapid_z ({}) must be at or above z_surface ({})",
                self.rapid_z, self.z_surface
            )));
        }
        if self.safe_z < self.rapid_z {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "safe_z ({}) must be at or above rapid_z ({})",
                self.safe_z, self.rapid_z
            )));
        }
        if let Some(increment) = self.peck {
            if increment <= 0.0 {
                return Err(RaygeoError::DegenerateGeometry(format!(
                    "peck increment must be above 0, got {}",
                    increment
                )));
            }
        }
        Ok(())
    }

    /// Spindle direction while tapping down (the cutting direction).
    fn cut_direction(&self) -> SpindleDirection {
        match self.handedness {
            ThreadHandedness::Rh => SpindleDirection::Cw,
            ThreadHandedness::Lh => SpindleDirection::Ccw,
        }
    }

    /// Spindle direction while retracting (the reversal).
    fn retract_direction(&self) -> SpindleDirection {
        match self.handedness {
            ThreadHandedness::Rh => SpindleDirection::Ccw,
            ThreadHandedness::Lh => SpindleDirection::Cw,
        }
    }
}

impl Assembler for TapSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "tap: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = generate_tap(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "tap: done");
        Ok(meta)
    }

    fn name(&self) -> &str {
        "tap"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Generate one rigid-tapping cycle at [`TapSpec::center`], wrapped in
/// a `TAPPING_CYCLE` section, and cuts the thread's disk at the
/// nominal diameter from the face's cleared area.
#[prof]
pub fn generate_tap(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &TapSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    opts.validate()?;
    let rpm = cut_state.spindle_rpm.ok_or_else(|| {
        RaygeoError::DegenerateGeometry(
            "tapping requires a spindle speed (spindle_rpm) to \
             synchronize the feed with"
                .to_string(),
        )
    })?;
    if rpm == 0 {
        return Err(RaygeoError::DegenerateGeometry(
            "tapping requires a spindle speed above 0".to_string(),
        ));
    }

    let cut_dir = opts.cut_direction();
    let retract_dir = opts.retract_direction();
    let c = opts.center;

    let start = ToolPose {
        pos: Point3D::new(c.x, c.y, opts.safe_z),
        heading: 0.0,
    };

    trace.ops_section_start(SectionType::TappingCycle, "tap", None)?;
    trace.apply_state(cut_state);

    // Spindle-synchronized state for the cycle's feed moves.
    trace.push_raw(OpNode::set_spindle_direction(cut_dir));
    trace.push_raw(OpNode::set_sync_pitch(opts.pitch));

    // Rapid approach down to the rapid plane above the surface.
    trace.move_to(c.x, c.y, opts.safe_z, None);
    trace.move_to(c.x, c.y, opts.rapid_z, None);

    // Tap down in pecks or one pass. Every descent and every retract
    // is a feed move under the active sync pitch.
    match opts.peck {
        None => {
            trace.line_to(c.x, c.y, opts.target_z, None);
        }
        Some(increment) => {
            let depth = opts.z_surface - opts.target_z;
            let n_pecks = (depth / increment).ceil().max(1.0) as usize;
            for k in 1..=n_pecks {
                let bottom =
                    (opts.z_surface - increment * k as f64).max(opts.target_z);
                trace.line_to(c.x, c.y, bottom, None);
                if k < n_pecks {
                    // Synchronized retract out of the hole to clear
                    // the flutes, then rapid re-entry just above the
                    // previous bottom.
                    trace.line_to(c.x, c.y, opts.rapid_z, None);
                    let reentry =
                        (bottom + REENTRY_CLEARANCE).min(opts.rapid_z);
                    trace.move_to(c.x, c.y, reentry, None);
                }
            }
        }
    }

    if opts.dwell_ms > 0.0 {
        trace.push_raw(OpNode::dwell(opts.dwell_ms));
    }

    // Reverse the spindle and unscrew the tap with a synchronized
    // retract up to the rapid plane.
    trace.push_raw(OpNode::set_spindle_direction(retract_dir));
    trace.line_to(c.x, c.y, opts.rapid_z, None);

    // Restore the cutting direction, drop the sync, and rapid out.
    trace.push_raw(OpNode::set_spindle_direction(cut_dir));
    trace.push_raw(OpNode::set_sync_pitch(0.0));
    trace.move_to(c.x, c.y, opts.safe_z, None);
    trace.ops_section_end(SectionType::TappingCycle, None)?;

    let disk = vec![get_circle_polygon(c, opts.nominal_diameter / 2.0, 64)];
    face.cleared.cut(&disk);

    Ok(AssemblyMeta {
        start,
        end: ToolPose {
            pos: Point3D::new(c.x, c.y, opts.safe_z),
            heading: 0.0,
        },
    })
}
