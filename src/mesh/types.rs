pub type Triangle = [usize; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryTag {
    Outer,
    Inner,
    None,
}

#[derive(Clone, Debug)]
pub struct TriangleMesh {
    pub vertices: Vec<crate::geo::types::Point>,
    pub triangles: Vec<Triangle>,
    pub adjacency: Vec<isize>,
    pub boundary_tags: Vec<BoundaryTag>,
}

/// GPU-ready triangle mesh of an extruded polygon (closed prism).
///
/// Vertices are laid out per face (not shared between faces) so each
/// face carries its own flat normal and UV.
#[derive(Clone, Debug, Default)]
pub struct PrismMesh {
    /// Flat XYZ positions, row-major, length `3N`.
    pub positions: Vec<f32>,
    /// Flat XYZ unit normals, length `3N`.
    pub normals: Vec<f32>,
    /// Flat XY UV coordinates, length `2N`.
    pub uvs: Vec<f32>,
    /// Triangle vertex indices, length `3T`.
    pub indices: Vec<u32>,
}
