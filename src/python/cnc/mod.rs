pub(crate) mod machining;

use pyo3::prelude::*;

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let cnc_mod = PyModule::new(py, "cnc")?;
    machining::register(&cnc_mod)?;
    parent.add_submodule(&cnc_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc", &cnc_mod)?;

    Ok(())
}
