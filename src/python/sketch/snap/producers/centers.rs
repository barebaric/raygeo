//! Snap lines and points at circle, arc, and ellipse centers.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::snap::types::{
    PyDragContext, PySnapLine, PySnapLineType, PySnapPoint,
};
use crate::sketch::entities::{arc::Arc, circle::Circle, ellipse::Ellipse};

#[gen_stub_pyclass]
#[pyclass(
    skip_from_py_object,
    module = "raygeo.sketch.snap.producers",
    name = "CentersProducer"
)]
pub struct PyCentersProducer {
    include_construction: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCentersProducer {
    #[new]
    #[pyo3(signature = (include_construction=true))]
    fn new(include_construction: bool) -> Self {
        Self {
            include_construction,
        }
    }

    #[getter]
    fn _include_construction(&self) -> bool {
        self.include_construction
    }

    fn produce(
        &self,
        py: Python<'_>,
        registry: &Bound<'_, PyEntityRegistry>,
        _drag_position: (f64, f64),
        drag_context: &PyDragContext,
        _threshold: f64,
    ) -> PyResult<Vec<Py<PySnapLine>>> {
        let reg = registry.borrow();
        let mut lines = Vec::new();
        for entity_id in reg.inner.entity_ids() {
            if drag_context.is_entity_dragged(entity_id) {
                continue;
            }
            let Some(entity) = reg.inner.get_entity(entity_id) else {
                continue;
            };
            if !self.include_construction && entity.base().construction {
                continue;
            }
            let center_id = entity
                .as_any()
                .downcast_ref::<Arc>()
                .map(|arc| arc.center)
                .or_else(|| {
                    entity
                        .as_any()
                        .downcast_ref::<Circle>()
                        .map(|circle| circle.center)
                })
                .or_else(|| {
                    entity
                        .as_any()
                        .downcast_ref::<Ellipse>()
                        .map(|ellipse| ellipse.center)
                });
            let Some(center_id) = center_id else {
                continue;
            };
            if drag_context.is_point_dragged(center_id) {
                continue;
            }
            let Some(center) = reg.inner.get_point(center_id) else {
                continue;
            };
            if drag_context.suppress_coincident_points
                && drag_context.point_ids().iter().any(|pid| {
                    reg.inner.get_point(*pid).is_some_and(|dragged| {
                        (dragged.x - center.x).abs() <= 1e-6
                            && (dragged.y - center.y).abs() <= 1e-6
                    })
                })
            {
                continue;
            }
            let source = registry
                .call_method1("get_entity_object", (entity_id,))?
                .extract::<Option<Py<PyAny>>>()?;
            lines.push(Py::new(
                py,
                PySnapLine {
                    is_horizontal: false,
                    coordinate: center.x,
                    line_type: PySnapLineType(PySnapLineType::CENTER_VALUE),
                    source: source.clone(),
                },
            )?);
            lines.push(Py::new(
                py,
                PySnapLine {
                    is_horizontal: true,
                    coordinate: center.y,
                    line_type: PySnapLineType(PySnapLineType::CENTER_VALUE),
                    source,
                },
            )?);
        }
        Ok(lines)
    }

    fn produce_points(
        &self,
        py: Python<'_>,
        registry: &Bound<'_, PyEntityRegistry>,
        drag_position: (f64, f64),
        drag_context: &PyDragContext,
        threshold: f64,
    ) -> PyResult<Vec<Py<PySnapPoint>>> {
        let reg = registry.borrow();
        let (x, y) = drag_position;
        let mut points = Vec::new();
        for entity_id in reg.inner.entity_ids() {
            if drag_context.is_entity_dragged(entity_id) {
                continue;
            }
            let Some(entity) = reg.inner.get_entity(entity_id) else {
                continue;
            };
            if !self.include_construction && entity.base().construction {
                continue;
            }
            let center_id = entity
                .as_any()
                .downcast_ref::<Arc>()
                .map(|arc| arc.center)
                .or_else(|| {
                    entity
                        .as_any()
                        .downcast_ref::<Circle>()
                        .map(|circle| circle.center)
                })
                .or_else(|| {
                    entity
                        .as_any()
                        .downcast_ref::<Ellipse>()
                        .map(|ellipse| ellipse.center)
                });
            let Some(center_id) = center_id else {
                continue;
            };
            if drag_context.is_point_dragged(center_id) {
                continue;
            }
            let Some(center) = reg.inner.get_point(center_id) else {
                continue;
            };
            if drag_context.suppress_coincident_points
                && drag_context.point_ids().iter().any(|pid| {
                    reg.inner.get_point(*pid).is_some_and(|dragged| {
                        (dragged.x - center.x).abs() <= 1e-6
                            && (dragged.y - center.y).abs() <= 1e-6
                    })
                })
            {
                continue;
            }
            let dist = ((center.x - x).powi(2) + (center.y - y).powi(2)).sqrt();
            if dist <= threshold {
                let source = registry
                    .call_method1("get_entity_object", (entity_id,))?
                    .extract::<Option<Py<PyAny>>>()?;
                points.push(Py::new(
                    py,
                    PySnapPoint {
                        x: center.x,
                        y: center.y,
                        line_type: PySnapLineType(PySnapLineType::CENTER_VALUE),
                        source,
                        spacing: None,
                        is_horizontal: false,
                        pattern_coords: None,
                        axis_coord: None,
                    },
                )?);
            }
        }
        Ok(points)
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyCentersProducer>()?;
    Ok(())
}
