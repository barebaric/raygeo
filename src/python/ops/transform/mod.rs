use pyo3::prelude::*;

pub(crate) mod link;
pub(crate) mod optimize;

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let transform_mod = PyModule::new(ops_mod.py(), "transform")?;

    link::register(&transform_mod)?;
    optimize::register(&transform_mod)?;

    ops_mod.add_submodule(&transform_mod)?;

    let sys_modules = ops_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform", &transform_mod)?;

    Ok(())
}
