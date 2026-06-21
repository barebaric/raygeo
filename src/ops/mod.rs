//! Ops: Machine operations and command sequences.
//!
//! This module provides types for representing machine operations including
//! command types, categories, axis flags, and machine state.

pub mod assembly;
pub mod axis;
pub(crate) mod clip;
pub mod container;
pub mod enums;
pub mod flip;
pub mod group;
pub(crate) mod layer;
pub mod linearize;
pub mod merge_lines;
pub mod optimize;
pub mod state;
pub mod transform;
pub mod types;

pub use assembly::hsm::{adaptive_peeling, link_arcs_to_ops};
pub use assembly::lead_in_out::apply_lead_in_out;
pub use assembly::overscan::apply_overscan;
pub use assembly::polyline::{link_passes, polyline_to_ops, LinkStrategy};
pub use assembly::tabs::{apply_tab_gaps, apply_tab_power, ClipPoint};
pub use axis::Axis;
pub use container::Ops;
pub use enums::{CommandCategory, CommandType, SectionType};
pub use group::{
    group_by_state_continuity, iter_section_ranges, iter_sections,
    segment_indices, split_into_subpaths, without_state, OpsSection,
    OpsSectionRange,
};
pub use merge_lines::merge_overlapping_lines;
pub use state::{CoolantMode, State};
pub use types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};
