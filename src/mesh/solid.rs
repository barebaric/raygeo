//! Plain-data closed-manifold triangle meshes for solid interchange.
//!
//! [`SolidMesh`] is the interchange format for solid geometry that
//! crosses module and language boundaries: material effects carried
//! in [`AssemblyOutput`](crate::ops::assembly::AssemblyOutput), the
//! remaining-stock solid of a material fold, and (eventually) CSG
//! results. It is deliberately minimal — f64 positions plus triangle
//! indices and nothing else — so that presentation formats
//! (float32 GPU buffers with normals/UVs, FEM meshes with adjacency)
//! stay out of the cached domain data.

use crate::geo::types::Point3D;

/// A closed-manifold triangle mesh in millimetres.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SolidMesh {
    /// Vertex positions (world mm).
    pub positions: Vec<Point3D>,
    /// Triangles as indices into `positions`.
    pub triangles: Vec<[u32; 3]>,
}

impl SolidMesh {
    /// Build a solid from positions and triangles.
    pub fn new(positions: Vec<Point3D>, triangles: Vec<[u32; 3]>) -> Self {
        Self {
            positions,
            triangles,
        }
    }

    /// True when the solid has no triangles.
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }
}
