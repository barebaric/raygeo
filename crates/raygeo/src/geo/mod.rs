//! Geo: Geometry, shapes, and algorithms.
//!
//! This module provides the core geometric types and operations including
//! the `Geometry` struct, shape primitives, and algorithms.

pub mod algo;
pub mod analysis;
pub mod geometry;
pub mod math;
pub mod query;
pub mod shape;

pub use algo::*;
pub use analysis::*;
pub use geometry::Geometry;
pub use math::*;
pub use query::*;
pub use shape::*;
