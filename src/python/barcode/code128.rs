//! Python bindings for Code 128 encoding.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::barcode::code128 as core;

use super::PyEncoded;

pyo3_stub_gen::module_doc!("raygeo.barcode.code128", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Code 128 encoding.

Long digit runs are packed as pairs in code set C; text uses code set B.
The encoder switches at run boundaries automatically.
";

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let module = PyModule::new(py, "code128")?;
    module.setattr("__doc__", MODULE_DOC)?;
    module.add("__all__", vec!["encode"])?;
    register_functions!(module, encode_py);
    parent.add_submodule(&module)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.barcode.code128", &module)?;
    Ok(())
}

/// Encodes printable ASCII as Code 128.
#[gen_stub_pyfunction(module = "raygeo.barcode.code128")]
#[pyfunction(name = "encode")]
fn encode_py(text: &str) -> PyResult<PyEncoded> {
    let encoded = core::encode(text)?;
    Ok(PyEncoded { inner: encoded })
}
