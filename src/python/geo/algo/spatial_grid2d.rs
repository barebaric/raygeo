use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::algo::spatial_grid2d;
use crate::types::Rect;

pyo3_stub_gen::module_doc!("raygeo.geo.algo.spatial_grid2d", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Grid-based spatial index for fast overlap queries.

Divides the 2D plane into fixed-size cells and associates each
inserted item with the cells its bounding box touches.
";

// ---------------------------------------------------------------------------
// SpatialGrid class
// ---------------------------------------------------------------------------

/// A grid-based spatial index for fast overlap queries.
///
/// Divides the 2D plane into fixed-size cells and indexes items by
/// their bounding box for efficient overlap lookups.
#[gen_stub_pyclass(module = "raygeo.geo.algo.spatial_grid2d")]
#[pyclass]
pub struct SpatialGrid {
    pub(crate) inner: spatial_grid2d::SpatialGrid,
}

#[gen_stub_pymethods]
#[pymethods]
impl SpatialGrid {
    /// Create a new spatial grid with the given cell size.
    ///
    /// :param cell_size: Side length of each grid cell in mm.
    #[new]
    pub fn new(cell_size: f64) -> Self {
        SpatialGrid {
            inner: spatial_grid2d::SpatialGrid::new(cell_size),
        }
    }

    /// Insert an item into the grid by its bounding box.
    ///
    /// :param index: Unique identifier for the item.
    /// :param bbox: ``[x_min, y_min, x_max, y_max]`` bounding box.
    /// :complexity: O(1) amortised
    pub fn insert(&mut self, index: usize, bbox: Vec<f64>) {
        let b = Rect(bbox[0], bbox[1], bbox[2], bbox[3]);
        self.inner.insert(index, b);
    }

    /// Query all items whose bounding box overlaps *bbox*.
    ///
    /// :param bbox: ``[x_min, y_min, x_max, y_max]`` query region.
    /// :returns: Sorted list of matching item indices.
    /// :complexity: O(cells + k) where k = number of matching items
    pub fn query(&self, bbox: Vec<f64>) -> Vec<usize> {
        let b = Rect(bbox[0], bbox[1], bbox[2], bbox[3]);
        let result = self.inner.query(b);
        let mut vec: Vec<usize> = result.into_iter().collect();
        vec.sort();
        vec
    }

    /// Remove all items from the grid.
    ///
    /// :complexity: O(n) where n = number of items
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    fn __repr__(&self) -> String {
        format!("SpatialGrid(cell_size={})", self.inner.cell_size())
    }
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "spatial_grid2d")?;
    m.setattr("__doc__", MODULE_DOC)?;
    m.add_class::<SpatialGrid>()?;
    algo_mod.add_submodule(&m)?;
    Ok(())
}
