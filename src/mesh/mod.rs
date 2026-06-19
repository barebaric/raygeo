//! Mesh construction, refinement, PDE solving, and spiral tracing.

pub mod build;
pub mod gradient;
pub mod laplace;
pub mod pde;
pub mod remesh;
pub mod types;

pub use build::{build_triangle_mesh, build_uniform_mesh};
pub use gradient::compute_gradient_field;
pub use laplace::{solve_laplace, solve_laplace_with_history};
pub use pde::trace_spiral;
pub use remesh::remesh;
pub use types::{BoundaryTag, Triangle, TriangleMesh};
