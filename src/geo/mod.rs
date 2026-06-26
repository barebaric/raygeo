//! Geo: Geometry, shapes, and algorithms.
//!
//! This module provides the core geometric types and operations including
//! the `Geometry` struct, shape primitives, and algorithms.

pub mod algo;
pub mod geometry;
pub use geometry::Geometry;
pub mod math;
pub mod query;
pub mod shape;
