//! Snap lines and points for every sketch point.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::snap::types::{
    PyDragContext, PySnapLine, PySnapPoint,
};
use crate::sketch::snap::producers as core;

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
        let context = drag_context.to_context();
        let mut lines = Vec::new();
        for line in core::entity_points::produce_lines(&reg.inner, &context) {
            let source = match line.source {
                Some(id) => registry
                    .call_method1("get_point_object", (id,))?
                    .extract::<Option<Py<PyAny>>>()?,
                None => None,
            };
            lines.push(PySnapLine::from_core(py, line, source)?);
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
        let context = drag_context.to_context();
        let mut points = Vec::new();
        for point in core::entity_points::produce_points(
            &reg.inner,
            &context,
            drag_position,
            threshold,
        ) {
            let source = match point.source {
                Some(id) => registry
                    .call_method1("get_point_object", (id,))?
                    .extract::<Option<Py<PyAny>>>()?,
                None => None,
            };
            points.push(PySnapPoint::from_core(py, point, source)?);
        }
        Ok(points)
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyEntityPointsProducer>()?;
    Ok(())
}
