pyo3_stub_gen::module_doc!("raygeo.cnc", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
CNC domain layer: operation orchestration.

Sequences machining operations (entry, clearing, finish), resolves
tool-aware State via StateStrategy, and drives the geo/ops primitives.
";

pub(crate) mod execution;
pub(crate) mod plan;
pub(crate) mod tool;

use pyo3::prelude::*;

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let cnc_mod = PyModule::new(py, "cnc")?;
    cnc_mod.setattr("__doc__", MODULE_DOC)?;
    execution::register(&cnc_mod)?;
    plan::register(&cnc_mod)?;
    tool::register(&cnc_mod)?;
    parent.add_submodule(&cnc_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc", &cnc_mod)?;

    Ok(())
}
