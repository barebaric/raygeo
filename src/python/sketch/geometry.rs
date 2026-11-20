//! Python bindings for sketch geometry emission.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::python::geo::geometry::Geometry as PyGeometry;
use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::solver::constraint_from_py;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::geometry::document_geometry as core_document_geometry;

/// Converts a sketch registry into geometry, linking chainable entities
/// into continuous paths.
#[gen_stub_pyfunction(module = "raygeo.sketch.geometry")]
#[pyfunction]
#[pyo3(signature = (registry, constraints, resolved_text = None))]
fn document_geometry(
    registry: &PyEntityRegistry,
    constraints: Vec<Bound<'_, PyAny>>,
    resolved_text: Option<HashMap<u64, Option<String>>>,
) -> PyResult<PyGeometry> {
    let parsed: Vec<Box<dyn SketchConstraint>> = constraints
        .iter()
        .map(constraint_from_py)
        .collect::<PyResult<_>>()?;
    let inner = core_document_geometry(
        &registry.inner,
        &parsed,
        &resolved_text.unwrap_or_default(),
    );
    Ok(PyGeometry { inner })
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let geometry_mod = PyModule::new(py, "geometry")?;

    geometry_mod
        .setattr("__doc__", "Geometry emission for sketch documents.")?;
    geometry_mod
        .add_function(wrap_pyfunction!(document_geometry, &geometry_mod)?)?;

    m.add_submodule(&geometry_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.geometry", &geometry_mod)?;

    Ok(())
}
