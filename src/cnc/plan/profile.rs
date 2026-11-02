//! Profile plan builder.
//!
//! [`plan_profile`] takes a [`Part`] and options and produces a one-step
//! [`Plan`] that profiles the face's boundary: the inset (inner) or grown
//! (outer) offset, walked once at the target depth. It is the contour
//! counterpart of [`plan_clearing`](super::clearing::plan_clearing) —
//! clearing pockets, profiling walls — and its finishing steps are the
//! same [`ProfileSpec`] this planner drives on its own.
//!
//! With `exact` the step follows the exact line-and-arc offset
//! ([`crate::geo::algo::offset_exact`]) and emits arcs as arcs, provided
//! the face carries curve-true geometry; polygon-only parts fall back to
//! the chord path.
//!
//! The boundary and islands are read from the specified face; the step
//! targets that same face. Like the clearing planner's finishing passes,
//! no entry step is planned: the pass descends at its own pace and the
//! embedder maps feeds.

use std::sync::Arc;

use prof_macros::prof;

use crate::cnc::plan::plan::{Plan, PlanStep};
use crate::error::{RaygeoError, RaygeoResult};
use crate::ops::assembly::profile::{ProfileKind, ProfileSpec};
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
    /// Boundary-following tolerance; also the flattening tolerance of the
    /// exact path's line chains.
    pub tolerance: f64,
    /// Cut feed (mm/min); `None` leaves the plan's cut state unset.
    pub feed_rate: Option<f64>,
    /// Vertical plunge feed (mm/min) for descents into material; `None`
    /// falls back to the cut feed.
    pub plunge_feed: Option<f64>,
}

/// Builds a one-step profile plan for one face of `part`.
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
