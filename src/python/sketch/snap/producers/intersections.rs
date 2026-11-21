//! Snap points at entity intersections.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::snap::types::{
    PyDragContext, PySnapLine, PySnapPoint,
};
use crate::sketch::snap::fingerprint::geometry_fingerprint;
use crate::sketch::snap::producers as core;

#[gen_stub_pyclass]
#[pyclass(
    dict,
    skip_from_py_object,
    module = "raygeo.sketch.snap.producers",
    name = "IntersectionsProducer"
)]
pub struct PyIntersectionsProducer {
    include_construction: bool,
    cache_key: Option<String>,
    cached: Vec<(f64, f64)>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyIntersectionsProducer {
    #[new]
    #[pyo3(signature = (include_construction=true))]
    fn new(include_construction: bool) -> Self {
        Self {
            include_construction,
            cache_key: None,
            cached: Vec::new(),
        }
    }

    #[getter]
    fn _include_construction(&self) -> bool {
        self.include_construction
    }

    #[getter]
    fn _cache_key(&self) -> Option<String> {
        self.cache_key.clone()
    }

    #[getter]
    fn _cached_intersections(&self) -> Vec<(f64, f64)> {
        self.cached.clone()
    }

    fn _get_all_intersections(
        &self,
        registry: &Bound<'_, PyEntityRegistry>,
        drag_context: &PyDragContext,
    ) -> Vec<(f64, f64)> {
        let reg = registry.borrow();
        let context = drag_context.to_context();
        core::intersections::all_intersections(
            &reg.inner,
            &context,
            self.include_construction,
        )
    }

    fn produce(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        registry: &Bound<'_, PyEntityRegistry>,
        _drag_position: (f64, f64),
        drag_context: &Bound<'_, PyAny>,
        _threshold: f64,
    ) -> PyResult<Vec<Py<PySnapLine>>> {
        let intersections = get_intersections(slf, registry, drag_context)?;
        let mut lines = Vec::new();
        for (ix, iy) in intersections {
            for (is_horizontal, coordinate) in [(false, ix), (true, iy)] {
                lines.push(PySnapLine::from_core(
                    py,
                    crate::sketch::snap::types::SnapLine {
                        is_horizontal,
                        coordinate,
                        kind: crate::sketch::snap::types::SnapLineType::
                            INTERSECTION,
                        source: None,
                    },
                    None,
                )?);
            }
        }
        Ok(lines)
    }

    fn produce_points(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        registry: &Bound<'_, PyEntityRegistry>,
        drag_position: (f64, f64),
        drag_context: &Bound<'_, PyAny>,
        threshold: f64,
    ) -> PyResult<Vec<Py<PySnapPoint>>> {
        let (x, y) = drag_position;
        let intersections = get_intersections(slf, registry, drag_context)?;
        let mut points = Vec::new();
        for (ix, iy) in intersections {
            let dist = ((ix - x).powi(2) + (iy - y).powi(2)).sqrt();
            if dist <= threshold {
                points.push(PySnapPoint::from_core(
                    py,
                    crate::sketch::snap::types::SnapPoint {
                        x: ix,
                        y: iy,
                        kind: crate::sketch::snap::types::SnapLineType::
                            INTERSECTION,
                        source: None,
                        spacing: None,
                        is_horizontal: false,
                        pattern_coords: None,
                        axis_coord: None,
                    },
                    None,
                )?);
            }
        }
        Ok(points)
    }
}

fn get_intersections(
    slf: &Bound<'_, PyIntersectionsProducer>,
    registry: &Bound<'_, PyEntityRegistry>,
    context_obj: &Bound<'_, PyAny>,
) -> PyResult<Vec<(f64, f64)>> {
    let context = context_obj.extract::<PyRef<'_, PyDragContext>>()?;
    let key = {
        let this = slf.borrow();
        let reg = registry.borrow();
        geometry_fingerprint(
            &reg.inner,
            Some(&context.to_context()),
            this.include_construction,
            false,
        )
    };
    {
        let this = slf.borrow();
        if this.cache_key.as_deref() == Some(key.as_str()) {
            return Ok(this.cached.clone());
        }
    }
    let dict = slf.getattr("__dict__")?;
    let dict = dict.cast::<pyo3::types::PyDict>()?;
    let computed: Vec<(f64, f64)> = match dict
        .get_item("_get_all_intersections")?
    {
        Some(callback) => callback.call1((registry, context_obj))?.extract()?,
        None => {
            let this = slf.borrow();
            let reg = registry.borrow();
            let context = context.to_context();
            core::intersections::all_intersections(
                &reg.inner,
                &context,
                this.include_construction,
            )
        }
    };
    let mut this = slf.borrow_mut();
    this.cached = computed.clone();
    this.cache_key = Some(key);
    Ok(computed)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyIntersectionsProducer>()?;
    Ok(())
}
