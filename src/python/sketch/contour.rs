//! Python bindings for contour preparation helpers.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::python::geo::geometry::Geometry;
use crate::python::sketch::entities::offset::{
    offset_item_for_entity, PyPolygonOutline,
};
use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::solver::constraint_from_py;
use crate::python::sketch::types::EntityId;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::coincident::pairs_for_entities;

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

#[gen_stub_pyfunction(module = "raygeo.sketch.contour")]
#[pyfunction]
#[pyo3(signature = (vertices, closed, allocate_id, holes = None))]
fn build_outline_item<'py>(
    py: Python<'py>,
    vertices: Vec<(f64, f64)>,
    closed: bool,
    allocate_id: &Bound<'py, PyAny>,
    holes: Option<Vec<Vec<(f64, f64)>>>,
) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>, Bound<'py, PyAny>)> {
    crate::python::sketch::entities::offset::outline_item(
        py,
        &vertices,
        closed,
        allocate_id,
        &holes.unwrap_or_default(),
    )
}

/// Partitions a selection into offsettable items.
///
/// Chainable entities are joined through coincident constraints and
/// sampled into outlines; lone entities keep their own offset behavior.
#[gen_stub_pyfunction(module = "raygeo.sketch.contour")]
#[pyfunction]
#[pyo3(signature = (registry, entity_ids, constraints, tolerance = 0.02))]
fn build_offset_items<'py>(
    py: Python<'py>,
    registry: &Bound<'py, PyEntityRegistry>,
    entity_ids: Vec<u64>,
    constraints: Vec<Bound<'py, PyAny>>,
    tolerance: f64,
) -> PyResult<Option<Vec<Py<PyAny>>>> {
    let parsed: Vec<Box<dyn SketchConstraint>> = constraints
        .iter()
        .map(constraint_from_py)
        .collect::<PyResult<_>>()?;

    let mut standalone = Vec::new();
    let mut chainable = Vec::new();
    {
        let reg = registry.borrow();
        for &entity_id in &entity_ids {
            let Some(entity) = reg.inner.get_entity(entity_id) else {
                continue;
            };
            if entity.get_endpoint_ids().len() == 2 {
                chainable.push(entity_id);
            } else {
                standalone.push(entity_id);
            }
        }
    }

    let mut items: Vec<Py<PyAny>> = Vec::new();
    for entity_id in standalone {
        let entity =
            registry.call_method1("get_entity", (EntityId(entity_id),))?;
        if entity.is_none() {
            continue;
        }
        if let Some(item) =
            offset_item_for_entity(py, &entity, registry.as_any())?
        {
            items.push(item.unbind());
        }
    }

    if !chainable.is_empty() {
        let (pairs, components) = {
            let reg = registry.borrow();
            let pairs = pairs_for_entities(&reg.inner, &chainable, &parsed);
            let components = crate::sketch::contour::partition_components(
                &reg.inner, &chainable, &pairs,
            );
            (pairs, components)
        };
        let Some(components) = components else {
            return Ok(None);
        };
        for component in components {
            if component.len() == 1 {
                let entity = registry
                    .call_method1("get_entity", (EntityId(component[0]),))?;
                if !entity.is_none() {
                    if let Some(item) =
                        offset_item_for_entity(py, &entity, registry.as_any())?
                    {
                        items.push(item.unbind());
                        continue;
                    }
                }
            }
            let (edges, closed, vertices) = {
                let reg = registry.borrow();
                let (edges, closed) = crate::sketch::contour::walk_component(
                    &reg.inner, &component, &pairs,
                );
                let vertices = crate::sketch::contour::chain_vertices(
                    &reg.inner, &edges, closed, tolerance,
                );
                (edges, closed, vertices)
            };
            if vertices.len() >= 2 {
                let outline = Py::new(
                    py,
                    PyPolygonOutline {
                        vertices,
                        closed,
                        source_ids: edges.iter().map(|(id, _)| *id).collect(),
                    },
                )?;
                items.push(outline.into_any());
            }
        }
    }

    Ok(if items.is_empty() { None } else { Some(items) })
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
    contour_mod
        .add_function(wrap_pyfunction!(build_outline_item, &contour_mod)?)?;
    contour_mod
        .add_function(wrap_pyfunction!(build_offset_items, &contour_mod)?)?;
    m.add_submodule(&contour_mod)?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.contour", &contour_mod)?;
    Ok(())
}
