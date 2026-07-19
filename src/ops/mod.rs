//! Ops: Machine operations and command sequences.
//!
//! This module provides types for representing machine operations including
//! command types, categories, axis flags, and machine state.

pub mod assembly;
pub mod axis;
pub mod callbacks;
pub mod container;
pub mod convert;
pub mod cut;
pub mod enums;
pub mod feature;
pub mod part;
pub mod state;
pub mod transform;
pub mod types;

pub use assembly::result::AssemblyMeta;
pub use axis::Axis;
pub use container::structure::{OpsSection, OpsSectionRange};
pub use container::Ops;
pub use enums::{CommandCategory, CommandType, RasterMode, SectionType};
pub use state::{AirAssistMode, CoolantMode, HeadCoolantMode, State};
pub use transform::{
    apply_bidir_scan_offset, apply_lead_in_out, apply_multipass,
    apply_overscan, apply_tab_gaps, apply_tab_power, flip_ops,
    group_by_auxiliary_state, link_passes, merge_overlapping_lines,
    optimize_travel, without_state, ClipPoint, LinkStrategy,
};
pub use types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};
