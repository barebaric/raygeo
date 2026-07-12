use crate::types::Polygon;

/// Boundary and islands of a workpiece — the geometric input to
/// clearing operations. Extracted once from geometry and cached on
/// [`Part`](super::Part).
#[derive(Clone, Debug, Default)]
pub struct StockRegion {
    pub boundary: Polygon,
    pub islands: Vec<Polygon>,
}

impl StockRegion {
    pub fn new(boundary: Polygon, islands: Vec<Polygon>) -> Self {
        Self { boundary, islands }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.boundary.len() < 3
    }
}
