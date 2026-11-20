//! Snap points at line and arc midpoints.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::snap::types::{
    PyDragContext, PySnapLine, PySnapLineType, PySnapPoint,
};
use crate::sketch::entities::{arc::Arc, line::Line};

#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.sketch.snap.producers", name = "MidpointsProducer")]
#[derive(Default)]
pub struct PyMidpointsProducer;

#[gen_stub_pymethods]
#[pymethods]
impl PyMidpointsProducer {
    #[new]
    fn new() -> Self {
        Self
    }

    fn produce(
        &self,
        _py: Python<'_>,
        _registry: &Bound<'_, PyEntityRegistry>,
        _drag_position: (f64, f64),
        _drag_context: &PyDragContext,
        _threshold: f64,
    ) -> Vec<Py<PySnapLine>> {
        Vec::new()
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
            if entity
                .get_point_ids()
                .iter()
                .any(|pid| drag_context.is_point_dragged(*pid))
            {
                continue;
            }
            let midpoint = if let Some(line) = entity.as_any().downcast_ref::<Line>() {
                let p1 = reg.inner.get_point(line.p1);
                let p2 = reg.inner.get_point(line.p2);
                match (p1, p2) {
                    (Some(p1), Some(p2)) => {
                        Some(((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0))
                    }
                    _ => None,
                }
            } else if let Some(arc) = entity.as_any().downcast_ref::<Arc>() {
                arc.get_midpoint(&reg.inner)
            } else {
                None
            };
            let Some((mx, my)) = midpoint else {
                continue;
            };
            let dist = ((mx - x).powi(2) + (my - y).powi(2)).sqrt();
            if dist <= threshold {
                let source = registry
                    .call_method1("get_entity_object", (entity_id,))?
                    .extract::<Option<Py<PyAny>>>()?;
                points.push(Py::new(
                    py,
                    PySnapPoint {
                        x: mx,
                        y: my,
                        line_type: PySnapLineType(PySnapLineType::MIDPOINT_VALUE),
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
    module.add_class::<PyMidpointsProducer>()?;
    Ok(())
}
