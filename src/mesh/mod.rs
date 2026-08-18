//! Mesh construction, refinement, PDE solving, and spiral tracing.

pub mod build;
pub mod gradient;
pub mod laplace;
pub mod pde;
pub mod remesh;
pub mod solid;
pub mod types;

pub use types::{BoundaryTag, Triangle, TriangleMesh};
