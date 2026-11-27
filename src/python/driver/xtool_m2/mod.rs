//! PyO3 bindings for the xTool M2 session (`raygeo.driver.xtool_m2`).

pub(crate) mod driver;

use pyo3::prelude::*;

const MODULE_DOC: &str = "\
xTool M2 WS-V2 driver. The session supports monitoring, motion and job \
control for jobs already staged on the machine; G-code upload is not \
implemented.";

pyo3_stub_gen::module_doc!("raygeo.driver.xtool_m2", "{}", MODULE_DOC);

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let module = PyModule::new(py, "xtool_m2")?;
    module.setattr("__doc__", MODULE_DOC)?;
    module.add("__all__", vec!["XtoolM2Driver"])?;
    module.add_class::<driver::PyXtoolM2Driver>()?;
    parent.add_submodule(&module)?;
    Ok(())
}
