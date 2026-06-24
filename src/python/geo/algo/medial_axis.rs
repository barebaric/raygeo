pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.medial_axis",
    "{}",
    MODULE_DOC_MEDIAL_AXIS
);

pub(crate) const MODULE_DOC_MEDIAL_AXIS: &str = "\
Medial Axis Transform (MAT) computation.

The MAT is the skeleton of a 2D domain — the set of points equidistant
to two or more boundary features.  It is computed via Delaunay-circumcenter
extraction from a constrained triangulation of the domain boundary.

* ``MedialAxis.compute`` — compute the MAT of a domain (with optional holes).
* ``MedialAxis.path_between`` — find a path between two points along the skeleton.
* ``MedialAxis.trim_to_polygons`` — filter nodes to those inside given polygons.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::algo::medial_axis as ma;
use crate::types::Point;

/// Medial Axis Transform of a planar domain.
///
/// The MAT is the set of points equidistant to two or more boundary
/// features, forming the skeleton of the free space.
///
/// **Usage**:
///
/// .. code-block:: python
///
///     axis = MedialAxis.compute(outer, holes)
///     path = axis.path_between((x1, y1), (x2, y2))
///     trimmed = axis.trim_to_polygons(polygons)
///     nodes = axis.nodes
///     clearances = axis.clearances
///     edges = axis.edges
///     root = axis.root
///     branches = axis.branches
#[gen_stub_pyclass(module = "raygeo.geo.algo.medial_axis")]
#[pyclass(skip_from_py_object, name = "MedialAxis")]
#[derive(Debug, Clone)]
pub struct PyMedialAxis {
    pub(crate) inner: ma::MedialAxis,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyMedialAxis {
    /// Compute the Medial Axis Transform of a planar domain.
    ///
    /// :param outer: Outer boundary polygon (list of ``(x, y)`` vertices).
    /// :param holes: List of hole polygons (each a list of ``(x, y)`` vertices).
    /// :param min_clearance: Minimum clearance distance in mm.
    /// :param sampling_spacing: Spacing between sampling points in mm.
    /// :returns: ``MedialAxis`` object.
    #[staticmethod]
    #[pyo3(signature = (
        outer,
        holes = None,
        min_clearance = 1.0,
        sampling_spacing = 1.0,
    ))]
    fn compute(
        outer: Vec<(f64, f64)>,
        holes: Option<Vec<Vec<(f64, f64)>>>,
        min_clearance: f64,
        sampling_spacing: f64,
    ) -> PyResult<PyMedialAxis> {
        let outer_pts: Vec<Point> =
            outer.into_iter().map(|(x, y)| Point::new(x, y)).collect();
        let holes_pts: Vec<Vec<Point>> = holes
            .unwrap_or_default()
            .into_iter()
            .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();

        let inner = ma::MedialAxis::compute(
            &outer_pts,
            &holes_pts,
            min_clearance,
            sampling_spacing,
        )
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

        Ok(PyMedialAxis { inner })
    }

    /// Find a path between two points along the medial axis skeleton.
    ///
    /// :param from_pt: Start point ``(x, y)``.
    /// :param to_pt: End point ``(x, y)``.
    /// :returns: List of ``(x, y)`` waypoints along the medial axis.
    #[pyo3(signature = (from_pt, to_pt))]
    fn path_between(
        &self,
        from_pt: (f64, f64),
        to_pt: (f64, f64),
    ) -> Option<Vec<(f64, f64)>> {
        let path = self.inner.path_between(
            Point::new(from_pt.0, from_pt.1),
            Point::new(to_pt.0, to_pt.1),
        );
        path.map(|pts| pts.into_iter().map(|p| (p.x, p.y)).collect())
    }

    /// Return a new ``MedialAxis`` containing only nodes whose
    /// positions fall inside at least one of the given polygons.
    ///
    /// :param polygons: List of polygons to trim against.
    /// :returns: Trimmed ``MedialAxis``.
    #[pyo3(signature = (polygons,))]
    fn trim_to_polygons(&self, polygons: Vec<Vec<(f64, f64)>>) -> PyMedialAxis {
        let pts: Vec<Vec<Point>> = polygons
            .into_iter()
            .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        let inner = self.inner.trim_to_polygons(&pts);
        PyMedialAxis { inner }
    }

    // ── Properties ────────────────────────────────────────────────

    #[getter]
    fn nodes(&self) -> Vec<(f64, f64)> {
        self.inner
            .nodes
            .iter()
            .map(|n| (n.point.x, n.point.y))
            .collect()
    }

    #[getter]
    fn clearances(&self) -> Vec<f64> {
        self.inner.nodes.iter().map(|n| n.clearance).collect()
    }

    #[getter]
    fn edges(&self) -> Vec<(usize, usize)> {
        self.inner.edges.clone()
    }

    #[getter]
    fn root(&self) -> usize {
        self.inner.root
    }

    #[getter]
    fn branches(&self) -> Vec<Vec<usize>> {
        self.inner
            .branches
            .iter()
            .map(|b| b.nodes.clone())
            .collect()
    }
}

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "medial_axis")?;
    m.setattr("__doc__", MODULE_DOC_MEDIAL_AXIS)?;

    m.add_class::<PyMedialAxis>()?;

    algo_mod.add_submodule(&m)?;
    Ok(())
}
