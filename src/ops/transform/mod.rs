//! Transform: operations that take existing Ops and return modified Ops.
//!
//! Modules in this layer consume [`Ops`](crate::ops::Ops) sequences and
//! produce new or mutated sequences — travel optimization, flipping,
//! pass linking, lead-in/out, overscan, tabs, linearization, merging,
//! grouping, and clipping.

pub mod affine;
pub mod bidir_scan_offset;
pub mod clip;
pub mod flip;
pub mod frame;
pub mod group;
pub mod layer;
pub mod lead_in_out;
pub mod linearize;
pub mod link;
pub mod merge_lines;
pub mod optimize;
pub mod overscan;
pub mod smooth;
pub mod split;
pub mod tabs;

pub use bidir_scan_offset::apply_bidir_scan_offset;
pub use flip::flip_ops;
pub use group::{group_by_auxiliary_state, without_state};
pub use lead_in_out::apply_lead_in_out;
pub use link::{link_passes, LinkStrategy};
pub use merge_lines::merge_overlapping_lines;
pub use optimize::optimize_travel;
pub use overscan::apply_overscan;
pub use tabs::{apply_tab_gaps, apply_tab_power, ClipPoint};
