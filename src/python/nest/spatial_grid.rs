use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::nest::spatial_grid;

pyo3_stub_gen::module_doc!("raygeo.nest.spatial_grid", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Grid-based spatial index for fast overlap queries.

Divides the 2D plane into fixed-size cells and associates each
inserted item with the cells its bounding box touches.
";

// ---------------------------------------------------------------------------
// SpatialGrid class
// ---------------------------------------------------------------------------

#[gen_stub_pyclass(module = "raygeo.nest.spatial_grid")]
#[pyclass]
pub struct SpatialGrid {
    pub(crate) inner: spatial_grid::SpatialGrid,
}

#[gen_stub_pymethods]
#[pymethods]
impl SpatialGrid {
    #[new]
    pub fn new(cell_size: f64) -> Self {
        SpatialGrid {
            inner: spatial_grid::SpatialGrid::new(cell_size),
        }
    }

    pub fn insert(&mut self, index: usize, bbox: Vec<f64>) {
        let b = (bbox[0], bbox[1], bbox[2], bbox[3]);
        self.inner.insert(index, b);
    }

    pub fn query(&self, bbox: Vec<f64>) -> Vec<usize> {
        let b = (bbox[0], bbox[1], bbox[2], bbox[3]);
        let result = self.inner.query(b);
        let mut vec: Vec<usize> = result.into_iter().collect();
        vec.sort();
        vec
    }

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

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SpatialGrid>()?;
    Ok(())
}
