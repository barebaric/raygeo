//! Tapping plan builder.
//!
//! [`plan_tapping`] takes a [`Part`] and options and produces a plan
//! that rigid-taps internal threads: one synchronized spindle-fed
//! cycle per hole, all sharing the same thread parameters. The holes
//! are pilot bores: either passed explicitly as centers or detected
//! automatically as circular island contours of the face.
//!
//! Tapping feeds derive from the thread itself — every synchronized
//! move travels `pitch × rpm` mm/min — so unlike drilling there are
//! no feed options; the spindle speed is mandatory. Detected bores are
//! validated against the thread geometry: a bore at or above the
//! nominal diameter has nothing left to tap. Holes needing different
//! parameters need a separate plan.

use std::sync::Arc;

use prof_macros::prof;

use crate::cnc::plan::drilling::{
    order_holes_nearest_neighbor, resolve_holes, HoleOrder,
};
use crate::cnc::plan::plan::{Plan, PlanStep};
use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::types::Point;
use crate::ops::assembly::tap::TapSpec;
use crate::ops::assembly::thread_mill::ThreadHandedness;
use crate::ops::part::Part;
use crate::ops::CoolantMode;

/// Options for [`plan_tapping`].
pub struct TappingWorkplanOptions {
    /// Nominal (major) diameter of the thread.
    pub nominal_diameter: f64,
    /// Thread pitch (lead): axial travel per spindle revolution.
    pub pitch: f64,
    pub handedness: ThreadHandedness,
    /// Surface Z the holes start at (toolpath convention: 0.0).
    pub z_surface: f64,
    /// Depth of the thread.
    pub target_z: f64,
    /// Rapid plane just above the surface.
    pub rapid_z: f64,
    pub safe_z: f64,
    /// Peck increment; `None` taps in a single synchronized pass.
    pub peck: Option<f64>,
    /// Dwell at the bottom of the hole before the spindle reversal (ms).
    pub dwell_ms: f64,
    /// Spindle speed. Mandatory: the synchronized feed is
    /// `pitch × spindle_rpm` mm/min.
    pub spindle_rpm: Option<u32>,
    pub coolant: Option<CoolantMode>,
    /// Explicit pilot-bore centers. When `None`, bores are detected as
    /// circular island contours of the face and validated against the
    /// thread geometry.
    pub holes: Option<Vec<Point>>,
    /// Max distance (mm) between an island contour sample and the
    /// fitted circle for it to count as a bore.
    pub hole_tolerance: f64,
    pub hole_order: HoleOrder,
}

/// Builds a rigid-tapping plan for one face of `part`.
#[prof]
pub fn plan_tapping(
    part: &Part,
    face_id: &str,
    opts: &TappingWorkplanOptions,
) -> RaygeoResult<Plan> {
    let face = part.face(face_id).ok_or_else(|| {
        RaygeoError::DegenerateGeometry(format!(
            "the part has no face {face_id:?}"
        ))
    })?;
    let (boundary, islands) = face.extract_boundary();
    if opts.spindle_rpm.is_none() || opts.spindle_rpm == Some(0) {
        return Err(RaygeoError::DegenerateGeometry(
            "tapping requires a spindle speed (spindle_rpm) to \
             synchronize the feed with"
                .to_string(),
        ));
    }
    // Validate the shared thread geometry (pitch, depths, peck) at
    // plan time via a probe spec.
    TapSpec {
        center: Point::default(),
        nominal_diameter: opts.nominal_diameter,
        pitch: opts.pitch,
        handedness: opts.handedness,
        z_surface: opts.z_surface,
        target_z: opts.target_z,
        rapid_z: opts.rapid_z,
        safe_z: opts.safe_z,
        peck: opts.peck,
        dwell_ms: opts.dwell_ms,
    }
    .validate()?;
    let holes = resolve_holes(
        opts.holes.as_deref(),
        &islands,
        opts.hole_tolerance,
        // A bore has to admit the tap at all: use the minor diameter
        // as the minimum detectable size.
        opts.nominal_diameter - 1.5 * opts.pitch,
    )?;
    if opts.holes.is_none() {
        for hole in &holes {
            if hole.diameter >= opts.nominal_diameter {
                return Err(RaygeoError::DegenerateGeometry(format!(
                    "detected bore of diameter {} is at or above the \
                     nominal diameter of the {}x{} thread; there is \
                     nothing left to tap",
                    hole.diameter, opts.nominal_diameter, opts.pitch
                )));
            }
        }
    }
    let holes = match opts.hole_order {
        HoleOrder::DetectionOrder => holes,
        HoleOrder::NearestNeighbor => order_holes_nearest_neighbor(holes),
    };

    let mut plan =
        Plan::new(boundary.unwrap_or_default(), islands, opts.safe_z);
    for hole in &holes {
        plan.steps.push(PlanStep {
            face_id: face_id.to_string(),
            spec: Arc::new(TapSpec {
                center: hole.center,
                nominal_diameter: opts.nominal_diameter,
                pitch: opts.pitch,
                handedness: opts.handedness,
                z_surface: opts.z_surface,
                target_z: opts.target_z,
                rapid_z: opts.rapid_z,
                safe_z: opts.safe_z,
                peck: opts.peck,
                dwell_ms: opts.dwell_ms,
            }),
            region_boundary: None,
        });
    }
    plan.cut_state.spindle_rpm = opts.spindle_rpm;
    plan.cut_state.coolant = opts.coolant;
    Ok(plan)
}
