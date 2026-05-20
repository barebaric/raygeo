//! Algo: Complex mathematical operations.
//!
//! This module provides advanced geometric algorithms including clipping,
//! curve fitting, Minkowski sums, simplification, and smoothing.

pub mod cleanup;
pub mod clipping;
pub mod fitting;
pub mod interp;
pub mod intersect;
pub mod minkowski;
pub mod offset;
pub mod simplify;
pub mod smooth;
pub mod topology;

pub use cleanup::*;
pub use clipping::*;
pub use fitting::*;
pub use interp::*;
pub use intersect::*;
pub use minkowski::*;
pub use offset::*;
pub use simplify::*;
pub use smooth::*;
pub use topology::*;
