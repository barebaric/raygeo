//! Adaptive profiling of pocket boundaries.
//!
//! Walks a tool around the **inner** or **outer** profile of a closed
//! boundary.  Inner profiling follows the inset boundary (offset inward
//! by tool radius), material-aware around islands.  Outer profiling
//! follows the grown boundary (offset outward) and ignores islands.

mod engine;
mod inner;
mod options;
mod outer;
mod tracelet;

pub use inner::profile_inner;
pub use options::{ProfileInnerOptions, ProfileOuterOptions};
pub use outer::profile_outer;
