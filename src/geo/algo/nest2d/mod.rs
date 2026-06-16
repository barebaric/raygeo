//! Nesting: 2D packing algorithms for sheet/plate layout.
//!
//! All algorithms in this module are **strictly planar (XY-plane only).**
//! Nesting is a 2D optimization problem — Z coordinates are not modeled.
//! 3D callers must project to the XY plane before calling these functions.

pub mod collision;
pub mod genetic;
pub mod gravity;
pub mod ifp;
pub mod nfp;
pub mod placement;
