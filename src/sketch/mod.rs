//! Sketch constraint solving, entity model and topology.
//!
//! Mirrors the reference sketcher core structure:
//! - [`entities`] — the entity classes (point + seven entity kinds)
//!   with their polymorphic geometry methods
//! - [`registry`] — the document registry (points + entities)
//! - [`constraints`] — the constraint classes' solver math
//! - [`solver`] (+ [`trf`], [`eval`], [`components`]) — the
//!   least-squares solver and its support machinery
//! - [`topology`] — chain assembly and closed-loop detection
//!
//! The layer depends only on [`crate::geo`] primitives; the caller
//! describes problems as flat descriptors and receives solved
//! positions, per-point constrained flags, conflict indices, chains
//! and faces.

pub mod components;
pub mod constraint;
pub mod constraints;
pub mod entities;
pub mod eval;
pub mod registry;
pub mod solver;
pub mod topology;
pub mod trf;
