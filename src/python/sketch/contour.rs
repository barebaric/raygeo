//! Python bindings for contour preparation helpers.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::python::geo::geometry::Geometry;
use crate::python::sketch::registry::PyEntityRegistry;

pyo3_stub_gen::module_doc!("raygeo.sketch.contour", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Contour preparation: connected components, chain orientation, sampled
chain vertices, ring grouping and outline offsetting for offset and
boolean tools.";

#[gen_stub_pyfunction(module = "raygeo.sketch.contour")]
#[pyfunction]
fn partition_components(
    registry: PyRef<'_, PyEntityRegistry>,
    entity_ids: Vec<u64>,
    coincident_pairs: Vec<(u64, u64)>,
) -> Option<Vec<Vec<u64>>> {
    crate::sketch::contour::partition_components(
        &registry.inner,
        &entity_ids,
        &coincident_pairs,
    )
}

#[gen_stub_pyfunction(module = "raygeo.sketch.contour")]
#[pyfunction]
#[pyo3(signature = (registry, entity_ids, coincident_pairs = Vec::new()))]
fn walk_component(
    registry: PyRef<'_, PyEntityRegistry>,
    entity_ids: Vec<u64>,
    coincident_pairs: Vec<(u64, u64)>,
) -> (Vec<(u64, bool)>, bool) {
    crate::sketch::contour::walk_component(
        &registry.inner,
        &entity_ids,
        &coincident_pairs,
    )
}

#[gen_stub_pyfunction(module = "raygeo.sketch.contour")]
#[pyfunction]
#[pyo3(signature = (registry, edges, closed, tolerance = 0.02))]
fn chain_vertices(
    registry: PyRef<'_, PyEntityRegistry>,
    edges: Vec<(u64, bool)>,
    closed: bool,
    tolerance: f64,
) -> Vec<(f64, f64)> {
    crate::sketch::contour::chain_vertices(
        &registry.inner,
        &edges,
        closed,
        tolerance,
    )
}

#[gen_stub_pyfunction(module = "raygeo.sketch.contour")]
#[pyfunction]
fn outline_frame(
    vertices: Vec<(f64, f64)>,
    closed: bool,
) -> ((f64, f64), (f64, f64)) {
    crate::sketch::contour::outline_frame(&vertices, closed)
}

#[gen_stub_pyfunction(module = "raygeo.sketch.contour")]
#[pyfunction]
fn rings_to_geometry(rings: Vec<Vec<(f64, f64)>>) -> Geometry {
    Geometry {
        inner: crate::sketch::contour::rings_to_geometry(&rings),
    }
}

#[gen_stub_pyfunction(module = "raygeo.sketch.contour")]
#[pyfunction]
fn group_rings_into_solids(
    rings: Vec<Vec<(f64, f64)>>,
) -> Vec<crate::sketch::contour::Solid> {
    crate::sketch::contour::group_rings_into_solids(&rings)
}

#[gen_stub_pyfunction(module = "raygeo.sketch.contour")]
#[pyfunction]
#[pyo3(signature = (vertices, closed, distance, tolerance = 0.02))]
fn offset_outline(
    vertices: Vec<(f64, f64)>,
    closed: bool,
    distance: f64,
    tolerance: f64,
) -> Vec<(Vec<(f64, f64)>, bool)> {
    crate::python::sketch::entities::offset::offset_outline(
        &vertices, closed, distance, tolerance,
    )
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let contour_mod = PyModule::new(py, "contour")?;
    contour_mod.setattr("__doc__", MODULE_DOC)?;
    contour_mod
        .add_function(wrap_pyfunction!(partition_components, &contour_mod)?)?;
    contour_mod
        .add_function(wrap_pyfunction!(walk_component, &contour_mod)?)?;
    contour_mod
        .add_function(wrap_pyfunction!(chain_vertices, &contour_mod)?)?;
    contour_mod.add_function(wrap_pyfunction!(outline_frame, &contour_mod)?)?;
    contour_mod
        .add_function(wrap_pyfunction!(rings_to_geometry, &contour_mod)?)?;
    contour_mod.add_function(wrap_pyfunction!(
        group_rings_into_solids,
        &contour_mod
    )?)?;
    contour_mod
        .add_function(wrap_pyfunction!(offset_outline, &contour_mod)?)?;
    m.add_submodule(&contour_mod)?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.contour", &contour_mod)?;
    Ok(())
}
