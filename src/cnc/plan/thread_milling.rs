//! Thread-milling plan builder.
//!
//! [`plan_thread_milling`] takes a [`Part`] and options and produces a
//! plan that thread-mills internal threads: one helical-interpolation
//! cycle per hole, all sharing the same thread parameters. The holes
//! are pilot bores: either passed explicitly as centers or detected
//! automatically as circular island contours of the face.
//!
//! Detected bores are validated against the thread geometry: the
//! thread mill must fit into the bore, and the bore must not be
//! oversized beyond the thread minor diameter (or there is nothing
//! left to thread). Holes needing different parameters need a
//! separate plan.

use std::sync::Arc;

use prof_macros::prof;

use crate::cnc::plan::drilling::{
    order_holes_nearest_neighbor, resolve_holes, HoleOrder,
};
use crate::cnc::plan::plan::{Plan, PlanStep};
use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::types::Point;
use crate::ops::assembly::thread_mill::{ThreadHandedness, ThreadMillSpec};
use crate::ops::part::Part;
use crate::ops::CoolantMode;

/// Options for [`plan_thread_milling`].
pub struct ThreadMillingWorkplanOptions {
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
    /// Cut feed (mm/min); `None` leaves the plan's cut state unset.
    pub feed_rate: Option<f64>,
    /// Vertical plunge feed (mm/min); `None` falls back to the cut feed.
    pub plunge_feed: Option<f64>,
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

/// Builds a thread-milling plan for one face of `part`.
#[prof]
pub fn plan_thread_milling(
    part: &Part,
    face_id: &str,
    opts: &ThreadMillingWorkplanOptions,
) -> RaygeoResult<Plan> {
    let face = part.face(face_id).ok_or_else(|| {
        RaygeoError::DegenerateGeometry(format!(
            "the part has no face {face_id:?}"
        ))
    })?;
    let (boundary, islands) = face.extract_boundary();
    let minor_diameter = opts.nominal_diameter - 1.5 * opts.pitch;
    // Validate the shared thread geometry (pitch, heights, integer
    // revolution count) at plan time via a probe spec.
    ThreadMillSpec {
        center: Point::default(),
        nominal_diameter: opts.nominal_diameter,
        pitch: opts.pitch,
        tool_diameter: opts.tool_diameter,
        z_top: opts.z_top,
        z_bottom: opts.z_bottom,
        handedness: opts.handedness,
        safe_z: opts.safe_z,
    }
    .validate()?;
    let holes = resolve_holes(
        opts.holes.as_deref(),
        &islands,
        opts.hole_tolerance,
        // The thread mill must enter the bore.
        opts.tool_diameter,
    )?;
    if opts.holes.is_none() {
        for hole in &holes {
            if hole.diameter > minor_diameter {
                return Err(RaygeoError::DegenerateGeometry(format!(
                    "detected bore of diameter {} is oversized for a \
                     {}x{} thread (minor diameter ≈ {}); there is \
                     nothing left to thread",
                    hole.diameter,
                    opts.nominal_diameter,
                    opts.pitch,
                    minor_diameter
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
            spec: Arc::new(ThreadMillSpec {
                center: hole.center,
                nominal_diameter: opts.nominal_diameter,
                pitch: opts.pitch,
                tool_diameter: opts.tool_diameter,
                z_top: opts.z_top,
                z_bottom: opts.z_bottom,
                handedness: opts.handedness,
                safe_z: opts.safe_z,
            }),
            region_boundary: None,
        });
    }
    plan.cut_state.feed_rate = opts.feed_rate.map(|f| f.round() as i32);
    plan.cut_state.plunge_feed = opts.plunge_feed.map(|f| f.round() as i32);
    plan.cut_state.spindle_rpm = opts.spindle_rpm;
    plan.cut_state.coolant = opts.coolant;
    Ok(plan)
}
