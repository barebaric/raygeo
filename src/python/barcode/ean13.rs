//! Python bindings for EAN-13 encoding.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::barcode::ean13 as core;

use super::PyEncoded;

pyo3_stub_gen::module_doc!("raygeo.barcode.ean13", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
EAN-13 encoding.

Twelve digits get their check digit computed; thirteen digits have the
supplied check digit verified. Guard-bar columns are marked so the
geometry layer can extend them into the printed digits.
";

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let module = PyModule::new(py, "ean13")?;
    module.setattr("__doc__", MODULE_DOC)?;
    module.add("__all__", vec!["encode"])?;
    register_functions!(module, encode_py);
    parent.add_submodule(&module)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.barcode.ean13", &module)?;
    Ok(())
}

/// Encodes 12 or 13 digits as EAN-13.
#[gen_stub_pyfunction(module = "raygeo.barcode.ean13")]
#[pyfunction(name = "encode")]
fn encode_py(text: &str) -> PyResult<PyEncoded> {
    let encoded = core::encode(text)?;
    Ok(PyEncoded { inner: encoded })
}
