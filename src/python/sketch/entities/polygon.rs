//! Bindings for the polygon entity.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::geo::geometry::Geometry as PyGeometry;
use crate::python::sketch::constraints::dict_from_value;
use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::entities::mirror::{MirrorAxis, MirrorDirection};
use crate::sketch::entities::polygon::PolygonEntity as CorePolygon;
use crate::sketch::entities::Entity;

/// A polygon defined in the local frame of center and handle
/// points.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch.entities.polygon", name = "Polygon")]
pub struct PyPolygon {
    pub inner: CorePolygon,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPolygon {
    /// Creates an unattached polygon; an id is assigned on
    /// insertion.
    #[new]
    #[pyo3(signature = (center, handle, vertices, closed = false, construction = false, rings = None))]
    fn new(
        center: u64,
        handle: u64,
        vertices: Vec<(f64, f64)>,
        closed: bool,
        construction: bool,
        rings: Option<Vec<Vec<(f64, f64)>>>,
    ) -> Self {
        PyPolygon {
            inner: CorePolygon::new(
                0,
                center,
                handle,
                vertices,
                closed,
                construction,
                rings.unwrap_or_default(),
            ),
        }
    }

    /// Unique entity id.
    #[getter]
    fn id(&self) -> u64 {
        self.inner.base.id
    }

    /// Center point id.
    #[getter]
    fn center_idx(&self) -> u64 {
        self.inner.center
    }

    /// Handle point id.
    #[getter]
    fn handle_idx(&self) -> u64 {
        self.inner.handle
    }

    /// Whether the polygon contour is closed.
    #[getter]
    fn closed(&self) -> bool {
        self.inner.closed
    }

    /// The outer contour vertices in the local frame.
    #[getter]
    fn vertices(&self) -> Vec<(f64, f64)> {
        self.inner.rings.first().cloned().unwrap_or_default()
    }

    /// The hole rings in the local frame.
    #[getter]
    fn rings(&self) -> Vec<Vec<(f64, f64)>> {
        self.inner.rings[1..].to_vec()
    }

    /// Whether the entity is construction geometry.
    #[getter]
    fn construction(&self) -> bool {
        self.inner.base.construction
    }

    /// Whether the entity is excluded from toolpath output.
    #[getter]
    fn invisible(&self) -> bool {
        self.inner.base.invisible
    }

    /// Whether the entity was created as part of an array copy.
    #[getter]
    fn array_copy(&self) -> bool {
        self.inner.base.array_copy
    }

    /// Whether the entity is fully constrained, as computed by the
    /// solver.
    #[getter]
    fn constrained(&self) -> bool {
        self.inner.base.constrained
    }

    /// The entity type name.
    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }

    /// The point ids the entity is defined by.
    fn get_point_ids(&self) -> Vec<u64> {
        self.inner.get_point_ids()
    }

    /// Serializes the entity to the reference dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        dict_from_value(py, &self.inner.to_json())
    }

    /// Returns the geometry signature used for change detection.
    fn geometry_signature<'py>(
        &self,
        py: Python<'py>,
        registry: &PyEntityRegistry,
    ) -> PyResult<Bound<'py, PyDict>> {
        dict_from_value(py, &self.inner.geometry_signature(&registry.inner))
    }

    /// Returns the entity's toolpath geometry in model coordinates.
    fn to_geometry(&self, registry: &PyEntityRegistry) -> PyGeometry {
        let geometry = self.inner.to_geometry(&registry.inner);
        PyGeometry { inner: geometry }
    }

    /// Returns the entity's outline as polygon vertices.
    #[pyo3(signature = (registry, forward = true))]
    fn to_polygon_vertices(
        &self,
        registry: &PyEntityRegistry,
        forward: bool,
    ) -> Vec<(f64, f64)> {
        self.inner.to_polygon_vertices(&registry.inner, forward)
    }

    /// Returns the entity's centerline as polyline vertices.
    #[pyo3(signature = (registry, tolerance = 0.1))]
    fn to_polyline(
        &self,
        registry: &PyEntityRegistry,
        tolerance: f64,
    ) -> Vec<(f64, f64)> {
        self.inner.to_polyline(&registry.inner, tolerance)
    }

    /// Whether the entity is hit at model coordinates (mx, my).
    #[pyo3(signature = (mx, my, threshold, registry))]
    fn hit_test(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        registry: &PyEntityRegistry,
    ) -> bool {
        self.inner.hit_test(mx, my, threshold, &registry.inner)
    }

    /// Whether the model-space point lies inside the entity.
    fn contains_point(
        &self,
        registry: &PyEntityRegistry,
        x: f64,
        y: f64,
    ) -> bool {
        self.inner.contains_point(&registry.inner, x, y)
    }

    /// Signed distance from the model-space point, if defined.
    fn signed_distance_to(
        &self,
        px: f64,
        py: f64,
        registry: &PyEntityRegistry,
    ) -> Option<f64> {
        self.inner.signed_distance_to(px, py, &registry.inner)
    }

    /// Tangent direction at the point nearest the model-space point.
    fn tangent_at(
        &self,
        registry: &PyEntityRegistry,
        point_id: u64,
    ) -> Option<(f64, f64)> {
        self.inner.tangent_at(&registry.inner, point_id)
    }

    /// Mirrors the entity across the axis.
    #[pyo3(signature = (direction, position))]
    fn mirror(&mut self, direction: &str, position: f64) -> PyResult<()> {
        let axis_direction = match direction {
            "vertical" => MirrorDirection::Vertical,
            "horizontal" => MirrorDirection::Horizontal,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown mirror direction: {other}"
                )))
            }
        };
        self.inner.mirror(MirrorAxis {
            direction: axis_direction,
            position,
        });
        Ok(())
    }

    /// Sets the fill color; returns whether the entity supports
    /// fills.
    fn set_fill_color(&mut self, color: Option<[f64; 4]>) -> bool {
        self.inner.set_fill_color(color)
    }
}

/// Registers the polygon class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPolygon>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities.polygon", m)?;
    Ok(())
}
