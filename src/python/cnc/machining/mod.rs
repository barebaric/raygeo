pyo3_stub_gen::module_doc!("raygeo.cnc.machining", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Machining operation orchestration.

Entry, clearing, and finish operations that combine geometry
with tool-aware state resolution.
";

pub(crate) mod entry;
pub(crate) mod plan;
pub(crate) mod wavefront;

use pyo3::prelude::*;

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let machining_mod = PyModule::new(py, "machining")?;
    machining_mod.setattr("__doc__", MODULE_DOC)?;
    entry::register(&machining_mod)?;
    wavefront::register(&machining_mod)?;
    plan::register(&machining_mod)?;
    parent.add_submodule(&machining_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.machining", &machining_mod)?;

    Ok(())
}
