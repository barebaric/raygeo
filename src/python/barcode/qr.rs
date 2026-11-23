//! Python bindings for QR Code encoding.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::barcode::qr as core;

use super::{PyEncoded, PyQrErrorCorrection};

pyo3_stub_gen::module_doc!("raygeo.barcode.qr", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
QR Code encoding.

The smallest QR version that fits the text is selected automatically;
the requested error-correction level is kept exactly instead of being
boosted to a higher level.
";

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let module = PyModule::new(py, "qr")?;
    module.setattr("__doc__", MODULE_DOC)?;
    module.add("__all__", vec!["encode"])?;
    register_functions!(module, encode_py);
    parent.add_submodule(&module)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.barcode.qr", &module)?;
    Ok(())
}

/// Encodes text as a QR Code.
#[gen_stub_pyfunction(module = "raygeo.barcode.qr")]
#[pyfunction(name = "encode")]
#[pyo3(signature = (text, error_correction = PyQrErrorCorrection::Medium))]
fn encode_py(
    text: &str,
    error_correction: PyQrErrorCorrection,
) -> PyResult<PyEncoded> {
    let encoded = core::encode(text, error_correction.into())?;
    Ok(PyEncoded { inner: encoded })
}
