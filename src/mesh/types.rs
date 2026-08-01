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
