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
pub mod linearize;
pub mod types;
pub mod state;
pub(crate) mod transform;

pub use axis::Axis;
pub use container::*;
pub use enums::*;
pub use group::*;
pub use types::*;
pub use state::*;
