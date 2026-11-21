//! Snap lines and points for every sketch point.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::snap::types::{
    PyDragContext, PySnapLine, PySnapLineType, PySnapPoint,
};

#[gen_stub_pyclass]
#[pyclass(
    skip_from_py_object,
    module = "raygeo.sketch.snap.producers",
    name = "EntityPointsProducer"
)]
#[derive(Default)]
pub struct PyEntityPointsProducer;

#[gen_stub_pymethods]
#[pymethods]
impl PyEntityPointsProducer {
    #[new]
    fn new() -> Self {
        Self
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
        for point_id in reg.inner.point_ids() {
            if drag_context.is_point_dragged(point_id) {
                continue;
            }
            let Some(point) = reg.inner.get_point(point_id) else {
                continue;
            };
            if drag_context.suppress_coincident_points
                && drag_context.point_ids().iter().any(|pid| {
                    reg.inner.get_point(*pid).is_some_and(|dragged| {
                        (dragged.x - point.x).abs() <= 1e-6
                            && (dragged.y - point.y).abs() <= 1e-6
                    })
                })
            {
                continue;
            }
            let source = registry
                .call_method1("get_point_object", (point_id,))?
                .extract::<Option<Py<PyAny>>>()?;
            lines.push(Py::new(
                py,
                PySnapLine {
                    is_horizontal: false,
                    coordinate: point.x,
                    line_type: PySnapLineType(
                        PySnapLineType::ENTITY_POINT_VALUE,
                    ),
                    source: source.clone(),
                },
            )?);
            lines.push(Py::new(
                py,
                PySnapLine {
                    is_horizontal: true,
                    coordinate: point.y,
                    line_type: PySnapLineType(
                        PySnapLineType::ENTITY_POINT_VALUE,
                    ),
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
        for point_id in reg.inner.point_ids() {
            if drag_context.is_point_dragged(point_id) {
                continue;
            }
            let Some(point) = reg.inner.get_point(point_id) else {
                continue;
            };
            if drag_context.suppress_coincident_points
                && drag_context.point_ids().iter().any(|pid| {
                    reg.inner.get_point(*pid).is_some_and(|dragged| {
                        (dragged.x - point.x).abs() <= 1e-6
                            && (dragged.y - point.y).abs() <= 1e-6
                    })
                })
            {
                continue;
            }
            let dist = ((point.x - x).powi(2) + (point.y - y).powi(2)).sqrt();
            if dist <= threshold {
                let source = registry
                    .call_method1("get_point_object", (point_id,))?
                    .extract::<Option<Py<PyAny>>>()?;
                points.push(Py::new(
                    py,
                    PySnapPoint {
                        x: point.x,
                        y: point.y,
                        line_type: PySnapLineType(
                            PySnapLineType::ENTITY_POINT_VALUE,
                        ),
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
    module.add_class::<PyEntityPointsProducer>()?;
    Ok(())
}
