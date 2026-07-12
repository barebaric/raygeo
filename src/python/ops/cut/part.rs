use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::cut::part::Part;
use crate::python::geo::geometry::Geometry as PyGeometry;
use crate::python::ops::cut::cleared_area::PyClearedArea;
use crate::python::ops::cut::stock_region::PyStockRegion;
use crate::types::{Point, Polygon};

/// Unified workpiece description for motion assembly.
///
/// Carries geometry, physical metadata, and a ``ClearedArea``
/// tracking what has already been cut.  Assemblers mutate the
/// cleared area as they work.
#[gen_stub_pyclass(module = "raygeo.ops.cut")]
#[pyclass(name = "Part", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyPart {
    pub inner: Part,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPart {
    /// Create a new Part.
    ///
    /// :param geometry: Optional vector geometry (outlines of the part).
    /// :param size_mm: Physical size ``(width, height)`` in millimetres.
    /// :param pixels_per_mm: Optional pixel density ``(x, y)`` in px/mm.
    #[new]
    #[pyo3(signature = (geometry=None, size_mm=(0.0, 0.0), pixels_per_mm=None))]
    fn __new__(
        py: Python,
        geometry: Option<Py<PyGeometry>>,
        size_mm: (f64, f64),
        pixels_per_mm: Option<(f64, f64)>,
    ) -> Self {
        let inner_geo = geometry.map(|py_geo| py_geo.borrow(py).inner.clone());
        let mut inner = Part::new(inner_geo, size_mm);
        inner.pixels_per_mm = pixels_per_mm;
        PyPart { inner }
    }

    /// Physical size ``(width, height)`` in millimetres.
    #[getter]
    fn size_mm(&self) -> (f64, f64) {
        self.inner.size_mm
    }

    /// Pixel density ``(x, y)`` in px/mm, if set.
    #[getter]
    fn pixels_per_mm(&self) -> Option<(f64, f64)> {
        self.inner.pixels_per_mm
    }

    /// Vector geometry (the outline(s) of the part), if any.
    ///
    /// Returns ``None`` if no geometry was provided at construction time.
    #[getter]
    fn geometry(&self) -> Option<PyGeometry> {
        self.inner.geometry.clone().map(|g| PyGeometry { inner: g })
    }

    /// Build a Part from a boundary polygon and optional islands.
    ///
    /// :param boundary: Outer boundary as ``[(x, y), ...]``.
    /// :param islands: List of island polygons, each ``[(x, y), ...]``
    ///     (default ``[]``).
    /// :param size_mm: Physical size ``(width, height)`` in mm
    ///     (default ``(0, 0)``).
    /// :param initial: Optional pre-seeded cleared polygons (e.g. a
    ///     seed circle for adaptive clearing).  When provided, the
    ///     part's cleared area starts with these fragments instead of
    ///     being empty.
    /// :returns: A new ``Part`` with the geometry constructed from the
    ///     given polygons.
    #[staticmethod]
    #[pyo3(signature = (boundary, islands=None, size_mm=(0.0, 0.0), initial=None))]
    fn from_polygons(
        boundary: Vec<(f64, f64)>,
        islands: Option<Vec<Vec<(f64, f64)>>>,
        size_mm: (f64, f64),
        initial: Option<Vec<Vec<(f64, f64)>>>,
    ) -> Self {
        let bnd: Polygon = boundary
            .into_iter()
            .map(|(x, y)| Point::new(x, y))
            .collect();
        let isls: Vec<Polygon> = islands
            .unwrap_or_default()
            .into_iter()
            .map(|isl| isl.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        let init: Vec<Polygon> = initial
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        PyPart {
            inner: if init.is_empty() {
                Part::from_polygons(&bnd, &isls, size_mm)
            } else {
                Part::from_polygons_initial(&bnd, &isls, &init, size_mm)
            },
        }
    }

    /// Accumulated cleared-area state — what has been cut so far.
    ///
    /// Read-only snapshot.  Assemblers mutate this internally;
    /// use it after an assembler returns to inspect remaining
    /// material, fragments, etc.
    #[getter]
    fn cleared(&self) -> PyClearedArea {
        PyClearedArea {
            inner: self.inner.cleared.clone(),
        }
    }

    /// Boundary and islands of the workpiece — cached extraction
    /// from geometry. Read-only.
    #[getter]
    fn stock_region(&self) -> PyStockRegion {
        PyStockRegion {
            inner: self.inner.stock_region.clone(),
        }
    }

    /// True if this Part has geometry.
    fn has_geometry(&self) -> bool {
        self.inner.geometry.is_some()
    }

    fn __repr__(&self) -> String {
        format!(
            "Part(size_mm=({:.1}, {:.1}), geometry={})",
            self.inner.size_mm.0,
            self.inner.size_mm.1,
            if self.inner.geometry.is_some() {
                "Some"
            } else {
                "None"
            },
        )
    }
}

pub fn register(cut_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    cut_mod.add_class::<PyPart>()?;
    Ok(())
}
