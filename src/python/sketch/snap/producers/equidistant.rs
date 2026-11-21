//! Snap points at equally spaced positions.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::snap::types::{
    PyDragContext, PySnapLine, PySnapPoint,
};
use crate::sketch::snap::producers::{self as core, EquidistantParams};

#[gen_stub_pyclass]
#[pyclass(
    skip_from_py_object,
    module = "raygeo.sketch.snap.producers",
    name = "EquidistantLinesProducer"
)]
pub struct PyEquidistantLinesProducer {
    params: EquidistantParams,
    include_construction: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEquidistantLinesProducer {
    #[new]
    #[pyo3(signature = (spacing_tolerance=0.5, max_spacing=100.0, include_construction=true))]
    fn new(
        spacing_tolerance: f64,
        max_spacing: f64,
        include_construction: bool,
    ) -> Self {
        Self {
            params: EquidistantParams {
                spacing_tolerance,
                max_spacing,
            },
            include_construction,
        }
    }

    #[getter]
    fn _spacing_tolerance(&self) -> f64 {
        self.params.spacing_tolerance
    }

    #[getter]
    fn _max_spacing(&self) -> f64 {
        self.params.max_spacing
    }

    #[getter]
    fn _include_construction(&self) -> bool {
        self.include_construction
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
        for point in core::equidistant::produce_points(
            &reg.inner,
            &context,
            &self.params,
            drag_position,
            threshold,
        ) {
            points.push(PySnapPoint::from_core(py, point, None)?);
        }
        Ok(points)
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyEquidistantLinesProducer>()?;
    Ok(())
}
