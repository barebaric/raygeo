//! Snap geometry to nearby sketch points, lines, and intersections.

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) mod engine;
pub(crate) mod producers;
pub(crate) mod spatial;
pub(crate) mod types;

pub(crate) fn register(sketch: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = sketch.py();
    let snap = PyModule::new(py, "snap")?;

    let types_mod = PyModule::new(py, "types")?;
    types::register(&types_mod)?;
    types::register(&snap)?;

    let spatial_mod = PyModule::new(py, "spatial")?;
    spatial::register(&spatial_mod)?;
    spatial::register(&snap)?;

    let engine_mod = PyModule::new(py, "engine")?;
    engine::register(&engine_mod)?;
    engine::register(&snap)?;

    let producers_mod = PyModule::new(py, "producers")?;
    producers::register(&producers_mod)?;
    producers::register(&snap)?;

    let spec_cls = py
        .import("importlib.machinery")?
        .getattr("ModuleSpec")?;
    for (name, module) in [
        ("types", &types_mod),
        ("spatial", &spatial_mod),
        ("engine", &engine_mod),
        ("producers", &producers_mod),
    ] {
        let full_name = format!("raygeo.sketch.snap.{name}");
        let spec = spec_cls.call1((full_name.as_str(), py.None()))?;
        module.setattr("__spec__", spec)?;
        module.setattr("__package__", "raygeo.sketch.snap")?;
        snap.add_submodule(module)?;
        snap.setattr(name, module)?;
    }
    let snap_spec = spec_cls.call1(("raygeo.sketch.snap", py.None()))?;
    snap.setattr("__spec__", snap_spec)?;
    snap.setattr("__package__", "raygeo.sketch")?;

    // Re-export producer classes at the package root.
    for name in [
        "CentersProducer",
        "EntityPointsProducer",
        "EquidistantLinesProducer",
        "IntersectionsProducer",
        "MidpointsProducer",
        "OnEntityProducer",
    ] {
        snap.setattr(name, producers_mod.getattr(name)?)?;
    }

    sketch.add_submodule(&snap)?;
    sketch.setattr("snap", &snap)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.snap", &snap)?;
    sys_modules.set_item("raygeo.sketch.snap.types", &types_mod)?;
    sys_modules.set_item("raygeo.sketch.snap.spatial", &spatial_mod)?;
    sys_modules.set_item("raygeo.sketch.snap.engine", &engine_mod)?;
    sys_modules.set_item("raygeo.sketch.snap.producers", &producers_mod)?;
    Ok(())
}
