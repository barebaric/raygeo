//! Bindings for the text box entity.

use crate::python::sketch::registry::value_from_dict;
use pyo3::exceptions::{PyNotImplementedError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::types::EntityId;

use super::PyEntityBase;
use crate::geo::shape::text::FontConfig as CoreFontConfig;
use crate::python::geo::geometry::Geometry as PyGeometry;
use crate::python::geo::shape::text::PyFontConfig;
use crate::python::sketch::constraints::dict_from_value;
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::entities::signature_to_py;
use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::entities::mirror::{MirrorAxis, MirrorDirection};
use crate::sketch::entities::text_box::TextBoxEntity as CoreTextBox;
use crate::sketch::entities::Entity;

/// A text box framed by origin, width and height points.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyEntityBase,
    subclass,module = "raygeo.sketch.entities.text_box", name = "TextBox")]
pub struct PyTextBox {
    pub inner: CoreTextBox,
    /// Live back-reference to the owning registry, if any.
    pub registry: Option<Py<PyEntityRegistry>>,
}

impl PyTextBox {
    /// Reads the current entity state from the owning registry so
    /// views stay live across edits; falls back to the local snapshot
    /// when unattached.
    fn live(&self) -> CoreTextBox {
        if let Some(registry) = &self.registry {
            return Python::attach(|py| {
                if let Ok(reg) = registry.bind(py).try_borrow() {
                    if let Some(entity) =
                        reg.inner.get_entity(self.inner.base.id)
                    {
                        if let Some(tb) =
                            entity.as_any().downcast_ref::<CoreTextBox>()
                        {
                            return tb.clone();
                        }
                    }
                }
                self.inner.clone()
            });
        }
        self.inner.clone()
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTextBox {
    /// Creates a text box with the given id and frame point ids.
    #[new]
    #[pyo3(signature = (id, origin_id, width_id, height_id,
                        content = String::new(), font_config = None,
                        construction = false, construction_line_ids = None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: i64,
        origin_id: i64,
        width_id: i64,
        height_id: i64,
        content: String,
        font_config: Option<&Bound<'_, PyAny>>,
        construction: bool,
        construction_line_ids: Option<Vec<i64>>,
    ) -> PyClassInitializer<Self> {
        let mut inner = CoreTextBox::new(
            id as u64,
            origin_id as u64,
            width_id as u64,
            height_id as u64,
            content,
            construction,
        );
        if let Some(config) = font_config {
            if let Ok(size) =
                config.getattr("size").and_then(|v| v.extract::<f64>())
            {
                let family: String = config
                    .getattr("family")
                    .and_then(|v| v.extract())
                    .unwrap_or_else(|_| "sans-serif".to_string());
                inner.font_config = CoreFontConfig::new(&family, size)
                    .bold(
                        config
                            .getattr("bold")
                            .and_then(|v| v.extract())
                            .unwrap_or(false),
                    )
                    .italic(
                        config
                            .getattr("italic")
                            .and_then(|v| v.extract())
                            .unwrap_or(false),
                    );
            }
        }
        inner.construction_line_ids = construction_line_ids
            .unwrap_or_default()
            .into_iter()
            .map(|id| id as u64)
            .collect();
        PyClassInitializer::from(PyEntityBase).add_subclass(PyTextBox {
            inner,
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

    /// Origin point id.
    #[getter]
    fn origin_id(&self) -> i64 {
        self.inner.origin as i64
    }

    #[setter]
    fn set_origin_id(&mut self, value: i64) {
        self.inner.origin = value as u64;
    }

    /// Width point id.
    #[getter]
    fn width_id(&self) -> i64 {
        self.inner.width as i64
    }

    #[setter]
    fn set_width_id(&mut self, value: i64) {
        self.inner.width = value as u64;
    }

    /// Height point id.
    #[getter]
    fn height_id(&self) -> i64 {
        self.inner.height as i64
    }

    #[setter]
    fn set_height_id(&mut self, value: i64) {
        self.inner.height = value as u64;
    }

    /// The text content.
    /// The ids of the frame construction lines.
    /// The fourth frame corner point id, if resolvable.
    fn get_fourth_corner_id(&self, registry: PointCtx<'_>) -> Option<u64> {
        self.inner.get_fourth_corner_id(&registry)
    }

    #[getter]
    fn construction_line_ids(&self) -> Vec<i64> {
        self.inner
            .construction_line_ids
            .iter()
            .map(|&id| id as i64)
            .collect()
    }

    #[setter]
    fn set_construction_line_ids(&mut self, value: Vec<i64>) {
        self.inner.construction_line_ids =
            value.iter().map(|&id| id as u64).collect();
    }

    #[getter]
    fn content(&self) -> String {
        self.live().content.clone()
    }

    #[setter]
    fn set_content(&mut self, value: String) {
        self.inner.content = value;
        self.write_back();
    }

    /// The font family name.
    #[getter]
    fn font_family(&self) -> String {
        self.live().font_config.family.clone()
    }

    /// The font size in points.
    #[getter]
    fn font_size(&self) -> f64 {
        self.live().font_config.size
    }

    /// Whether the text is bold.
    #[getter]
    fn bold(&self) -> bool {
        self.live().font_config.bold
    }

    /// Whether the text is italic.
    #[getter]
    fn italic(&self) -> bool {
        self.live().font_config.italic
    }

    /// The font configuration as a FontConfig object.
    #[getter]
    fn font_config(&self, py: Python<'_>) -> PyFontConfig {
        let live = self.live();
        let extra = PyDict::new(py);
        for (key, value) in &live.font_extra {
            if let Ok(obj) =
                crate::python::sketch::constraints::value_to_py(py, value)
            {
                let _ = extra.set_item(key.as_str(), obj);
            }
        }
        PyFontConfig {
            core: live.font_config.clone(),
            extra: extra.into(),
        }
    }

    #[setter]
    fn set_font_config(&mut self, value: PyRef<'_, PyFontConfig>) {
        self.inner.font_config = value.core.clone();
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
        let geometry = Entity::to_geometry(&self.inner, &registry);
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
        let supported = self.inner.set_fill_color(color);
        self.write_back();
        supported
    }
    /// Persists in-place edits back to the owning registry, if any.
    fn write_back(&self) {
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
        if let Ok(e) = other.extract::<PyRef<'_, PyTextBox>>() {
            return self.inner.base.id == e.inner.base.id;
        }
        false
    }

    fn __hash__(&self) -> u64 {
        self.inner.base.id
    }
    /// Point ids that act as drag anchors for the given point.
    fn get_drag_anchor_points(&self, point_id: u64) -> Vec<u64> {
        self.inner.get_drag_anchor_points(point_id)
    }
    /// All four frame corner point ids.
    fn get_all_frame_point_ids(&self, registry: PointCtx<'_>) -> Vec<u64> {
        self.inner.get_all_frame_point_ids(&registry)
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
        Ok(())
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
    ) -> PyResult<Py<PyTextBox>> {
        let value = value_from_dict(data)?;
        let inner = CoreTextBox::from_json(&value)
            .ok_or_else(|| PyValueError::new_err("invalid text_box dict"))?;
        Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyTextBox {
                inner,
                registry: None,
            }),
        )
    }

    /// The four-corner frame polygon in world coordinates.
    fn get_frame_polygon(
        &self,
        registry: PointCtx<'_>,
    ) -> Option<Vec<(f64, f64)>> {
        self.inner.get_frame_polygon(&registry)
    }

    /// Natural (width, height) of the text content.
    fn get_natural_size(&self, content: Option<String>) -> (f64, f64) {
        self.inner.get_natural_size(content.as_deref())
    }

    /// Font metrics `(ascent, descent, height)` for the configured font.
    fn get_font_metrics(&self) -> (f64, f64, f64) {
        crate::geo::shape::text::get_font_metrics(&self.inner.font_config)
            .unwrap_or((0.0, 0.0, 0.0))
    }
}

/// Registers the text_box class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTextBox>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities.text_box", m)?;
    Ok(())
}
