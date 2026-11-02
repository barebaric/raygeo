//! Profile plan builder.
//!
//! [`plan_profile`] takes a [`Part`] and options and produces a plan that
//! profiles the face's boundary: the inset (inner) or grown (outer)
//! offset, walked once at the target depth. It is the contour counterpart
//! of [`plan_clearing`](super::clearing::plan_clearing) — clearing
//! pockets, profiling walls — and drives the same [`ProfileSpec`] the
//! clearing planner uses for its finishing passes.
//!
//! With a `ramp_angle` the plan composes a
//! [`ProfileRampSpec`] entry step before the profile pass: the tool ramps
//! down along the same tool-center loop the pass will walk, so it enters
//! the material gradually instead of plunging. Like the exact preference
//! itself, the entry needs curve-true face geometry to derive the loop;
//! polygon-only faces get no entry step. The profile pass's own descent
//! then lands in the channel the ramp cut.
//!
//! The boundary and islands are read from the specified face; the steps
//! target that same face. Feeds ride the plan's cut state.

use std::sync::Arc;

use prof_macros::prof;

use crate::cnc::plan::plan::{Plan, PlanStep};
use crate::error::{RaygeoError, RaygeoResult};
use crate::ops::assembly::profile::{
    ProfileKind, ProfileRampSpec, ProfileSpec,
};
use crate::ops::part::Part;
use crate::ops::types::CutDirection;

/// Options for [`plan_profile`].
pub struct ProfileWorkplanOptions {
    /// Profile the inset (inner) or grown (outer) boundary.
    pub side: ProfileKind,
    pub tool_radius: f64,
    pub step_over: f64,
    pub step_length: f64,
    pub target_z: f64,
    pub safe_z: f64,
    pub wall_margin: f64,
    pub stock_to_leave: f64,
    pub cut_direction: CutDirection,
    /// Steepest entry descent along the profile loop (radians); 0 enters
    /// with a straight plunge.
    pub ramp_angle: f64,
    /// Boundary-following tolerance; also the flattening tolerance of the
    /// exact path's line chains.
    pub tolerance: f64,
    /// Cut feed (mm/min); `None` leaves the plan's cut state unset.
    pub feed_rate: Option<f64>,
    /// Vertical plunge feed (mm/min) for descents into material; `None`
    /// falls back to the cut feed.
    pub plunge_feed: Option<f64>,
}

/// Builds a profile plan for one face of `part`.
#[prof]
pub fn plan_profile(
    part: &Part,
    face_id: &str,
    opts: &ProfileWorkplanOptions,
) -> RaygeoResult<Plan> {
    // Kept for workplan parity with clearing; a single profile pass does
    // not step over, but a nonsense value is still rejected.
    if opts.step_over <= 0.0 || opts.step_over > 2.0 * opts.tool_radius {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "step_over must be above 0 and at most the tool diameter ({}), got {}",
            2.0 * opts.tool_radius,
            opts.step_over
        )));
    }
    let Some(face) = part.face(face_id) else {
        return Err(RaygeoError::DegenerateGeometry(format!(
            "the part has no face {face_id:?}"
        )));
    };
    let (boundary, islands) = face.extract_boundary();
    let Some(boundary) = boundary else {
        return Err(RaygeoError::DegenerateGeometry(
            "the face has no boundary to profile".into(),
        ));
    };
    let mut plan = Plan::new(boundary, islands, opts.safe_z);
    // The ramp entry runs where the pass will walk, so it needs the same
    // curve-true geometry the exact path derives the loop from.
    if opts.ramp_angle > 0.0 && face.geometry.is_some() {
        plan.steps.push(PlanStep {
            face_id: face_id.to_string(),
            spec: Arc::new(ProfileRampSpec {
                kind: opts.side,
                tool_radius: opts.tool_radius,
                wall_margin: opts.wall_margin,
                stock_to_leave: opts.stock_to_leave,
                ramp_angle: opts.ramp_angle,
                z_start: opts.safe_z,
                z_end: opts.target_z,
                cut_direction: opts.cut_direction,
                tolerance: opts.tolerance,
            }),
            region_boundary: None,
        });
    }
    plan.steps.push(PlanStep {
        face_id: face_id.to_string(),
        spec: Arc::new(ProfileSpec {
            kind: opts.side,
            tool_radius: opts.tool_radius,
            step_over: opts.step_over,
            step_length: opts.step_length,
            target_z: opts.target_z,
            safe_z: opts.safe_z,
            wall_margin: opts.wall_margin,
            stock_to_leave: opts.stock_to_leave,
            cut_direction: opts.cut_direction,
            start_pos: None,
            tolerance: opts.tolerance,
            expansion_batch_size: 20,
            cancel_check: None,
            engagement_area_threshold: 1.0,
            engagement_angle_threshold: 30.0,
            feed_reduction_factor: 0.5,
            trace_path: None,
        }),
        region_boundary: None,
    });
    plan.cut_state.feed_rate = opts.feed_rate.map(|f| f.round() as i32);
    plan.cut_state.plunge_feed = opts.plunge_feed.map(|f| f.round() as i32);
    Ok(plan)
}
