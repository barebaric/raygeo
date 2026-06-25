//! Ops: Machine operations and command sequences.
//!
//! This module provides types for representing machine operations including
//! command types, categories, axis flags, and machine state.

pub mod assembly;
pub mod axis;
pub mod cleared_area;
pub mod container;
pub mod enums;
pub mod state;
pub mod transform;
pub mod types;

pub use assembly::polyline::polyline_to_ops;
pub use axis::Axis;
pub use container::Ops;
pub use enums::{CommandCategory, CommandType, SectionType};
pub use state::{CoolantMode, State};
pub use transform::{
    apply_lead_in_out, apply_overscan, apply_tab_gaps, apply_tab_power,
    flip_ops, group_by_state_continuity, iter_section_ranges, iter_sections,
    link_passes, merge_overlapping_lines, optimize_travel, segment_indices,
    split_into_subpaths, without_state, ClipPoint, LinkStrategy, OpsSection,
    OpsSectionRange,
};
pub use types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};
