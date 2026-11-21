//! Python bindings for the sketch entities, mirroring the
//! `sketch::entities` module tree: one leaf module per entity kind.

pub(crate) mod arc;
pub(crate) mod bezier;
pub(crate) mod circle;
pub(crate) mod ellipse;
pub(crate) mod line;
pub(crate) mod offset;
pub(crate) mod point;
pub(crate) mod polygon;
pub(crate) mod text_box;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction};
use pyo3_stub_gen::{ImportKind, ImportRef, ModuleRef, PyStubType, TypeInfo};
use std::collections::{HashMap, HashSet};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::entities::entity::Entity;

/// Rounds a coordinate to the model's six-decimal precision.
#[gen_stub_pyfunction(module = "raygeo.sketch.entities")]
#[pyfunction]
fn quantize(value: f64) -> f64 {
    crate::sketch::entities::entity::quantize(value)
}

/// Formats a geometry signature as the reference tuple shape:
/// sorted `(key, value)` pairs for objects, the values for arrays.
pub(crate) fn signature_to_py<'py>(
    py: Python<'py>,
    value: &serde_json::Value,
) -> PyResult<Bound<'py, pyo3::types::PyTuple>> {
    use pyo3::types::{PyString, PyTuple};

    let items: Vec<Bound<'py, PyAny>> = match value {
        serde_json::Value::Object(map) => {
            let mut pairs: Vec<(&String, &serde_json::Value)> =
                map.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            let mut items = Vec::with_capacity(pairs.len());
            for (key, item) in pairs {
                let key = PyString::new(py, key);
                let value =
                    crate::python::sketch::constraints::value_to_py(py, item)?;
                let pair = PyTuple::new(py, [key.into_any(), value])?;
                items.push(pair.into_any());
            }
            items
        }
        serde_json::Value::Array(array) => array
            .iter()
            .map(|item| {
                crate::python::sketch::constraints::value_to_py(py, item)
            })
            .collect::<PyResult<Vec<_>>>()?,
        other => {
            vec![crate::python::sketch::constraints::value_to_py(py, other)?]
        }
    };
    PyTuple::new(py, items)
}

/// Builds the Python object for an entity from its concrete type.
pub(crate) fn entity_to_py<'py>(
    py: Python<'py>,
    entity: &dyn Entity,
    registry: Option<&Bound<'_, PyEntityRegistry>>,
) -> PyResult<Bound<'py, PyAny>> {
    use crate::python::sketch::entities::arc::PyArc;
    use crate::python::sketch::entities::bezier::PyBezier;
    use crate::python::sketch::entities::circle::PyCircle;
    use crate::python::sketch::entities::ellipse::PyEllipse;
    use crate::python::sketch::entities::line::PyLine;
    use crate::python::sketch::entities::polygon::PyPolygon;
    use crate::python::sketch::entities::text_box::PyTextBox;
    use crate::sketch::entities::arc::Arc;
    use crate::sketch::entities::bezier::Bezier;
    use crate::sketch::entities::circle::Circle;
    use crate::sketch::entities::ellipse::Ellipse;
    use crate::sketch::entities::line::Line;
    use crate::sketch::entities::polygon::PolygonEntity;
    use crate::sketch::entities::text_box::TextBoxEntity;

    let registry = registry.map(|reg| reg.clone().unbind());
    let any = entity.as_any();
    if let Some(line) = any.downcast_ref::<Line>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyLine {
                inner: line.clone(),
                registry: registry.clone(),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(arc) = any.downcast_ref::<Arc>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyArc {
                inner: arc.clone(),
                registry: registry.clone(),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(circle) = any.downcast_ref::<Circle>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyCircle {
                inner: circle.clone(),
                registry: registry.clone(),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(ellipse) = any.downcast_ref::<Ellipse>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyEllipse {
                inner: ellipse.clone(),
                helper_line_ids: ellipse
                    .helper_line_ids
                    .iter()
                    .map(|&id| id as i64)
                    .collect(),
                registry: registry.clone(),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(bezier) = any.downcast_ref::<Bezier>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyBezier {
                inner: bezier.clone(),
                registry: registry.clone(),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(polygon) = any.downcast_ref::<PolygonEntity>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyPolygon {
                inner: polygon.clone(),
                registry: registry.clone(),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(text_box) = any.downcast_ref::<TextBoxEntity>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyTextBox {
                inner: text_box.clone(),
                registry: registry.clone(),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    Err(PyTypeError::new_err(format!(
        "unsupported entity kind: {}",
        entity.type_name()
    )))
}

/// Extracts an owned entity from a Python entity object.
pub(crate) fn extract_entity(
    obj: &Bound<'_, PyAny>,
) -> PyResult<Box<dyn Entity>> {
    use crate::python::sketch::entities::arc::PyArc;
    use crate::python::sketch::entities::bezier::PyBezier;
    use crate::python::sketch::entities::circle::PyCircle;
    use crate::python::sketch::entities::ellipse::PyEllipse;
    use crate::python::sketch::entities::line::PyLine;
    use crate::python::sketch::entities::polygon::PyPolygon;
    use crate::python::sketch::entities::text_box::PyTextBox;

    if let Ok(line) = obj.extract::<PyRef<'_, PyLine>>() {
        return Ok(Box::new(line.inner.clone()));
    }
    if let Ok(arc) = obj.extract::<PyRef<'_, PyArc>>() {
        return Ok(Box::new(arc.inner.clone()));
    }
    if let Ok(circle) = obj.extract::<PyRef<'_, PyCircle>>() {
        return Ok(Box::new(circle.inner.clone()));
    }
    if let Ok(ellipse) = obj.extract::<PyRef<'_, PyEllipse>>() {
        return Ok(Box::new(ellipse.inner.clone()));
    }
    if let Ok(bezier) = obj.extract::<PyRef<'_, PyBezier>>() {
        return Ok(Box::new(bezier.inner.clone()));
    }
    if let Ok(polygon) = obj.extract::<PyRef<'_, PyPolygon>>() {
        return Ok(Box::new(polygon.inner.clone()));
    }
    if let Ok(text_box) = obj.extract::<PyRef<'_, PyTextBox>>() {
        return Ok(Box::new(text_box.inner.clone()));
    }
    Err(PyTypeError::new_err("expected a sketch entity object"))
}

/// The common base class of every entity binding: a nominal `Entity`
/// type for `isinstance` checks and annotations.  The per-class
/// members live on the subclasses; polymorphism is carried by the
/// Rust-side `Entity` trait.
#[gen_stub_pyclass]
#[pyclass(subclass, module = "raygeo.sketch.entities", name = "Entity")]
#[derive(Default)]
pub struct PyEntityBase;

/// Runs `$body` with the mutable core entity behind the compiled
/// binding, dispatching over the leaf classes.  Evaluates to
/// `Err(PyAttributeError)` when `slf` is not an entity binding.
macro_rules! with_entity_mut {
    ($slf:expr, |$entity:ident| $body:expr; $($py_ty:ty),+ $(,)?) => {{
        let slf = $slf.as_any();
        $(
            if let Ok(mut binding) = slf.extract::<PyRefMut<'_, $py_ty>>() {
                let $entity: &mut dyn $crate::sketch::entities::entity::Entity =
                    &mut binding.inner;
                let result = $body;
                return Ok(result);
            }
        )+
        Err(PyAttributeError::new_err(
            "expected a raygeo.sketch.entities entity object",
        ))
    }};
}

/// Like [`with_entity_mut`], but persists the edit to the owning
/// registry when the binding is a live view.
macro_rules! with_entity_mut_write {
    ($slf:expr, |$entity:ident| $body:expr; $($py_ty:ty),+ $(,)?) => {{
        let slf = $slf.as_any();
        $(
            if let Ok(mut binding) = slf.extract::<PyRefMut<'_, $py_ty>>() {
                let result = {
                    let $entity: &mut dyn $crate::sketch::entities::entity::Entity =
                        &mut binding.inner;
                    $body
                };
                binding.write_back();
                return Ok(result);
            }
        )+
        Err(PyAttributeError::new_err(
            "expected a raygeo.sketch.entities entity object",
        ))
    }};
}

use pyo3::exceptions::{PyAttributeError, PyNotImplementedError, PyValueError};
use pyo3::types::{PyAny, PyDict};
use pyo3_stub_gen::derive::gen_stub_pymethods;

#[gen_stub_pymethods]
#[pymethods]
impl PyEntityBase {
    /// The unique entity id.
    #[getter]
    fn id(slf: &Bound<'_, Self>) -> PyResult<i64> {
        Ok(extract_entity(slf.as_any())?.base().id as i64)
    }

    #[setter]
    fn set_id(slf: &Bound<'_, Self>, value: i64) -> PyResult<()> {
        with_entity_mut!(slf, |entity| {
            entity.base_mut().id = value as u64;
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// The entity kind label (`"line"`, `"arc"`, ...).
    #[getter]
    #[pyo3(name = "type")]
    fn type_(slf: &Bound<'_, Self>) -> PyResult<String> {
        Ok(extract_entity(slf.as_any())?.type_name().to_string())
    }

    /// Whether the entity is construction geometry.
    #[getter]
    fn construction(slf: &Bound<'_, Self>) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.base().construction)
    }

    #[setter]
    fn set_construction(slf: &Bound<'_, Self>, value: bool) -> PyResult<()> {
        with_entity_mut!(slf, |entity| {
            entity.base_mut().construction = value;
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// Whether the entity is hidden from output generation.
    #[getter]
    fn invisible(slf: &Bound<'_, Self>) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.base().invisible)
    }

    #[setter]
    fn set_invisible(slf: &Bound<'_, Self>, value: bool) -> PyResult<()> {
        with_entity_mut!(slf, |entity| {
            entity.base_mut().invisible = value;
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// Whether the entity is part of an array.
    #[getter]
    fn array_copy(slf: &Bound<'_, Self>) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.base().array_copy)
    }

    #[setter]
    fn set_array_copy(slf: &Bound<'_, Self>, value: bool) -> PyResult<()> {
        with_entity_mut!(slf, |entity| {
            entity.base_mut().array_copy = value;
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// Whether the entity's geometry is fully determined by the
    /// solver.
    #[getter]
    fn constrained(slf: &Bound<'_, Self>) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.base().constrained)
    }

    #[setter]
    fn set_constrained(slf: &Bound<'_, Self>, value: bool) -> PyResult<()> {
        with_entity_mut!(slf, |entity| {
            entity.base_mut().constrained = value;
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// The fill color, when one is set.
    #[getter]
    fn fill_color(
        slf: &Bound<'_, Self>,
    ) -> PyResult<Option<(f64, f64, f64, f64)>> {
        Ok(extract_entity(slf.as_any())?
            .base()
            .fill_color
            .map(|color| (color[0], color[1], color[2], color[3])))
    }

    #[setter]
    #[pyo3(name = "fill_color")]
    fn set_fill_color_attr(
        slf: &Bound<'_, Self>,
        value: Option<(f64, f64, f64, f64)>,
    ) -> PyResult<()> {
        with_entity_mut!(slf, |entity| {
            entity.base_mut().fill_color =
                value.map(|color| [color.0, color.1, color.2, color.3]);
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// The ids of all control points used by this entity.
    fn get_point_ids(slf: &Bound<'_, Self>) -> PyResult<Vec<u64>> {
        Ok(extract_entity(slf.as_any())?.get_point_ids())
    }

    /// The ids of the entity's endpoints.
    fn get_endpoint_ids(slf: &Bound<'_, Self>) -> PyResult<Vec<u64>> {
        Ok(extract_entity(slf.as_any())?.get_endpoint_ids())
    }

    /// The ids of the entity's junction points.
    fn get_junction_point_ids(slf: &Bound<'_, Self>) -> PyResult<Vec<u64>> {
        Ok(extract_entity(slf.as_any())?.get_junction_point_ids())
    }

    /// The ids of the entity's helper points.
    fn get_helper_ids(slf: &Bound<'_, Self>) -> PyResult<Vec<u64>> {
        Ok(extract_entity(slf.as_any())?.get_helper_ids())
    }

    /// The ids of points that may stay unconstrained.
    fn get_ignorable_unconstrained_points(
        slf: &Bound<'_, Self>,
    ) -> PyResult<Vec<u64>> {
        Ok(extract_entity(slf.as_any())?.get_ignorable_unconstrained_points())
    }

    /// The ids of points rigidly connected to the given point.
    fn get_rigidly_connected_points(
        slf: &Bound<'_, Self>,
        point_id: u64,
    ) -> PyResult<Vec<u64>> {
        Ok(
            extract_entity(slf.as_any())?
                .get_rigidly_connected_points(point_id),
        )
    }

    /// The ids of the entity's drag anchor points.
    fn get_drag_anchor_points(
        slf: &Bound<'_, Self>,
        point_id: u64,
    ) -> PyResult<Vec<u64>> {
        Ok(extract_entity(slf.as_any())?.get_drag_anchor_points(point_id))
    }

    /// The ids of all frame points, including text box frames.
    fn get_all_frame_point_ids(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<Vec<u64>> {
        Ok(extract_entity(slf.as_any())?.get_all_frame_point_ids(&registry))
    }

    /// Clears the entity's helper point references.
    fn clear_helper_references(slf: &Bound<'_, Self>) -> PyResult<()> {
        with_entity_mut_write!(slf, |entity| {
            entity.clear_helper_references();
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// Whether the entity supports fills.
    fn supports_fill(slf: &Bound<'_, Self>) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.supports_fill())
    }

    /// Whether the entity supports point-on-curve constraints.
    fn supports_point_on_curve(slf: &Bound<'_, Self>) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.supports_point_on_curve())
    }

    /// Whether the entity has a radius.
    fn is_radius_entity(slf: &Bound<'_, Self>) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.is_radius_entity())
    }

    /// Whether the entity alone forms a closed loop.
    fn is_closed_loop(slf: &Bound<'_, Self>) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.is_closed_loop())
    }

    /// Whether the entity is an edge entity.
    fn is_edge_entity(slf: &Bound<'_, Self>) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.is_edge_entity())
    }

    /// The entity's characteristic length.
    fn characteristic_length(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<f64> {
        Ok(extract_entity(slf.as_any())?.characteristic_length(&registry))
    }

    /// The entity's characteristic-length point pairs.
    fn characteristic_length_pairs(
        slf: &Bound<'_, Self>,
    ) -> PyResult<Vec<[u64; 2]>> {
        Ok(extract_entity(slf.as_any())?.characteristic_length_pairs())
    }

    /// The signed area the entity encloses on its own; raises
    /// `NotImplementedError` when unsupported.
    fn enclosed_signed_area(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<f64> {
        extract_entity(slf.as_any())?
            .enclosed_signed_area(&registry)
            .ok_or_else(|| {
                PyNotImplementedError::new_err(
                    "signed area not defined for this entity",
                )
            })
    }

    /// Serializes the entity to the reference dictionary shape.
    fn to_dict<'py>(
        slf: &Bound<'_, Self>,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let entity = extract_entity(slf.as_any())?;
        crate::python::sketch::constraints::dict_from_value(
            py,
            &entity.to_json(),
        )
    }

    /// Solver-relevant discrete state for undo/redo snapshots.
    fn get_state<'py>(
        slf: &Bound<'_, Self>,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        match extract_entity(slf.as_any())?.get_state() {
            Some(value) => {
                crate::python::sketch::constraints::dict_from_value(py, &value)
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    /// Restores solver-relevant discrete state from a snapshot.
    fn set_state(
        slf: &Bound<'_, Self>,
        state: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        let value = crate::python::sketch::registry::value_from_dict(state)?;
        with_entity_mut!(slf, |entity| {
            entity.set_state(&value);
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// Updates the constrained flag from defining-point status and
    /// constraints that dimension this entity's radius or length.
    fn update_constrained_status(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
        constraints: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        with_entity_mut!(slf, |entity| {
            let constrained = entity.compute_constrained(&registry, &|eid: u64| {
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
            entity.base_mut().constrained = constrained;
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// The geometry signature used for change detection.
    fn geometry_signature<'py>(
        slf: &Bound<'_, Self>,
        py: Python<'py>,
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<Bound<'py, pyo3::types::PyTuple>> {
        let entity = extract_entity(slf.as_any())?;
        signature_to_py(py, &entity.geometry_signature(&registry))
    }

    /// Returns the entity's toolpath geometry in model coordinates.
    fn to_geometry(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<crate::python::geo::geometry::Geometry> {
        let entity = extract_entity(slf.as_any())?;
        Ok(crate::python::geo::geometry::Geometry {
            inner: entity.to_geometry(&registry),
        })
    }

    /// Returns the entity's outline as polygon vertices.
    #[pyo3(signature = (registry, forward = true))]
    fn to_polygon_vertices(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
        forward: bool,
    ) -> PyResult<Vec<(f64, f64)>> {
        Ok(extract_entity(slf.as_any())?
            .to_polygon_vertices(&registry, forward))
    }

    /// Returns the entity's centerline as polyline vertices.
    #[pyo3(signature = (registry, tolerance = 0.1))]
    fn to_polyline(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
        tolerance: f64,
    ) -> PyResult<Vec<(f64, f64)>> {
        Ok(extract_entity(slf.as_any())?.to_polyline(&registry, tolerance))
    }

    /// Samples the entity into one or more polylines.
    #[pyo3(signature = (registry, tolerance = 0.1))]
    fn to_polylines(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
        tolerance: f64,
    ) -> PyResult<Vec<Vec<(f64, f64)>>> {
        Ok(extract_entity(slf.as_any())?.to_polylines(&registry, tolerance))
    }

    /// Appends the entity's outline to the geometry.
    #[pyo3(signature = (geo, registry, forward = true))]
    fn append_to_geometry(
        slf: &Bound<'_, Self>,
        geo: &mut crate::python::geo::geometry::Geometry,
        registry: crate::python::sketch::context::PointCtx<'_>,
        forward: bool,
    ) -> PyResult<()> {
        let entity = extract_entity(slf.as_any())?;
        entity.append_to_geometry(&mut geo.inner, &registry, forward);
        Ok(())
    }

    /// The fill geometry of the entity, if it supports fills.
    fn create_fill_geometry(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<Option<crate::python::geo::geometry::Geometry>> {
        let entity = extract_entity(slf.as_any())?;
        Ok(entity.create_fill_geometry(&registry).map(|geometry| {
            crate::python::geo::geometry::Geometry { inner: geometry }
        }))
    }

    /// Whether the entity is hit at model coordinates (mx, my).
    #[pyo3(signature = (mx, my, threshold, registry))]
    fn hit_test(
        slf: &Bound<'_, Self>,
        mx: f64,
        my: f64,
        threshold: f64,
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<bool> {
        Ok(
            extract_entity(slf.as_any())?
                .hit_test(mx, my, threshold, &registry),
        )
    }

    /// Whether the model-space point lies inside the entity.
    fn contains_point(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
        x: f64,
        y: f64,
    ) -> PyResult<bool> {
        Ok(extract_entity(slf.as_any())?.contains_point(&registry, x, y))
    }

    /// Whether the entity is fully contained by the given rect.
    fn is_contained_by(
        slf: &Bound<'_, Self>,
        rect: (f64, f64, f64, f64),
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<bool> {
        let rect = crate::geo::types::Rect {
            min: crate::geo::types::Point::new(rect.0, rect.1),
            max: crate::geo::types::Point::new(rect.2, rect.3),
        };
        Ok(extract_entity(slf.as_any())?.is_contained_by(rect, &registry))
    }

    /// Whether the entity intersects the given rect.
    fn intersects_rect(
        slf: &Bound<'_, Self>,
        rect: (f64, f64, f64, f64),
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<bool> {
        let rect = crate::geo::types::Rect {
            min: crate::geo::types::Point::new(rect.0, rect.1),
            max: crate::geo::types::Point::new(rect.2, rect.3),
        };
        Ok(extract_entity(slf.as_any())?.intersects_rect(rect, &registry))
    }

    /// Signed distance from the model-space point, if defined.
    fn signed_distance_to(
        slf: &Bound<'_, Self>,
        px: f64,
        py: f64,
        registry: crate::python::sketch::context::PointCtx<'_>,
    ) -> PyResult<Option<f64>> {
        Ok(extract_entity(slf.as_any())?.signed_distance_to(px, py, &registry))
    }

    /// Tangent direction at the point nearest the model-space
    /// point; raises `NotImplementedError` when the entity has no
    /// tangent.
    fn tangent_at(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
        point_id: u64,
    ) -> PyResult<(f64, f64)> {
        extract_entity(slf.as_any())?
            .tangent_at(&registry, point_id)
            .ok_or_else(|| {
                PyNotImplementedError::new_err(
                    "tangent not defined for this entity",
                )
            })
    }

    /// Sets the fill color; returns whether the entity supports
    /// fills.
    fn set_fill_color(
        slf: &Bound<'_, Self>,
        color: Option<[f64; 4]>,
    ) -> PyResult<bool> {
        with_entity_mut!(slf, |entity| {
            entity.set_fill_color(color)
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// Rewrites every point/entity id reference through *id_map*.
    fn remap_references(
        slf: &Bound<'_, Self>,
        id_map: HashMap<i64, i64>,
    ) -> PyResult<()> {
        let map = |value: u64| -> u64 {
            id_map
                .get(&(value as i64))
                .map(|new| *new as u64)
                .unwrap_or(value)
        };
        with_entity_mut_write!(slf, |entity| {
            entity.remap_references(&map);
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// Mirrors the entity across the reference axis object.
    fn mirror(slf: &Bound<'_, Self>, axis: &Bound<'_, PyAny>) -> PyResult<()> {
        let direction: String = axis.getattr("direction")?.extract()?;
        let position: f64 = axis.getattr("position")?.extract()?;
        let axis_direction = match direction.as_str() {
            "vertical" => {
                crate::sketch::entities::mirror::MirrorDirection::Vertical
            }
            "horizontal" => {
                crate::sketch::entities::mirror::MirrorDirection::Horizontal
            }
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown mirror direction: {other}"
                )))
            }
        };
        with_entity_mut_write!(slf, |entity| {
            entity.mirror(crate::sketch::entities::mirror::MirrorAxis {
                direction: axis_direction,
                position,
            });
        }; crate::python::sketch::entities::line::PyLine, crate::python::sketch::entities::arc::PyArc, crate::python::sketch::entities::circle::PyCircle, crate::python::sketch::entities::ellipse::PyEllipse, crate::python::sketch::entities::bezier::PyBezier, crate::python::sketch::entities::polygon::PyPolygon, crate::python::sketch::entities::text_box::PyTextBox)
    }

    /// Transforms entity-internal offset state for a placement.
    fn transform_offsets(
        slf: &Bound<'_, Self>,
        placement: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let any = slf.as_any();
        if let Ok(mut bezier) = any.extract::<PyRefMut<
            '_,
            crate::python::sketch::entities::bezier::PyBezier,
        >>() {
            let result = offset::bezier_transform_offsets(
                &mut bezier.inner,
                placement,
            );
            bezier.write_back();
            return result;
        }
        if let Ok(mut polygon) = any.extract::<PyRefMut<
            '_,
            crate::python::sketch::entities::polygon::PyPolygon,
        >>() {
            let result = offset::polygon_transform_offsets(
                &mut polygon.inner,
                placement,
            );
            polygon.write_back();
            return result;
        }
        Ok(())
    }

    /// Re-derives copy state from a template.
    fn rewrite_offsets_from(
        slf: &Bound<'_, Self>,
        template: &Bound<'_, PyAny>,
        placement: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let template_entity = extract_entity(template)?;
        let template_any = template_entity.as_any();
        let any = slf.as_any();
        if let Ok(mut bezier) = any.extract::<PyRefMut<
            '_,
            crate::python::sketch::entities::bezier::PyBezier,
        >>() {
            if let Some(template) = template_any
                .downcast_ref::<crate::sketch::entities::bezier::Bezier>()
            {
                let result = offset::bezier_rewrite_offsets_from(
                    &mut bezier.inner,
                    template,
                    placement,
                );
                bezier.write_back();
                return result;
            }
        }
        if let Ok(mut polygon) = any.extract::<PyRefMut<
            '_,
            crate::python::sketch::entities::polygon::PyPolygon,
        >>() {
            if let Some(template) = template_any
                .downcast_ref::<crate::sketch::entities::polygon::PolygonEntity>()
            {
                let result = offset::polygon_rewrite_offsets_from(
                    &mut polygon.inner,
                    template,
                    placement,
                );
                polygon.write_back();
                return result;
            }
        }
        if let Ok(mut text_box) =
            any.extract::<PyRefMut<
                '_,
                crate::python::sketch::entities::text_box::PyTextBox,
            >>()
        {
            if let Some(template) = template_any
                .downcast_ref::<crate::sketch::entities::text_box::TextBoxEntity>()
            {
                offset::text_box_rewrite_offsets_from(
                    &mut text_box.inner,
                    template,
                );
            }
        }
        Ok(())
    }

    /// The offsettable item this entity forms on its own; the
    /// reference default is None.
    fn as_offset_item<'py>(
        slf: &Bound<'py, Self>,
        py: Python<'py>,
        sketch: &Bound<'_, PyAny>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let registry = sketch
            .getattr("registry")
            .unwrap_or_else(|_| sketch.clone());
        offset::offset_item_for_entity(py, slf.as_any(), &registry)
    }

    /// Returns a pure description of the offset changes; the
    /// reference default is None (no analytic offset).
    fn plan_offset<'py>(
        slf: &Bound<'py, Self>,
        py: Python<'py>,
        registry: crate::python::sketch::context::PointCtx<'_>,
        offset: f64,
        _allocate_id: &Bound<'_, PyAny>,
    ) -> PyResult<Option<offset::PyOffsetPlan>> {
        let entity = extract_entity(slf.as_any())?;
        let any = entity.as_any();
        if let Some(circle) =
            any.downcast_ref::<crate::sketch::entities::circle::Circle>()
        {
            return offset::circle_plan(py, circle, &registry, offset);
        }
        if let Some(arc) =
            any.downcast_ref::<crate::sketch::entities::arc::Arc>()
        {
            return offset::arc_plan(py, arc, &registry, offset);
        }
        if let Some(ellipse) =
            any.downcast_ref::<crate::sketch::entities::ellipse::Ellipse>()
        {
            return offset::ellipse_plan(py, ellipse, &registry, offset);
        }
        if let Some(polygon) = any
            .downcast_ref::<crate::sketch::entities::polygon::PolygonEntity>(
        ) {
            return offset::polygon_plan(
                py,
                polygon,
                &registry,
                offset,
                _allocate_id,
            );
        }
        Ok(None)
    }

    /// Coarse polylines of the entity's offset for live preview;
    /// the reference default is no preview.
    fn preview_polylines(
        slf: &Bound<'_, Self>,
        registry: crate::python::sketch::context::PointCtx<'_>,
        offset: f64,
    ) -> PyResult<Vec<Vec<(f64, f64)>>> {
        let entity = extract_entity(slf.as_any())?;
        let any = entity.as_any();
        if let Some(circle) =
            any.downcast_ref::<crate::sketch::entities::circle::Circle>()
        {
            return Ok(offset::circle_preview(circle, &registry, offset));
        }
        if let Some(arc) =
            any.downcast_ref::<crate::sketch::entities::arc::Arc>()
        {
            return Ok(offset::arc_preview(arc, &registry, offset));
        }
        if let Some(ellipse) =
            any.downcast_ref::<crate::sketch::entities::ellipse::Ellipse>()
        {
            return Ok(offset::ellipse_preview(ellipse, &registry, offset));
        }
        if let Some(polygon) = any
            .downcast_ref::<crate::sketch::entities::polygon::PolygonEntity>(
        ) {
            return Ok(offset::polygon_preview(polygon, &registry, offset));
        }
        Ok(Vec::new())
    }

    /// Deep-copies the entity through its reference dictionary
    /// shape, rebuilding via the concrete class's `from_dict`.
    fn __deepcopy__<'py>(
        slf: &Bound<'py, Self>,
        memo: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _ = memo;
        let dict = slf.call_method0("to_dict")?;
        let from_dict = slf.get_type().getattr("from_dict")?;
        from_dict.call1((dict,))
    }

    fn __copy__<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        Self::__deepcopy__(slf, &PyDict::new(slf.py()))
    }
}

/// The base of all entity classes, for typed registry returns.
pub struct AnyEntity<'py>(pub Bound<'py, PyAny>);

impl<'a, 'py> FromPyObject<'a, 'py> for AnyEntity<'py> {
    type Error = PyErr;

    fn extract(ob: pyo3::Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        Ok(AnyEntity(ob.to_owned()))
    }
}

impl<'py> IntoPyObject<'py> for AnyEntity<'py> {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(
        self,
        _py: Python<'py>,
    ) -> Result<Self::Output, Self::Error> {
        Ok(self.0)
    }
}

impl PyStubType for AnyEntity<'_> {
    fn type_output() -> TypeInfo {
        let module = ModuleRef::Named("raygeo.sketch.entities".to_string());
        TypeInfo {
            name: "Entity".to_string(),
            source_module: None,
            import: HashSet::from([ImportRef::Module(module.clone())]),
            type_refs: HashMap::from([(
                "Entity".to_string(),
                pyo3_stub_gen::TypeIdentifierRef {
                    module,
                    import_kind: ImportKind::Module,
                },
            )]),
        }
    }
}

/// A sketch point view: coordinates and solver flags.
#[allow(dead_code)]
pub struct AnyPoint<'py>(pub Bound<'py, PyAny>);

pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.entities" from "raygeo.sketch.entities.point";
    "SketchPoint"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.entities" from "raygeo.sketch.entities.line";
    "Line"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.entities" from "raygeo.sketch.entities.arc";
    "Arc"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.entities" from "raygeo.sketch.entities.circle";
    "Circle"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.entities" from "raygeo.sketch.entities.ellipse";
    "Ellipse"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.entities" from "raygeo.sketch.entities.bezier";
    "Bezier"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.entities" from "raygeo.sketch.entities.polygon";
    "Polygon"
);
pyo3_stub_gen::reexport_module_members!(
    "raygeo.sketch.entities" from "raygeo.sketch.entities.text_box";
    "TextBox"
);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let entities_mod = PyModule::new(py, "entities")?;

    entities_mod.add_class::<PyEntityBase>()?;
    offset::register(&entities_mod)?;
    entities_mod.add_function(wrap_pyfunction!(quantize, &entities_mod)?)?;

    point::register(&entities_mod)?;
    line::register(&entities_mod)?;
    arc::register(&entities_mod)?;
    circle::register(&entities_mod)?;
    ellipse::register(&entities_mod)?;
    bezier::register(&entities_mod)?;
    polygon::register(&entities_mod)?;
    text_box::register(&entities_mod)?;

    m.add_submodule(&entities_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities", &entities_mod)?;
    Ok(())
}
