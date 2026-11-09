//! Thread milling: one internal thread per cycle via helical
//! interpolation.
//!
//! The tool rapids to the bore center, plunges to the bottom of the
//! thread (the pilot bore is assumed pre-drilled), swings out on a
//! 180° tangential lead-in arc to the orbit radius, climbs the thread
//! as a helical interpolation (half-turn arcs, one pitch per
//! revolution), and swings back to the center on a lead-out arc.
//!
//! The orbit radius is `(nominal_diameter - tool_diameter) / 2`; the
//! thread length must be an integer multiple of the pitch — the
//! constraint a full-profile thread mill actually works under.
//!
//! A right-hand thread climbs counter-clockwise, a left-hand thread
//! clockwise (viewed from +Z); the lead-in/out arcs sweep in the
//! opposite direction so they join the orbit tangentially.
use prof_macros::prof;

use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::shape::polygon::get_circle_polygon;
use crate::geo::types::{Point, Point3D};
use crate::ops::assembly::result::AssemblyMeta;
use crate::ops::assembly::{AssembleCtx, Assembler, Tracelet};
use crate::ops::part::FaceState;
use crate::ops::state::State;
use crate::ops::types::ToolPose;

/// Thread handedness, determining the orbit direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadHandedness {
    /// Right-hand thread: counter-clockwise orbit when climbing.
    Rh,
    /// Left-hand thread: clockwise orbit when climbing.
    Lh,
}

/// Spec for the thread-milling assembler: one internal thread.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreadMillSpec {
    pub center: Point,
    /// Nominal (major) diameter of the thread.
    pub nominal_diameter: f64,
    /// Thread pitch (lead): axial travel per revolution.
    pub pitch: f64,
    /// Diameter of the thread mill cutter.
    pub tool_diameter: f64,
    /// Z of the top end of the thread (milling runs bottom-up).
    pub z_top: f64,
    /// Z of the bottom end of the thread.
    pub z_bottom: f64,
    pub handedness: ThreadHandedness,
    pub safe_z: f64,
}

impl ThreadMillSpec {
    /// Validate the spec; returns the orbit radius on success. Also
    /// run by the thread-milling planner so bad thread geometry is
    /// rejected at plan time.
    pub fn validate(&self) -> RaygeoResult<f64> {
        if self.nominal_diameter <= 0.0 {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "nominal_diameter must be above 0, got {}",
                self.nominal_diameter
            )));
        }
        if self.tool_diameter <= 0.0 {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "tool_diameter must be above 0, got {}",
                self.tool_diameter
            )));
        }
        let orbit_r = orbit_radius(self.nominal_diameter, self.tool_diameter);
        if orbit_r <= 0.0 {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "tool_diameter ({}) must be below the nominal diameter ({})",
                self.tool_diameter, self.nominal_diameter
            )));
        }
        if self.pitch <= 0.0 {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "pitch must be above 0, got {}",
                self.pitch
            )));
        }
        if self.z_top <= self.z_bottom {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "z_top ({}) must be above z_bottom ({})",
                self.z_top, self.z_bottom
            )));
        }
        if self.safe_z < self.z_top {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "safe_z ({}) must be at or above z_top ({})",
                self.safe_z, self.z_top
            )));
        }
        let revs_f = (self.z_top - self.z_bottom) / self.pitch;
        if (revs_f - revs_f.round()).abs() > 1e-3 || revs_f.round() < 1.0 {
            return Err(RaygeoError::DegenerateGeometry(format!(
                "thread length ({}) must be a positive integer multiple of the pitch ({})",
                self.z_top - self.z_bottom,
                self.pitch
            )));
        }
        Ok(orbit_r)
    }
}

impl Assembler for ThreadMillSpec {
    fn assemble(&self, ctx: &mut AssembleCtx) -> Result<AssemblyMeta, String> {
        ctx.callbacks.report_progress(0.0, "thread_mill: assemble");
        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }
        let meta = generate_thread_mill(ctx.face, ctx.trace, self, ctx.state)
            .map_err(|e| e.to_string())?;
        ctx.callbacks.report_progress(1.0, "thread_mill: done");
        Ok(meta)
    }

    fn name(&self) -> &str {
        "thread_mill"
    }

    fn boxed_clone(&self) -> Box<dyn Assembler> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Radius the tool center orbits at.
fn orbit_radius(nominal_diameter: f64, tool_diameter: f64) -> f64 {
    (nominal_diameter - tool_diameter) / 2.0
}

/// Generate one thread-milling cycle at [`ThreadMillSpec::center`].
///
/// The cleared area gains a disk at the thread's major diameter —
/// matching the helix assembler's convention of clearing the tool's
/// full swept envelope, so later steps on the same face treat the
/// bore as finished.
#[prof]
pub fn generate_thread_mill(
    face: &mut FaceState,
    trace: &mut Tracelet,
    opts: &ThreadMillSpec,
    cut_state: &State,
) -> RaygeoResult<AssemblyMeta> {
    let orbit_r = opts.validate()?;
    let ccw_orbit = opts.handedness == ThreadHandedness::Rh;
    let arc_cw = !ccw_orbit; // Lead-in/out sweep opposite to the orbit.
    let revs_f = (opts.z_top - opts.z_bottom) / opts.pitch;
    let revs = revs_f.round();
    debug_assert!((revs_f - revs).abs() < 1e-3 && revs >= 1.0);
    let revs = revs as usize;

    let c = opts.center;
    // Orbit entry point: 0° on the orbit circle.
    let entry = Point3D::new(c.x + orbit_r, c.y, opts.z_bottom);
    // Lead arc midpoint: halfway between center and entry point; the
    // 180° arc around it joins the orbit tangentially at the entry.
    let lead_mid = Point3D::new(c.x + orbit_r / 2.0, c.y, 0.0);

    let start = ToolPose {
        pos: Point3D::new(c.x, c.y, opts.safe_z),
        heading: 0.0,
    };

    trace.apply_state(cut_state);
    trace.move_to(c.x, c.y, opts.safe_z, None);

    // Plunge to the bottom of the thread in the pre-drilled pilot bore.
    let plunge_feed = cut_state.plunge_feed.or(cut_state.feed_rate);
    if plunge_feed != cut_state.feed_rate {
        trace.set_feed_rate(plunge_feed.unwrap_or(0));
    }
    trace.line_to(c.x, c.y, opts.z_bottom, None);
    if plunge_feed != cut_state.feed_rate {
        if let Some(fr) = cut_state.feed_rate {
            trace.set_feed_rate(fr);
        }
    }

    // 180° lead-in arc from the center to the orbit entry point.
    trace.arc_to(
        entry.x,
        entry.y,
        lead_mid.x - c.x,
        lead_mid.y - c.y,
        arc_cw,
        opts.z_bottom,
        None,
    );

    // Helix up in half-turn arcs: one pitch per full revolution.
    for k in 0..(revs * 2) {
        let a0 = sign_angle(k as f64 * std::f64::consts::PI, ccw_orbit);
        let a1 = sign_angle((k + 1) as f64 * std::f64::consts::PI, ccw_orbit);
        let z1 = opts.z_bottom + (k + 1) as f64 * opts.pitch / 2.0;
        let s = Point3D::new(
            c.x + orbit_r * a0.cos(),
            c.y + orbit_r * a0.sin(),
            opts.z_bottom + k as f64 * opts.pitch / 2.0,
        );
        let e = Point3D::new(
            c.x + orbit_r * a1.cos(),
            c.y + orbit_r * a1.sin(),
            z1,
        );
        trace.arc_to(e.x, e.y, c.x - s.x, c.y - s.y, arc_cw, e.z, None);
    }

    // 180° lead-out arc back to the center (helix ends at the entry
    // point because the thread length is an integer multiple of the
    // pitch).
    trace.arc_to(
        c.x,
        c.y,
        lead_mid.x - entry.x,
        lead_mid.y - entry.y,
        arc_cw,
        opts.z_top,
        None,
    );

    // Retract out of the bore.
    trace.move_to(c.x, c.y, opts.safe_z, None);

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

/// Map a CCW sweep angle to the actual angle for the orbit direction
/// (negative angles for a CW orbit).
fn sign_angle(angle: f64, ccw: bool) -> f64 {
    if ccw {
        angle
    } else {
        -angle
    }
}
