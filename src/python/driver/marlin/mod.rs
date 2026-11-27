//! The `raygeo.driver.marlin` Python submodule.

pub(crate) mod driver;
pub(crate) mod parser;
pub(crate) mod probe;

use pyo3::prelude::*;

pub(crate) const MODULE_DOC: &str = "\
Marlin device session: strict ping-pong commands, M114 position \
polling. Dialects remain data owned by Rayforge: resolved command \
templates are passed into the session.
";

pyo3_stub_gen::module_doc!("raygeo.driver.marlin", "{}", MODULE_DOC);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let marlin_mod = PyModule::new(py, "marlin")?;

    marlin_mod.setattr("__doc__", MODULE_DOC)?;
    marlin_mod.add("__all__", vec!["MarlinSerialDriver", "parser", "probe"])?;
    marlin_mod.add_class::<driver::PyMarlinSerialDriver>()?;

    let parser_mod = PyModule::new(py, "parser")?;
    parser::register(&parser_mod)?;
    marlin_mod.add_submodule(&parser_mod)?;

    let probe_mod = PyModule::new(py, "probe")?;
    probe::register(&probe_mod)?;
    marlin_mod.add_submodule(&probe_mod)?;

    m.add_submodule(&marlin_mod)?;
    Ok(())
}
