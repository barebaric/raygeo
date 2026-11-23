//! Python bindings for host-agnostic template support.

use pyo3::prelude::*;

pub(crate) mod counter;
pub(crate) mod placeholders;

/// Registers the `raygeo.template` module.
pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let m = PyModule::new(py, "template")?;
    m.add(
        "__all__",
        vec![
            "CounterSettings",
            "names_in_expression",
            "placeholder_names",
            "referenced_names",
        ],
    )?;
    counter::add_to(&m)?;
    placeholders::add_to(&m)?;
    parent.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.template", &m)?;

    Ok(())
}
