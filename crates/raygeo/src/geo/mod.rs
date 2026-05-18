//! Geo: Geometry, shapes, and algorithms.
//!
//! This module provides the core geometric types and operations including
//! the `Geometry` struct, shape primitives, and algorithms.

pub mod algo;
pub mod analysis;
pub mod cleanup;
pub mod geometry;
pub mod intersect;
pub mod query;
pub mod shape;
pub mod split;
pub mod transform;

pub use algo::*;
pub use analysis::*;
pub use cleanup::*;
pub use geometry::Geometry;
pub use intersect::*;
pub use query::*;
pub use shape::*;
pub use split::*;
pub use transform::*;
