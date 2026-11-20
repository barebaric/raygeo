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

pub mod arrays;
pub mod boolean;
pub mod components;
pub mod constraint;
pub mod constraints;
pub mod contour;
pub mod document;
pub mod draw;
pub mod entities;
pub mod entity_group;
pub mod eval;
pub mod geometry;
pub mod params;
pub mod registry;
pub mod selection;
pub mod solver;
pub mod topology;
pub mod transform;
pub mod trf;
