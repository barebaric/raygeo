use pyo3::prelude::*;

pub(crate) mod dict;
pub(crate) mod numpy;

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let convert_mod = PyModule::new(ops_mod.py(), "convert")?;
    ops_mod.add_submodule(&convert_mod)?;

    let sys_modules = ops_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.convert", &convert_mod)?;

    Ok(())
}
