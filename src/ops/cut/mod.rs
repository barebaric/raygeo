pub mod cleared_area;
pub(crate) mod crescent;
pub mod interp;
pub mod search;
pub mod stepper;
mod types;

pub use cleared_area::ClearedArea;
pub use crescent::cut_area;
pub use search::{search_frontier_engagement, search_reengagement};
pub use stepper::{
    run_segment, step, step_adaptive, target_engagement_from_advance,
    StepResult, StepStatus, StepperOptions,
};
pub use types::ToolPose;
