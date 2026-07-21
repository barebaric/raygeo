pyo3_stub_gen::module_doc!("raygeo.cnc.execution", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "CNC execution orchestration.";

pub mod converter;
pub(crate) mod intent;
pub mod specs;

use pyo3::prelude::*;

use crate::python::pipeline::execute::set_execute_hook;

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let exec_mod = PyModule::new(py, "execution")?;
    exec_mod.setattr("__doc__", MODULE_DOC)?;

    intent::register(&exec_mod)?;
    specs::register(&exec_mod)?;

    parent.add_submodule(&exec_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.execution", &exec_mod)?;

    // Set the pipeline execute hook so the bare execute_stages and
    // Pipeline.execute functions can dispatch through the CNC layer.
    set_execute_hook(converter::create_execute_hook());

    Ok(())
}
