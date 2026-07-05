pub(crate) mod entry;

use pyo3::prelude::*;

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let machining_mod = PyModule::new(py, "machining")?;
    entry::register(&machining_mod)?;
    parent.add_submodule(&machining_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.machining", &machining_mod)?;

    Ok(())
}
