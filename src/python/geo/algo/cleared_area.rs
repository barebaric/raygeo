pyo3_stub_gen::module_doc!("raygeo.geo.algo.cleared_area", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Incremental cleared-area tracker for adaptive clearing.

Maintains a union of tool-swept polygons and provides a spatial-indexed
windowed query for efficient engagement computation.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::algo::cleared_area::ClearedArea as RustClearedArea;
use crate::types::Rect;

#[gen_stub_pyclass(module = "raygeo.geo.algo.cleared_area")]
#[pyclass]
pub struct ClearedArea {
    pub(crate) inner: RustClearedArea,
}

#[gen_stub_pymethods]
#[pymethods]
impl ClearedArea {
    #[new]
    #[pyo3(signature = (initial = None))]
    pub fn new(initial: Option<Vec<Vec<(f64, f64)>>>) -> Self {
        match initial {
            Some(polys) => {
                let polygons: Vec<crate::types::Polygon> = polys
                    .into_iter()
                    .map(|v| {
                        v.into_iter()
                            .map(|(x, y)| crate::types::Point::new(x, y))
                            .collect()
                    })
                    .collect();
                ClearedArea {
                    inner: RustClearedArea::from_polygons(&polygons),
                }
            }
            None => ClearedArea {
                inner: RustClearedArea::new(),
            },
        }
    }

    /// :complexity: O(n) where n = number of path points
    pub fn expand(&mut self, tool_path: Vec<(f64, f64)>, tool_radius: f64) {
        let path: Vec<crate::types::Point> = tool_path
            .into_iter()
            .map(|(x, y)| crate::types::Point::new(x, y))
            .collect();
        self.inner.expand(&path, tool_radius);
    }

    /// :complexity: O(n) where n = total vertices across all polygons
    pub fn add_cleared_polygons(&mut self, polygons: Vec<Vec<(f64, f64)>>) {
        let polys: Vec<crate::types::Polygon> = polygons
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        self.inner.add_cleared_polygons(&polys);
    }

    /// :complexity: O(m + k) where m = number of fragments, k = output vertices
    pub fn query_window(
        &self,
        bbox: (f64, f64, f64, f64),
    ) -> Vec<Vec<(f64, f64)>> {
        let rect = Rect(bbox.0, bbox.1, bbox.2, bbox.3);
        let frags = self.inner.query_window(rect);
        frags
            .into_iter()
            .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// :complexity: O(n * m) where n = bounds vertices, m = fragments
    pub fn remaining(
        &self,
        bounds: Vec<Vec<(f64, f64)>>,
    ) -> Vec<Vec<(f64, f64)>> {
        let bounds_polys: Vec<crate::types::Polygon> = bounds
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(|(x, y)| crate::types::Point::new(x, y))
                    .collect()
            })
            .collect();
        let remaining = self.inner.remaining(&bounds_polys);
        remaining
            .into_iter()
            .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
            .collect()
    }

    /// :complexity: O(1)
    pub fn total_area(&self) -> f64 {
        self.inner.total_area()
    }

    fn __repr__(&self) -> String {
        format!("ClearedArea({} fragments)", self.inner.len())
    }
}

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "cleared_area")?;
    m.setattr("__doc__", MODULE_DOC)?;

    m.add_class::<ClearedArea>()?;

    algo_mod.add_submodule(&m)?;
    Ok(())
}
