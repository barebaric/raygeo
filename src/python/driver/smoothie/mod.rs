//! The `raygeo.driver.smoothie` Python submodule.

pub(crate) mod driver;
pub(crate) mod probe;

use pyo3::prelude::*;

pub(crate) const MODULE_DOC: &str = "\
Smoothieware device session: strict ping-pong commands, Grbl-style \
status reports. Dialects remain data owned by Rayforge: resolved \
command templates are passed into the session.
";

pyo3_stub_gen::module_doc!("raygeo.driver.smoothie", "{}", MODULE_DOC);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let smoothie_mod = PyModule::new(py, "smoothie")?;

    smoothie_mod.setattr("__doc__", MODULE_DOC)?;
    smoothie_mod.add("__all__", vec!["SmoothieDriver", "probe"])?;
    probe::register(&smoothie_mod)?;
    smoothie_mod.add_class::<driver::PySmoothieDriver>()?;

    m.add_submodule(&smoothie_mod)?;
    Ok(())
}
