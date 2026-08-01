use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::types::Point;
use crate::ops::part::StockRegion;

/// Boundary and islands of a workpiece — the geometric input to
/// clearing operations.
#[gen_stub_pyclass(module = "raygeo.ops.part")]
#[pyclass(name = "StockRegion", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyStockRegion {
    pub(crate) inner: StockRegion,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStockRegion {
    /// Create a new StockRegion.
    ///
    /// :param boundary: Outer boundary polygon as ``[(x, y), ...]``.
    /// :param islands: List of island polygons, each ``[(x, y), ...]``
    ///     (default ``[]``).
    #[new]
    #[pyo3(signature = (boundary, islands=None))]
    fn __new__(
        boundary: Vec<(f64, f64)>,
        islands: Option<Vec<Vec<(f64, f64)>>>,
    ) -> Self {
        let b: crate::geo::types::Polygon = boundary
            .into_iter()
            .map(|(x, y)| Point::new(x, y))
            .collect();
        let i: Vec<crate::geo::types::Polygon> = islands
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        PyStockRegion {
            inner: StockRegion::new(b, i),
        }
    }

    /// Outer boundary polygon as ``[(x, y), ...]``.
    #[getter]
    fn boundary(&self) -> Vec<(f64, f64)> {
        self.inner.boundary.iter().map(|p| (p.x, p.y)).collect()
    }

    /// List of island polygons, each ``[(x, y), ...]``.
    #[getter]
    fn islands(&self) -> Vec<Vec<(f64, f64)>> {
        self.inner
            .islands
            .iter()
            .map(|isl| isl.iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "StockRegion(boundary_len={}, islands={})",
            self.inner.boundary.len(),
            self.inner.islands.len()
        )
    }
}

impl PyStockRegion {
    pub fn from_parts(
        boundary: Vec<(f64, f64)>,
        islands: Vec<Vec<(f64, f64)>>,
    ) -> Self {
        let b: crate::geo::types::Polygon = boundary
            .into_iter()
            .map(|(x, y)| Point::new(x, y))
            .collect();
        let i: Vec<crate::geo::types::Polygon> = islands
            .into_iter()
            .map(|v| v.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        PyStockRegion {
            inner: StockRegion::new(b, i),
        }
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyStockRegion>()?;
    Ok(())
}
