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
pub mod lead_in_out;
pub mod linearize;
pub mod merge_lines;
pub mod optimize;
pub mod overscan;
pub mod polyline;
pub mod raster;
pub mod state;
pub mod tabs;
pub mod transform;
pub mod types;

pub use axis::Axis;
pub use container::Ops;
pub use enums::{CommandCategory, CommandType, SectionType};
pub use group::{
    group_by_state_continuity, iter_section_ranges, iter_sections,
    segment_indices, split_into_subpaths, without_state, OpsSection,
    OpsSectionRange,
};
pub use lead_in_out::apply_lead_in_out;
pub use merge_lines::merge_overlapping_lines;
pub use overscan::apply_overscan;
pub use polyline::{link_passes, polyline_to_ops, LinkStrategy};
pub use state::State;
pub use tabs::{apply_tab_gaps, apply_tab_power, ClipPoint};
pub use types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};
