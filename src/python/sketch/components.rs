//! Python bindings for sketch constraint components.

use std::collections::BTreeSet;

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::components::compute_constraint_components as core_components;
use crate::sketch::constraint::SketchConstraint;

use super::solver::constraint_from_py;

/// Returns all point ids a constraint couples: the points it
/// references directly plus the points of the entities it references.
#[gen_stub_pyfunction(module = "raygeo.sketch.components")]
#[pyfunction]
fn get_referenced_points(
    registry: &PyEntityRegistry,
    constraint: &Bound<'_, PyAny>,
) -> PyResult<Vec<u64>> {
    referenced_points_of(registry, constraint)
}

/// The point ids a constraint couples, callable from Rust so
/// `PySketch::solve` needs no Python round-trip.
pub(crate) fn referenced_points_of(
    registry: &PyEntityRegistry,
    constraint: &Bound<'_, PyAny>,
) -> PyResult<Vec<u64>> {
    let constraint = constraint_from_py(constraint)?;
    Ok(constraint.referenced_points(&registry.inner))
}

/// Partitions all points referenced by constraints into connected
/// components of the constraint graph.
#[gen_stub_pyfunction(module = "raygeo.sketch.components")]
#[pyfunction]
fn compute_constraint_components(
    registry: &PyEntityRegistry,
    constraints: Vec<Bound<'_, PyAny>>,
) -> PyResult<Vec<BTreeSet<u64>>> {
    let parsed: Vec<Box<dyn SketchConstraint>> = constraints
        .iter()
        .map(constraint_from_py)
        .collect::<PyResult<_>>()?;
    let refs: Vec<&dyn SketchConstraint> = parsed
        .iter()
        .map(|constraint| constraint.as_ref())
        .collect();
    Ok(core_components(&registry.inner, &refs))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let components_mod = PyModule::new(py, "components")?;

    components_mod
        .setattr("__doc__", "Connected components of the constraint graph.")?;

    components_mod.add_function(wrap_pyfunction!(
        get_referenced_points,
        &components_mod
    )?)?;
    components_mod.add_function(wrap_pyfunction!(
        compute_constraint_components,
        &components_mod
    )?)?;

    m.add_submodule(&components_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.components", &components_mod)?;

    Ok(())
}
