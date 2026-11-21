//! Snap points at line and arc midpoints.

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
    name = "MidpointsProducer"
)]
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
        let context = drag_context.to_context();
        let mut points = Vec::new();
        for point in core::midpoints::produce_points(
            &reg.inner,
            &context,
            drag_position,
            threshold,
        ) {
            let source = match point.source {
                Some(id) => registry
                    .call_method1("get_entity_object", (id,))?
                    .extract::<Option<Py<PyAny>>>()?,
                None => None,
            };
            points.push(PySnapPoint::from_core(py, point, source)?);
        }
        Ok(points)
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyMidpointsProducer>()?;
    Ok(())
}
