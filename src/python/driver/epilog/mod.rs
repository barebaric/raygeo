//! The `raygeo.driver.epilog` Python submodule.

pub(crate) mod driver;

use pyo3::prelude::*;

pub(crate) const MODULE_DOC: &str = "\
Epilog laser support implemented in Rust. EpilogDriver uploads \
pre-encoded print jobs (the encoder decides the printer language) \
over the BSD LPD protocol (port 515) and reports reachability. The \
machine exposes no status channel, so every other operation raises \
NotImplementedError.
";

pyo3_stub_gen::module_doc!("raygeo.driver.epilog", "{}", MODULE_DOC);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let epilog_mod = PyModule::new(py, "epilog")?;

    epilog_mod.setattr("__doc__", MODULE_DOC)?;
    epilog_mod.add("__all__", vec!["EpilogDriver"])?;
    epilog_mod.add_class::<driver::PyEpilogDriver>()?;

    m.add_submodule(&epilog_mod)?;
    // Note: raygeo.driver.epilog is intentionally NOT registered in
    // sys.modules — the Python __init__.py at
    // python/raygeo/driver/epilog/__init__.py serves as the package and
    // delegates to this Rust module.
    Ok(())
}
