//! Built-in snap line and point producers.

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) mod centers;
pub(crate) mod entity_points;
pub(crate) mod equidistant;
pub(crate) mod intersections;
pub(crate) mod midpoints;
pub(crate) mod on_entity;

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    centers::register(module)?;
    entity_points::register(module)?;
    equidistant::register(module)?;
    intersections::register(module)?;
    midpoints::register(module)?;
    on_entity::register(module)?;

    let py = module.py();
    let spec_cls = py
        .import("importlib.machinery")?
        .getattr("ModuleSpec")?;
    let kwargs = pyo3::types::PyDict::new(py);
    kwargs.set_item("is_package", true)?;
    let package_spec = spec_cls.call(
        ("raygeo.sketch.snap.producers", py.None()),
        Some(&kwargs),
    )?;
    module.setattr("__spec__", package_spec)?;
    module.setattr("__package__", "raygeo.sketch.snap")?;
    module.setattr("__path__", pyo3::types::PyList::empty(py))?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    for (name, register) in [
        ("centers", centers::register as fn(&Bound<'_, PyModule>) -> PyResult<()>),
        ("entity_points", entity_points::register),
        ("equidistant", equidistant::register),
        ("intersections", intersections::register),
        ("midpoints", midpoints::register),
        ("on_entity", on_entity::register),
    ] {
        let full_name = format!("raygeo.sketch.snap.producers.{name}");
        let child = PyModule::new(py, name)?;
        register(&child)?;
        child.setattr("__package__", "raygeo.sketch.snap.producers")?;
        child.setattr(
            "__spec__",
            spec_cls.call1((full_name.as_str(), py.None()))?,
        )?;
        module.add_submodule(&child)?;
        module.setattr(name, &child)?;
        sys_modules.set_item(full_name, &child)?;
    }
    Ok(())
}
