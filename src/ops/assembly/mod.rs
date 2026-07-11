//! Motion-path assembly: turning raw geometry primitives into Ops.
//!
//! Functions in this module compose geo-layer primitives (polylines,
//! arcs, polygons) into complete motion sequences represented as
//! [`crate::ops::Ops`] objects. They decide traversal order, linking
//! strategy, lead-in/out, overscan, and tab insertion — concerns that
//! belong to motion assembly rather than pure geometry.

pub mod adaptive;
pub mod helix;
pub mod profile;
pub mod ramp;
pub mod result;
pub mod slot;
pub mod spiral;
pub mod toroid;
pub mod tracelet;
pub mod wavefront;

pub use tracelet::{write_polyline, ProgressEvent, Tracelet};
