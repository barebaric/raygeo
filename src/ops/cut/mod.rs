pub mod cleared_area;
pub(crate) mod crescent;
pub mod interp;
pub mod part;
pub mod search;
pub mod stepper;
mod types;

pub use cleared_area::ClearedArea;
pub use crescent::cut_area;
pub use part::Part;
pub use search::search_frontier_engagement;
pub use stepper::{step, StepResult, StepStatus, StepperOptions};
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
    /// One-sided angle bounds for [`step`], relative to the
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

    /// Sign of the steering angle the tool should prefer to honour
    /// this rotational direction.
    ///
    /// * `Cw`  → `+1.0` (deflect left / positive angle)
    /// * `Ccw` → `−1.0` (deflect right / negative angle)
    ///
    /// Used by the adaptive stepper as a soft directional bias when
    /// material is present on both sides of the tool (a "breakthrough"
    /// between two cleared regions).  `0.0` would mean "no bias"; pass
    /// [`CutDirection::sign`] to opt into the bias.
    pub fn sign(&self) -> f64 {
        match self {
            CutDirection::Cw => 1.0,
            CutDirection::Ccw => -1.0,
        }
    }
}
