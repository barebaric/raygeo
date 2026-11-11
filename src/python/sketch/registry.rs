//! Python bindings for the sketch data model.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use serde_json::Value;

use crate::geo::types::Rect;
use crate::python::geo::geometry::Geometry as PyGeometry;
use crate::sketch::entities::mirror::{MirrorAxis, MirrorDirection};
use crate::sketch::entities::point::{SketchPoint, WaypointType};
use crate::sketch::registry::EntityRegistry;
use crate::sketch::registry::FontConfig;

/// Converts a Python dictionary to a serde_json value.
fn value_from_dict(dict: &Bound<'_, PyDict>) -> PyResult<Value> {
    let mut map = serde_json::Map::new();
    for (key, value) in dict.iter() {
        let key: String = key.extract()?;
        map.insert(key, any_to_value(&value)?);
    }
    Ok(Value::Object(map))
}

fn any_to_value(value: &Bound<'_, PyAny>) -> PyResult<Value> {
    if value.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = value.extract::<bool>() {
        return Ok(Value::Bool(b));
    }
    if let Ok(i) = value.extract::<i64>() {
        return Ok(Value::Number(i.into()));
    }
    if let Ok(f) = value.extract::<f64>() {
        return Ok(serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null));
    }
    if let Ok(s) = value.extract::<String>() {
        return Ok(Value::String(s));
    }
    if let Ok(list) = value.extract::<Vec<Bound<'_, PyAny>>>() {
        return Ok(Value::Array(
            list.iter().map(any_to_value).collect::<PyResult<_>>()?,
        ));
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        return value_from_dict(dict);
    }
    Err(PyValueError::new_err(
        "unsupported value type in sketch registry dict",
    ))
}

fn value_to_py<'py>(
    py: Python<'py>,
    value: &Value,
) -> PyResult<Bound<'py, PyAny>> {
    use pyo3::types::PyList;
    match value {
        Value::Null => Ok(py.None().into_bound(py)),
        Value::Bool(b) => {
            let owned = (*b).into_pyobject(py)?.to_owned();
            Ok(owned.into_any())
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any())
            } else {
                let f = n.as_f64().unwrap_or(0.0);
                Ok(f.into_pyobject(py)?.into_any())
            }
        }
        Value::String(s) => Ok(s.into_pyobject(py)?.into_any()),
        Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(list.into_any())
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, item) in map {
                dict.set_item(key, value_to_py(py, item)?)?;
            }
            Ok(dict.into_any())
        }
    }
}

/// The sketch document model: points, entities, id allocation.
///
/// Serializes to and from the same dictionaries the reference
/// entities produce (`.ryp` compatible).
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.registry",
    name = "EntityRegistry",
    skip_from_py_object
)]
pub struct PyEntityRegistry {
    inner: EntityRegistry,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEntityRegistry {
    /// Creates an empty model.
    #[new]
    fn new() -> Self {
        PyEntityRegistry {
            inner: EntityRegistry::new(),
        }
    }

    /// Deserializes the model from a reference registry dictionary.
    #[staticmethod]
    fn from_dict(dict: &Bound<'_, PyDict>) -> PyResult<Self> {
        let value = value_from_dict(dict)?;
        EntityRegistry::from_json(&value)
            .map(|inner| PyEntityRegistry { inner })
            .ok_or_else(|| PyValueError::new_err("invalid registry dict"))
    }

    /// Serializes the model to a reference registry dictionary.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let value = self.inner.to_json();
        let dict = PyDict::new(py);
        match &value {
            Value::Object(map) => {
                for (key, item) in map {
                    dict.set_item(key, value_to_py(py, item)?)?;
                }
            }
            _ => return Err(PyValueError::new_err("serialization error")),
        }
        Ok(dict)
    }

    /// Adds a point and returns its id.
    #[pyo3(signature = (x, y, fixed = false))]
    fn add_point(&mut self, x: f64, y: f64, fixed: bool) -> u64 {
        self.inner.add_point(x, y, fixed)
    }

    /// Appends pre-built points given as ``(id, x, y, fixed)`` tuples.
    fn add_points(&mut self, points: Vec<(u64, f64, f64, bool)>) {
        self.inner.add_points(
            points
                .into_iter()
                .map(|(id, x, y, fixed)| SketchPoint {
                    id,
                    x,
                    y,
                    fixed,
                    waypoint_type: WaypointType::Sharp,
                    constrained: false,
                })
                .collect(),
        );
    }

    /// Removes the given points.
    fn remove_points(&mut self, point_ids: Vec<u64>) {
        self.inner.remove_points(&point_ids);
    }

    /// Adds a line and returns its id.
    #[pyo3(signature = (p1, p2, construction = false))]
    fn add_line(&mut self, p1: u64, p2: u64, construction: bool) -> u64 {
        self.inner.add_line(p1, p2, construction)
    }

    /// Adds an arc and returns its id.
    #[pyo3(signature = (start, end, center, clockwise = false, construction = false))]
    fn add_arc(
        &mut self,
        start: u64,
        end: u64,
        center: u64,
        clockwise: bool,
        construction: bool,
    ) -> u64 {
        self.inner
            .add_arc(start, end, center, clockwise, construction)
    }

    /// Adds a circle and returns its id.
    #[pyo3(signature = (center, radius_point, construction = false))]
    fn add_circle(
        &mut self,
        center: u64,
        radius_point: u64,
        construction: bool,
    ) -> u64 {
        self.inner.add_circle(center, radius_point, construction)
    }

    /// Adds an ellipse and returns its id.
    #[pyo3(signature = (center, radius_x_point, radius_y_point, construction = false))]
    fn add_ellipse(
        &mut self,
        center: u64,
        radius_x_point: u64,
        radius_y_point: u64,
        construction: bool,
    ) -> u64 {
        self.inner.add_ellipse(
            center,
            radius_x_point,
            radius_y_point,
            construction,
        )
    }

    /// Adds a bezier and returns its id.
    #[pyo3(signature = (start, end, construction = false, cp1 = None, cp2 = None))]
    fn add_bezier(
        &mut self,
        start: u64,
        end: u64,
        construction: bool,
        cp1: Option<(f64, f64)>,
        cp2: Option<(f64, f64)>,
    ) -> u64 {
        self.inner.add_bezier(
            start,
            end,
            construction,
            cp1.map(|p| [p.0, p.1]),
            cp2.map(|p| [p.0, p.1]),
        )
    }

    /// Adds a polygon and returns its id.
    #[pyo3(signature = (center, handle, vertices, closed = false, construction = false, rings = None))]
    fn add_polygon(
        &mut self,
        center: u64,
        handle: u64,
        vertices: Vec<(f64, f64)>,
        closed: bool,
        construction: bool,
        rings: Option<Vec<Vec<(f64, f64)>>>,
    ) -> u64 {
        let to_pairs = |list: Vec<(f64, f64)>| -> Vec<(f64, f64)> { list };
        self.inner.add_polygon(
            center,
            handle,
            to_pairs(vertices),
            closed,
            construction,
            rings
                .map(|r| r.into_iter().map(to_pairs).collect())
                .unwrap_or_default(),
        )
    }

    /// Adds a text box and returns its id.
    #[pyo3(signature = (origin, width, height, content = String::new(), font_family = "sans-serif".to_string(), font_size = 10.0, bold = false, italic = false))]
    #[allow(clippy::too_many_arguments)]
    fn add_text_box(
        &mut self,
        origin: u64,
        width: u64,
        height: u64,
        content: String,
        font_family: String,
        font_size: f64,
        bold: bool,
        italic: bool,
    ) -> u64 {
        self.inner.add_text_box(
            origin,
            width,
            height,
            content,
            FontConfig {
                family: font_family,
                size: font_size,
                bold,
                italic,
                extra: serde_json::Map::new(),
            },
            Vec::new(),
        )
    }

    /// Removes entities by id.
    fn remove_entities(&mut self, entity_ids: Vec<u64>) {
        self.inner.remove_entities_by_id(&entity_ids);
    }

    /// Point count.
    fn point_count(&self) -> usize {
        self.inner.point_count()
    }

    /// Entity count.
    fn entity_count(&self) -> usize {
        self.inner.entity_count()
    }

    /// All point ids in insertion order.
    fn point_ids(&self) -> Vec<u64> {
        self.inner.point_ids()
    }

    /// All entity ids in insertion order.
    fn entity_ids(&self) -> Vec<u64> {
        self.inner.entity_ids()
    }

    /// Sets a point's solver-constrained flag.
    fn set_point_constrained(&mut self, id: u64, constrained: bool) -> bool {
        self.inner.set_point_constrained(id, constrained)
    }

    /// Sets a point's coordinates.
    fn set_point_position(&mut self, id: u64, x: f64, y: f64) -> bool {
        self.inner.set_point_position(id, x, y)
    }

    /// Whether the point is referenced by any entity.
    fn is_point_used(&self, id: u64) -> bool {
        self.inner.is_point_used(id)
    }

    /// The serialization type string of an entity, or ``None``.
    fn entity_type(&self, id: u64) -> Option<&'static str> {
        self.inner.entity_type(id)
    }

    /// Control point ids of an entity, or ``None``.
    fn entity_point_ids(&self, id: u64) -> Option<Vec<u64>> {
        self.inner.entity_point_ids(id)
    }

    /// Endpoint ids of an edge entity, or ``None``.
    fn entity_endpoint_ids(&self, id: u64) -> Option<Vec<u64>> {
        self.inner.entity_endpoint_ids(id)
    }

    /// Monotonic entity-list version.
    fn entity_version(&self) -> u64 {
        self.inner.entity_version()
    }

    /// Whether the entity participates in edge/chaining adjacency.
    fn is_edge_entity(&self, id: u64) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| e.is_edge_entity())
            .unwrap_or(false)
    }

    /// Whether the entity has a radius.
    fn is_radius_entity(&self, id: u64) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| e.is_radius_entity())
            .unwrap_or(false)
    }

    /// Whether the entity alone forms a closed loop.
    fn is_closed_loop(&self, id: u64) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| e.is_closed_loop())
            .unwrap_or(false)
    }

    /// Whether a point can be constrained onto this entity's curve.
    fn supports_point_on_curve(&self, id: u64) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| e.supports_point_on_curve())
            .unwrap_or(false)
    }

    /// Whether the entity is fully constrained.
    fn is_constrained(&self, id: u64) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| e.base().constrained)
            .unwrap_or(false)
    }

    /// Whether the entity is a construction entity.
    fn is_construction(&self, id: u64) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| e.base().construction)
            .unwrap_or(false)
    }

    /// Sets the construction flag.
    fn set_construction(&mut self, id: u64, construction: bool) {
        let Some(entity) = self.inner.get_entity_mut(id) else {
            return;
        };
        entity.base_mut().construction = construction;
    }

    /// Adds a constraint from a reference constraint dictionary
    /// (`.ryp` shape), resolving entity references through the
    /// registry.  Returns the constraint handle.
    fn add_constraint_json(
        &mut self,
        dict: &Bound<'_, PyDict>,
    ) -> PyResult<u64> {
        let value = value_from_dict(dict)?;
        self.inner
            .add_constraint_json(&value)
            .map_err(PyValueError::new_err)
    }

    /// Number of stored constraints.
    fn constraint_count(&self) -> usize {
        self.inner.constraint_count()
    }

    /// The stored reference dictionary of a constraint.
    fn constraint_dict<'py>(
        &self,
        py: Python<'py>,
        index: u64,
    ) -> PyResult<Bound<'py, PyDict>> {
        let Some(value) = self.inner.constraint_dict(index) else {
            return Err(PyValueError::new_err("constraint index out of range"));
        };
        let dict = PyDict::new(py);
        if let Value::Object(map) = &value {
            for (key, item) in map {
                dict.set_item(key, value_to_py(py, item)?)?;
            }
        }
        Ok(dict)
    }

    /// Removes a constraint by handle.
    fn remove_constraint(&mut self, index: u64) {
        self.inner.remove_constraint(index);
    }

    /// Residual values of all stored constraints at the current
    /// point positions.
    fn constraint_residuals(&self) -> Vec<f64> {
        self.inner.constraint_residuals()
    }

    /// Whether the point is solver-constrained.
    fn point_constrained(&self, id: u64) -> bool {
        self.inner
            .get_point(id)
            .map(|p| p.constrained)
            .unwrap_or(false)
    }

    /// Waypoint type string of a point ("sharp"/"smooth"/"symmetric").
    fn waypoint_type(&self, id: u64) -> Option<String> {
        self.inner
            .get_point(id)
            .map(|p| p.waypoint_type.as_str().to_string())
    }

    /// Coordinates of a point as ``(x, y)``, or ``None``.
    fn get_point(&self, id: u64) -> Option<(f64, f64)> {
        self.inner.point_position(id)
    }

    /// Junction point ids of an entity.
    fn junction_point_ids(&self, id: u64) -> Vec<u64> {
        self.inner
            .get_entity(id)
            .map(|e| e.get_junction_point_ids())
            .unwrap_or_default()
    }

    /// Helper/child ids of a compound entity.
    fn helper_ids(&self, id: u64) -> Vec<u64> {
        self.inner
            .get_entity(id)
            .map(|e| e.get_helper_ids())
            .unwrap_or_default()
    }

    /// Drops the entity's references to its helper entities.
    fn clear_helper_references(&mut self, id: u64) {
        let Some(entity) = self.inner.get_entity_mut(id) else {
            return;
        };
        entity.clear_helper_references();
    }

    /// Points that move rigidly with `point_id`.
    fn rigidly_connected_points(&self, point_id: u64) -> Vec<u64> {
        self.inner.get_rigidly_connected_points(point_id)
    }

    /// Points pinned while `point_id` is dragged.
    fn drag_anchor_points(&self, point_id: u64) -> Vec<u64> {
        self.inner.get_drag_anchor_points(point_id)
    }

    /// Points that can stay unconstrained when the entity is
    /// constrained.
    fn ignorable_unconstrained_points(&self, id: u64) -> Vec<u64> {
        self.inner
            .get_entity(id)
            .map(|e| e.get_ignorable_unconstrained_points())
            .unwrap_or_default()
    }

    /// Characteristic length used by equal-length constraints.
    fn characteristic_length(&self, id: u64) -> f64 {
        self.inner
            .get_entity(id)
            .map(|e| e.characteristic_length(&self.inner))
            .unwrap_or(0.0)
    }

    /// Signed area enclosed by a closed single-entity loop.
    fn enclosed_signed_area(&self, id: u64) -> Option<f64> {
        self.inner
            .get_entity(id)
            .and_then(|e| e.enclosed_signed_area(&self.inner))
    }

    /// Whether the point lies within threshold distance of the
    /// entity.
    #[pyo3(signature = (id, mx, my, threshold))]
    fn hit_test(&self, id: u64, mx: f64, my: f64, threshold: f64) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| e.hit_test(mx, my, threshold, &self.inner))
            .unwrap_or(false)
    }

    /// Whether the entity is fully strictly contained in the rect.
    fn is_contained_by(&self, id: u64, rect: [f64; 4]) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| {
                e.is_contained_by(
                    Rect::new(rect[0], rect[1], rect[2], rect[3]),
                    &self.inner,
                )
            })
            .unwrap_or(false)
    }

    /// Whether the entity intersects the rect or is contained by it.
    fn intersects_rect(&self, id: u64, rect: [f64; 4]) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| {
                e.intersects_rect(
                    Rect::new(rect[0], rect[1], rect[2], rect[3]),
                    &self.inner,
                )
            })
            .unwrap_or(false)
    }

    /// Whether a point is inside the entity (closed shapes).
    fn contains_point(&self, id: u64, x: f64, y: f64) -> bool {
        self.inner
            .get_entity(id)
            .map(|e| e.contains_point(&self.inner, x, y))
            .unwrap_or(false)
    }

    /// Signed distance from a point to the entity's locus, or
    /// ``None`` when unsupported.
    #[pyo3(signature = (id, px, py))]
    fn signed_distance_to(&self, id: u64, px: f64, py: f64) -> Option<f64> {
        self.inner
            .get_entity(id)
            .and_then(|e| e.signed_distance_to(px, py, &self.inner))
    }

    /// Tangent vector at a defining point, or ``None`` when the
    /// entity has no directional tangent.
    fn tangent_at(&self, id: u64, point_id: u64) -> Option<(f64, f64)> {
        self.inner
            .get_entity(id)
            .and_then(|e| e.tangent_at(&self.inner, point_id))
    }

    /// The entity's geometry signature (quantized defining points
    /// plus kind-specific state), or ``None`` for unknown ids.
    fn geometry_signature<'py>(
        &self,
        py: Python<'py>,
        id: u64,
    ) -> PyResult<Bound<'py, PyAny>> {
        match self
            .inner
            .get_entity(id)
            .map(|e| e.geometry_signature(&self.inner))
        {
            Some(value) => value_to_py(py, &value),
            None => Ok(py.None().into_bound(py)),
        }
    }

    /// Solver-relevant state snapshot of an entity, or ``None``.
    fn get_state<'py>(
        &self,
        py: Python<'py>,
        id: u64,
    ) -> PyResult<Bound<'py, PyAny>> {
        match self.inner.get_entity(id).and_then(|e| e.get_state()) {
            Some(state) => value_to_py(py, &state),
            None => Ok(py.None().into_bound(py)),
        }
    }

    /// Restores an entity's state snapshot.
    fn set_state(
        &mut self,
        id: u64,
        state: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        let value = value_from_dict(state)?;
        let Some(entity) = self.inner.get_entity_mut(id) else {
            return Ok(());
        };
        entity.set_state(&value);
        Ok(())
    }

    /// The entity as a raygeo Geometry object.
    #[allow(clippy::borrowed_box)]
    fn to_geometry(&self, id: u64) -> PyGeometry {
        let Some(entity) = self.inner.get_entity(id) else {
            return PyGeometry {
                inner: crate::geo::geometry::Geometry::new(),
            };
        };
        let geo = entity.to_geometry(&self.inner);
        PyGeometry { inner: geo }
    }

    /// Polygon vertices of the entity (sampled for curves).
    #[pyo3(signature = (id, forward))]
    fn to_polygon_vertices(&self, id: u64, forward: bool) -> Vec<(f64, f64)> {
        self.inner
            .get_entity(id)
            .map(|e| e.to_polygon_vertices(&self.inner, forward))
            .unwrap_or_default()
    }

    /// Sampled polyline of the entity.
    #[pyo3(signature = (id, tolerance = 0.1))]
    fn to_polyline(&self, id: u64, tolerance: f64) -> Vec<(f64, f64)> {
        self.inner
            .get_entity(id)
            .map(|e| e.to_polyline(&self.inner, tolerance))
            .unwrap_or_default()
    }

    /// Sampled polylines of the entity (multi-contour for text).
    #[pyo3(signature = (id, tolerance = 0.1))]
    fn to_polylines(&self, id: u64, tolerance: f64) -> Vec<Vec<(f64, f64)>> {
        self.inner
            .get_entity(id)
            .map(|e| e.to_polylines(&self.inner, tolerance))
            .unwrap_or_default()
    }

    /// Appends the entity's geometry (traversal direction aware) to
    /// an existing Geometry object.
    fn append_to_geometry(&self, geo: &mut PyGeometry, id: u64, forward: bool) {
        let Some(entity) = self.inner.get_entity(id) else {
            return;
        };
        entity.append_to_geometry(&mut geo.inner, &self.inner, forward);
    }

    /// Re-computes constrained flags.  Entities whose radius is
    /// explicitly dimensioned are listed in
    /// ``radius_constrained_entity_ids``.
    #[pyo3(signature = (radius_constrained_entity_ids = Vec::new()))]
    fn update_constrained_status(
        &mut self,
        radius_constrained_entity_ids: Vec<u64>,
    ) {
        let radius_ids: std::collections::HashSet<u64> =
            radius_constrained_entity_ids.into_iter().collect();
        let constrains_radius =
            move |entity_id: u64| radius_ids.contains(&entity_id);
        self.inner.update_constrained_status(&constrains_radius);
    }

    /// Mirrors entity-internal state across the axis (arc chirality,
    /// bezier offsets, polygon winding).
    fn mirror(
        &mut self,
        id: u64,
        direction: &str,
        position: f64,
    ) -> PyResult<()> {
        let direction = match direction {
            "vertical" => MirrorDirection::Vertical,
            "horizontal" => MirrorDirection::Horizontal,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown mirror direction: {other}"
                )))
            }
        };
        let Some(entity) = self.inner.get_entity_mut(id) else {
            return Ok(());
        };
        entity.mirror(MirrorAxis {
            direction,
            position,
        });
        Ok(())
    }

    /// Serializes the entity to a reference entity dictionary.
    fn entity_to_dict<'py>(
        &self,
        py: Python<'py>,
        id: u64,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        if let Some(entity) = self.inner.get_entity(id) {
            let value = entity.to_json();
            if let Value::Object(map) = &value {
                for (key, item) in map {
                    dict.set_item(key, value_to_py(py, item)?)?;
                }
            }
        }
        Ok(dict)
    }
}

pyo3_stub_gen::module_doc!("raygeo.sketch.registry", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "Sketch document model.

Model is the registry of points and entities of a sketch document:
it stores the same data as the reference entity classes and
serializes to and from the same dictionaries (`.ryp` compatible).
Entities are addressed by id; use add_line/add_arc/add_circle/
add_ellipse/add_bezier/add_polygon/add_text_box to extend the
document and entity_point_ids/entity_endpoint_ids/entity_type for
polymorphic queries.
";

/// Registers the model class with the given module.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let model_mod = PyModule::new(py, "registry")?;

    model_mod.setattr("__doc__", MODULE_DOC)?;
    model_mod.add_class::<PyEntityRegistry>()?;

    m.add_submodule(&model_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.registry", &model_mod)?;

    Ok(())
}
