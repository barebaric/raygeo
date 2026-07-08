//! Feature detection for machining analysis.
//!
//! This submodule provides higher-level feature detection that combines
//! raw geometry from the `geo` layer with machining knowledge to
//! identify features like narrow passages, plunge points, and ramps.

pub mod narrow;
pub mod near;
pub mod ramp;
pub mod region;
pub mod slot_path;
