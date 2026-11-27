//! The `raydriver.ruida` Python submodule.

pub(crate) mod driver;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) const MODULE_DOC: &str = "\
Ruida laser controllers implemented in Rust. RuidaDriver speaks the \
binary rpascript protocol over USB, UDP or TCP: it runs complete job \
scripts, executes live motion and job-control commands, polls the \
controller status and reports the machine state through the uniform \
driver event surface. Job compilation from Ops lives in the encoder; \
this session consumes rpascript text.
";

pyo3_stub_gen::module_doc!("raygeo.driver.ruida", "{}", MODULE_DOC);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let ruida_mod = PyModule::new(py, "ruida")?;

    ruida_mod.setattr("__doc__", MODULE_DOC)?;
    ruida_mod.add("__all__", vec!["RuidaDriver", "encode_script"])?;
    ruida_mod.add_class::<driver::PyRuidaDriver>()?;
    ruida_mod
        .add_function(wrap_pyfunction!(encode_script, ruida_mod.clone())?)?;

    m.add_submodule(&ruida_mod)?;
    // Note: raydriver.ruida is intentionally NOT registered in
    // sys.modules — the Python __init__.py at
    // python/raydriver/ruida/__init__.py serves as the package and
    // delegates to this Rust module.
    Ok(())
}

/// Encode rpascript lines into the controller's raw byte stream,
/// including the file checksum.
#[gen_stub_pyfunction(module = "raygeo.driver.ruida")]
#[pyfunction]
fn encode_script(lines: Vec<String>) -> PyResult<Vec<u8>> {
    crate::driver::ruida::codec::encode_script(&lines)
        .map_err(|error| PyValueError::new_err(error.to_string()))
}
