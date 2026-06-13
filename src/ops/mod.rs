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
#[cfg_attr(not(feature = "python"), allow(dead_code))]
pub(crate) mod lead_in_out;
pub mod linearize;
#[cfg_attr(not(feature = "python"), allow(dead_code))]
pub(crate) mod merge_lines;
pub mod optimize;
#[cfg_attr(not(feature = "python"), allow(dead_code))]
pub(crate) mod overscan;
pub mod raster;
pub mod state;
#[cfg_attr(not(feature = "python"), allow(dead_code))]
pub(crate) mod tabs;
#[cfg_attr(not(feature = "python"), allow(dead_code))]
pub(crate) mod transform;
pub mod types;

pub use axis::Axis;
pub use container::Ops;
pub use enums::{CommandCategory, CommandType, SectionType};
pub use group::{
    group_by_state_continuity, iter_section_ranges, iter_sections,
    segment_indices, segments, split_into_subpaths, without_state, OpsSection,
    OpsSectionRange,
};
pub use state::State;
pub use types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};
