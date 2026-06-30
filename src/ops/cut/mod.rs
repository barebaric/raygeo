pub mod cleared_area;
pub(crate) mod crescent;
pub mod interp;
pub mod search;
pub mod stepper;
mod types;

pub use cleared_area::ClearedArea;
pub use crescent::cut_area;
pub use search::search_frontier_engagement;
pub use stepper::{
    run_segment, step, step_adaptive, target_engagement_from_advance,
    StepResult, StepStatus, StepperOptions,
};
pub use types::ToolPose;

/// Milling rotational direction. All cutting moves respect this
/// for the whole run.  Resume strategies use it to determine the
/// frontier walk direction, and `probe_step` uses it to vet resume
/// positions with one-sided deflection bounds.  The main stepper
/// loop uses symmetric bounds and relies on heading momentum +
/// cleared-area geometry to maintain the rotational bias.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CutDirection {
    /// Clockwise.
    Cw,
    /// Counter-clockwise.
    #[default]
    Ccw,
}

impl CutDirection {
    /// One-sided angle bounds for `step_adaptive`, relative to the
    /// heading.  `max_deflection` is the magnitude of the allowed
    /// turn.  Returns `(angle_min, angle_max)` in radians.
    ///
    /// When walking CCW around the cleared area, uncut material is on
    /// the RIGHT of the heading, so the tool must deflect right
    /// (negative angle).  CW cutting is the mirror.
    ///
    /// Used by `probe_step` to vet resume positions — a candidate
    /// that only finds engagement on the wrong (cleared) side is
    /// rejected.
    pub fn angle_bounds(&self, max_deflection: f64) -> (f64, f64) {
        match self {
            CutDirection::Cw => (0.0, max_deflection),
            CutDirection::Ccw => (-max_deflection, 0.0),
        }
    }
}
