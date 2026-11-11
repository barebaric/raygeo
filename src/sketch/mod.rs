//! Sketch constraint solving.
//!
//! Solves 2D geometric constraint systems for parametric sketching:
//! a set of points is moved so that geometric relationships (horizontal,
//! distance, tangency, …) hold, using the trust-region-reflective
//! least-squares algorithm with analytically supplied (or finite
//! difference) Jacobians.
//!
//! The layer depends only on [`crate::geo`] primitives; the caller
//! describes the problem as a flat list of points and typed constraint
//! descriptors, and receives solved positions plus per-point
//! constrained (fully-determined) flags and per-constraint conflict
//! indices.

pub mod components;
pub mod constraint;
pub mod eval;
pub mod solver;
pub mod trf;
