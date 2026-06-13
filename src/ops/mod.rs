//! Ops: Machine operations and command sequences.
//!
//! This module provides types for representing machine operations including
//! command types, categories, axis flags, and machine state.

pub mod axis;
pub(crate) mod clip;
pub mod container;
pub mod enums;
pub mod flip;
pub mod group;
pub(crate) mod layer;
#[cfg(feature = "python")]
pub(crate) mod lead_in_out;
pub mod linearize;
#[cfg(feature = "python")]
pub(crate) mod merge_lines;
pub mod optimize;
#[cfg(feature = "python")]
pub(crate) mod overscan;
pub mod raster;
pub mod state;
#[cfg(feature = "python")]
pub(crate) mod tabs;
#[cfg(feature = "python")]
pub(crate) mod transform;
pub mod types;

pub use axis::Axis;
pub use container::Ops;
pub use enums::{CommandCategory, CommandType, SectionType};
pub use group::{
    group_by_state_continuity, iter_section_ranges, iter_sections,
    segment_indices, split_into_subpaths, without_state, OpsSection,
    OpsSectionRange,
};
pub use state::State;
pub use types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};
