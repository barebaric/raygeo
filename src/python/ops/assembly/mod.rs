use pyo3::prelude::*;

pub(crate) mod adaptive;
pub(crate) mod helix;
pub(crate) mod profile;
pub(crate) mod ramp;
pub(crate) mod result;
pub(crate) mod spiral;
pub(crate) mod toroid;
pub(crate) mod wavefront;

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let assembly_mod = PyModule::new(py, "assembly")?;

    adaptive::register(&assembly_mod)?;
    helix::register(&assembly_mod)?;
    profile::register(&assembly_mod)?;
    ramp::register(&assembly_mod)?;
    result::register(&assembly_mod)?;
    spiral::register(&assembly_mod)?;
    toroid::register(&assembly_mod)?;
    wavefront::register(&assembly_mod)?;

    ops_mod.add_submodule(&assembly_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly", &assembly_mod)?;

    Ok(())
}
