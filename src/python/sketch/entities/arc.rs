//! Bindings for the arc entity.

use crate::python::sketch::registry::{value_from_dict, PyEntityRegistry};
use pyo3::exceptions::{PyNotImplementedError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::types::EntityId;

use super::PyEntityBase;
use crate::python::geo::geometry::Geometry as PyGeometry;
use crate::python::sketch::constraints::dict_from_value;
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::entities::signature_to_py;
use crate::sketch::entities::arc::Arc as CoreArc;
use crate::sketch::entities::Entity;

/// A circular arc from start to end around a center.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyEntityBase,
    subclass,module = "raygeo.sketch.entities.arc", name = "Arc")]
pub struct PyArc {
    pub inner: CoreArc,
    /// Live back-reference to the owning registry, if any.
    pub registry: Option<Py<PyEntityRegistry>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyArc {
    /// Creates an arc with the given id and defining point ids.
    #[new]
    #[pyo3(signature = (id, start_idx, end_idx, center_idx,
                        clockwise = false, construction = false))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: i64,
        start_idx: i64,
        end_idx: i64,
        center_idx: i64,
        clockwise: bool,
        construction: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyEntityBase).add_subclass(PyArc {
            inner: CoreArc::new(
                id as u64,
                start_idx as u64,
                end_idx as u64,
                center_idx as u64,
                clockwise,
                construction,
            ),
            registry: None,
        })
    }

    /// Unique entity id.
    #[getter]
    fn id(&self) -> EntityId {
        EntityId(self.inner.base.id)
    }

    #[setter]
    fn set_id(&mut self, value: EntityId) {
        self.inner.base.id = value.0;
    }

    /// Start point id.
    #[getter]
    fn start_idx(&self) -> i64 {
        self.inner.start as i64
    }

    #[setter]
    fn set_start_idx(&mut self, value: i64) {
        self.inner.start = value as u64;
        self.write_back();
    }

    /// End point id.
    #[getter]
    fn end_idx(&self) -> i64 {
        self.inner.end as i64
    }

    #[setter]
    fn set_end_idx(&mut self, value: i64) {
        self.inner.end = value as u64;
        self.write_back();
    }

    /// Center point id.
    #[getter]
    fn center_idx(&self) -> i64 {
        self.inner.center as i64
    }

    #[setter]
    fn set_center_idx(&mut self, value: i64) {
        self.inner.center = value as u64;
        self.write_back();
    }

    /// Whether the arc sweeps clockwise from start to end.
    #[getter]
    fn clockwise(&self) -> bool {
        self.inner.clockwise
    }

    #[setter]
    fn set_clockwise(&mut self, value: bool) {
        self.inner.clockwise = value;
        self.write_back();
    }

    /// Whether the entity is construction geometry.
    #[getter]
    fn construction(&self) -> bool {
        self.inner.base.construction
    }

    #[setter]
    fn set_construction(&mut self, value: bool) {
        self.inner.base.construction = value;
        self.write_back();
    }

    /// Whether the entity is excluded from toolpath output.
    #[getter]
    fn invisible(&self) -> bool {
        self.inner.base.invisible
    }

    #[setter]
    fn set_invisible(&mut self, value: bool) {
        self.inner.base.invisible = value;
        self.write_back();
    }

    /// Whether the entity was created as part of an array copy.
    #[getter]
    fn array_copy(&self) -> bool {
        self.inner.base.array_copy
    }

    #[setter]
    fn set_array_copy(&mut self, value: bool) {
        self.inner.base.array_copy = value;
        self.write_back();
    }

    /// Whether the entity is fully constrained, as computed by the
    /// solver.
    #[getter]
    fn constrained(&self) -> bool {
        self.inner.base.constrained
    }

    #[setter]
    fn set_constrained(&mut self, value: bool) {
        self.inner.base.constrained = value;
        if let Some(registry) = &self.registry {
            Python::attach(|py| {
                let _ = registry.bind(py).call_method1(
                    "set_entity_constrained",
                    (self.inner.base.id, value),
                );
            });
        }
    }

    /// Fill color as (r, g, b, a), or None.
    #[getter]
    fn fill_color(&self) -> Option<(f64, f64, f64, f64)> {
        self.inner.base.fill_color.map(|c| (c[0], c[1], c[2], c[3]))
    }

    #[setter]
    #[pyo3(name = "fill_color")]
    fn set_fill_color_attr(&mut self, value: Option<(f64, f64, f64, f64)>) {
        self.inner.base_mut().fill_color =
            value.map(|(r, g, b, a)| [r, g, b, a]);
        self.write_back();
    }

    /// The arc's midpoint in model coordinates, if defined.
    fn get_midpoint(&self, registry: PointCtx<'_>) -> Option<(f64, f64)> {
        self.inner.get_midpoint(&registry)
    }

    /// The entity type name.
    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }

    /// The entity type name (attribute alias).
    #[getter]
    #[pyo3(name = "type")]
    fn type_(&self) -> &'static str {
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
        registry: PointCtx<'_>,
    ) -> PyResult<Bound<'py, pyo3::types::PyTuple>> {
        signature_to_py(py, &self.inner.geometry_signature(&registry))
    }

    /// Returns the entity's toolpath geometry in model coordinates.
    fn to_geometry(&self, registry: PointCtx<'_>) -> PyGeometry {
        let geometry = self.inner.to_geometry(&registry);
        PyGeometry { inner: geometry }
    }

    /// Returns the entity's outline as polygon vertices.
    #[pyo3(signature = (registry, forward = true))]
    fn to_polygon_vertices(
        &self,
        registry: PointCtx<'_>,
        forward: bool,
    ) -> Vec<(f64, f64)> {
        self.inner.to_polygon_vertices(&registry, forward)
    }

    /// Returns the entity's centerline as polyline vertices.
    #[pyo3(signature = (registry, tolerance = 0.1))]
    fn to_polyline(
        &self,
        registry: PointCtx<'_>,
        tolerance: f64,
    ) -> Vec<(f64, f64)> {
        self.inner.to_polyline(&registry, tolerance)
    }

    /// Whether the entity is hit at model coordinates (mx, my).
    #[pyo3(signature = (mx, my, threshold, registry))]
    fn hit_test(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        registry: PointCtx<'_>,
    ) -> bool {
        self.inner.hit_test(mx, my, threshold, &registry)
    }

    /// Whether the model-space point lies inside the entity.
    fn contains_point(&self, registry: PointCtx<'_>, x: f64, y: f64) -> bool {
        self.inner.contains_point(&registry, x, y)
    }

    /// Signed distance from the model-space point, if defined.
    fn signed_distance_to(
        &self,
        px: f64,
        py: f64,
        registry: PointCtx<'_>,
    ) -> Option<f64> {
        self.inner.signed_distance_to(px, py, &registry)
    }

    /// Tangent direction at the point nearest the model-space point.
    fn tangent_at(
        &self,
        registry: PointCtx<'_>,
        point_id: u64,
    ) -> PyResult<(f64, f64)> {
        self.inner.tangent_at(&registry, point_id).ok_or_else(|| {
            PyNotImplementedError::new_err(
                "tangent not defined for this entity",
            )
        })
    }

    /// Sets the fill color; returns whether the entity supports
    /// fills.
    fn set_fill_color(&mut self, color: Option<[f64; 4]>) -> bool {
        let supported = self.inner.set_fill_color(color);
        self.write_back();
        supported
    }
    /// Persists in-place edits back to the owning registry, if any.
    pub(crate) fn write_back(&self) {
        if let Some(registry) = &self.registry {
            Python::attach(|py| {
                if let Ok(dict) =
                    crate::python::sketch::constraints::dict_from_value(
                        py,
                        &self.inner.to_json(),
                    )
                {
                    let _ = registry.bind(py).call_method1(
                        "replace_entity_from_dict",
                        (self.inner.base.id, dict),
                    );
                }
            });
        }
    }
    /// Entities compare equal by id (raygeo hands out fresh views).
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        if let Ok(e) = other.extract::<PyRef<'_, PyArc>>() {
            return self.inner.base.id == e.inner.base.id;
        }
        false
    }

    fn __hash__(&self) -> u64 {
        self.inner.base.id
    }
    /// Whether an angle lies within the arc's sweep.
    fn is_angle_within_sweep(
        &self,
        angle: f64,
        registry: PointCtx<'_>,
    ) -> bool {
        self.inner.is_angle_within_sweep(angle, &registry)
    }
    /// Point ids that act as drag anchors for the given point.
    fn get_drag_anchor_points(&self, point_id: u64) -> Vec<u64> {
        self.inner.get_drag_anchor_points(point_id)
    }

    /// Restores base flags serialized by to_dict() but not accepted
    /// as constructor arguments.
    fn restore_base_flags(&mut self, data: &Bound<'_, PyDict>) {
        let get = |key: &str| -> bool {
            data.get_item(key)
                .ok()
                .flatten()
                .and_then(|v| v.extract().ok())
                .unwrap_or(false)
        };
        self.inner.base_mut().invisible = get("invisible");
        self.inner.base_mut().array_copy = get("array_copy");
    }

    /// Whether the entity has a radius (arc, circle).
    fn is_radius_entity(&self) -> bool {
        self.inner.is_radius_entity()
    }

    /// Whether the entity forms a closed loop.
    fn is_closed_loop(&self) -> bool {
        self.inner.is_closed_loop()
    }

    /// Whether the entity participates in edge/chaining adjacency
    /// for loop detection (line, arc, bezier).
    fn is_edge_entity(&self) -> bool {
        self.inner.is_edge_entity()
    }

    /// The entity ids of the curve endpoints.
    fn get_endpoint_ids(&self) -> Vec<u64> {
        self.inner.get_endpoint_ids()
    }

    /// Point ids acting as junction candidates for chaining.
    fn get_junction_point_ids(&self) -> Vec<u64> {
        self.inner.get_junction_point_ids()
    }

    /// Auxiliary/child entity ids belonging to this compound entity.
    fn get_helper_ids(&self) -> Vec<u64> {
        self.inner.get_helper_ids()
    }

    /// Drops this entity's references to its helper entities.
    fn clear_helper_references(&mut self) {
        self.inner.clear_helper_references()
    }

    /// Point ids that can remain unconstrained when this entity is
    /// constrained.
    fn get_ignorable_unconstrained_points(&self) -> Vec<u64> {
        self.inner.get_ignorable_unconstrained_points()
    }

    /// Whether a point can be constrained onto this entity's curve.
    fn supports_point_on_curve(&self) -> bool {
        self.inner.supports_point_on_curve()
    }

    /// Whether this entity can receive a fill color.
    fn supports_fill(&self) -> bool {
        self.inner.supports_fill()
    }

    /// The characteristic length used by equal-length constraints.
    fn characteristic_length(&self, registry: PointCtx<'_>) -> f64 {
        self.inner.characteristic_length(&registry)
    }

    /// The defining-point pairs of the characteristic length.
    fn characteristic_length_pairs(&self) -> Vec<[u64; 2]> {
        self.inner.characteristic_length_pairs()
    }

    /// The signed area enclosed by the entity, if defined.
    fn enclosed_signed_area(&self, registry: PointCtx<'_>) -> PyResult<f64> {
        self.inner.enclosed_signed_area(&registry).ok_or_else(|| {
            PyNotImplementedError::new_err(
                "signed area not defined for this entity",
            )
        })
    }

    /// Point ids that move rigidly with the given point.
    fn get_rigidly_connected_points(&self, point_id: u64) -> Vec<u64> {
        self.inner.get_rigidly_connected_points(point_id)
    }

    /// Frame point ids of a text box, for containment checks.
    fn get_all_frame_point_ids(&self, registry: PointCtx<'_>) -> Vec<u64> {
        self.inner.get_all_frame_point_ids(&registry)
    }

    /// Solver-relevant discrete state for undo/redo snapshots.
    fn get_state<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        match self.inner.get_state() {
            Some(value) => dict_from_value(py, &value).map(Some),
            None => Ok(None),
        }
    }

    /// Restores solver-relevant discrete state from a snapshot.
    fn set_state(&mut self, state: &Bound<'_, PyDict>) -> PyResult<()> {
        let value = value_from_dict(state)?;
        self.inner.set_state(&value);
        if let Some(registry) = &self.registry {
            Python::attach(|py| {
                let _ = registry
                    .bind(py)
                    .call_method1("set_state", (self.inner.base().id, state));
            });
        }
        Ok(())
    }

    /// The entity radius at the registry's point positions.
    fn radius(&self, registry: PointCtx<'_>) -> f64 {
        self.inner.radius(&registry)
    }

    /// Updates the constrained flag from defining-point status and
    /// constraints that dimension this entity's radius or length.
    fn update_constrained_status(
        &mut self,
        registry: PointCtx<'_>,
        constraints: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let constrained =
            self.inner.compute_constrained(&registry, &|eid: u64| {
                Python::attach(|_py| {
                    let registry = registry.0.clone();
                    let mut constrained = false;
                    if let Ok(iter) = constraints.try_iter() {
                        for constraint in iter.flatten() {
                            if let Ok(flag) = constraint
                                .call_method1(
                                    "constrains_radius",
                                    (&registry, eid),
                                )
                                .and_then(|v| v.extract::<bool>())
                            {
                                if flag {
                                    constrained = true;
                                    break;
                                }
                            }
                        }
                    }
                    constrained
                })
            });
        self.inner.base_mut().constrained = constrained;
        if let Some(registry) = &self.registry {
            Python::attach(|py| {
                let _ = registry.bind(py).call_method1(
                    "set_entity_constrained",
                    (self.inner.base().id, constrained),
                );
            });
        }
        Ok(())
    }

    /// Whether the entity is fully contained by the given rect.
    fn is_contained_by(
        &self,
        rect: (f64, f64, f64, f64),
        registry: PointCtx<'_>,
    ) -> bool {
        let rect = crate::geo::types::Rect {
            min: crate::geo::types::Point::new(rect.0, rect.1),
            max: crate::geo::types::Point::new(rect.2, rect.3),
        };
        self.inner.is_contained_by(rect, &registry)
    }

    /// Whether the entity intersects the given rect.
    fn intersects_rect(
        &self,
        rect: (f64, f64, f64, f64),
        registry: PointCtx<'_>,
    ) -> bool {
        let rect = crate::geo::types::Rect {
            min: crate::geo::types::Point::new(rect.0, rect.1),
            max: crate::geo::types::Point::new(rect.2, rect.3),
        };
        self.inner.intersects_rect(rect, &registry)
    }

    /// Appends the entity's outline to the geometry.
    #[pyo3(signature = (geo, registry, forward = true))]
    fn append_to_geometry(
        &self,
        geo: &mut crate::python::geo::geometry::Geometry,
        registry: PointCtx<'_>,
        forward: bool,
    ) {
        self.inner
            .append_to_geometry(&mut geo.inner, &registry, forward);
    }

    /// The fill geometry of the entity, if it supports fills.
    fn create_fill_geometry(
        &self,
        registry: PointCtx<'_>,
    ) -> Option<crate::python::geo::geometry::Geometry> {
        self.inner.create_fill_geometry(&registry).map(|geometry| {
            crate::python::geo::geometry::Geometry { inner: geometry }
        })
    }

    /// Builds an entity from its reference dictionary shape.
    #[staticmethod]
    fn from_dict(
        py: Python<'_>,
        data: &Bound<'_, PyDict>,
    ) -> PyResult<Py<PyArc>> {
        let value = value_from_dict(data)?;
        let inner = CoreArc::from_json(&value)
            .ok_or_else(|| PyValueError::new_err("invalid arc dict"))?;
        Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyArc {
                inner,
                registry: None,
            }),
        )
    }
}

/// Registers the arc class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyArc>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities.arc", m)?;
    Ok(())
}
