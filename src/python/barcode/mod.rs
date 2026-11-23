//! Python bindings for barcode and QR code generation.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pyfunction,
    gen_stub_pymethods,
};

use crate::barcode as core;

pub(crate) mod code128;
pub(crate) mod ean13;
pub(crate) mod geometry;
pub(crate) mod qr;

pyo3_stub_gen::module_doc!("raygeo.barcode", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Barcode and QR code generation.

Encodes text as QR Code, Code 128 or EAN-13 modules, and renders those
modules to millimetre-space geometry with optional human-readable text,
quiet-zone border and inverted variant.

The symbology modules expose their own ``encode`` function; the
top-level ``encode`` dispatches by ``Symbology``. ``geometry.to_geometry``
turns the encoded modules into closed outlines suitable for filling or
contouring.
";

/// Registers the `raygeo.barcode` module.
pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let module = PyModule::new(py, "barcode")?;
    module.setattr("__doc__", MODULE_DOC)?;
    module.add(
        "__all__",
        vec![
            "Encoded",
            "QrErrorCorrection",
            "Symbology",
            "encode",
            "min_quiet_zone",
        ],
    )?;
    module.add_class::<PySymbology>()?;
    module.add_class::<PyQrErrorCorrection>()?;
    module.add_class::<PyEncoded>()?;
    register_functions!(module, encode_py, min_quiet_zone_py);
    qr::register(&module)?;
    code128::register(&module)?;
    ean13::register(&module)?;
    geometry::register(&module)?;
    parent.add_submodule(&module)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.barcode", &module)?;
    Ok(())
}

/// A barcode symbology.
#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.barcode",
    name = "Symbology",
    eq,
    hash,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PySymbology {
    /// QR Code.
    #[pyo3(name = "QR")]
    Qr,
    /// Code 128.
    #[pyo3(name = "CODE128")]
    Code128,
    /// EAN-13.
    #[pyo3(name = "EAN13")]
    Ean13,
}

impl From<PySymbology> for core::Symbology {
    fn from(symbology: PySymbology) -> Self {
        match symbology {
            PySymbology::Qr => core::Symbology::Qr,
            PySymbology::Code128 => core::Symbology::Code128,
            PySymbology::Ean13 => core::Symbology::Ean13,
        }
    }
}

impl From<core::Symbology> for PySymbology {
    fn from(symbology: core::Symbology) -> Self {
        match symbology {
            core::Symbology::Qr => PySymbology::Qr,
            core::Symbology::Code128 => PySymbology::Code128,
            core::Symbology::Ean13 => PySymbology::Ean13,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySymbology {
    fn __repr__(&self) -> String {
        format!("Symbology.{}", self.name())
    }

    fn __str__(&self) -> String {
        self.value().to_string()
    }

    #[getter]
    fn name(&self) -> &str {
        match self {
            PySymbology::Qr => "QR",
            PySymbology::Code128 => "CODE128",
            PySymbology::Ean13 => "EAN13",
        }
    }

    /// The stable serialization value (`"qr"`, `"code128"`, `"ean13"`).
    #[getter]
    fn value(&self) -> &str {
        core::Symbology::from(*self).value()
    }

    /// Parses a serialization value produced by :attr:`value`.
    #[staticmethod]
    fn from_value(value: &str) -> PyResult<Self> {
        core::Symbology::from_value(value)
            .map(Into::into)
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown symbology {value:?}"
                ))
            })
    }

    fn __reduce__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let cls = py.get_type::<Self>();
        let builtins = py.import("builtins")?;
        let getattr = builtins.getattr("getattr")?;
        let args = (cls, self.name());
        Ok((getattr, args).into_pyobject(py)?.into_any().unbind())
    }
}

/// A QR Code error-correction level.
#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.barcode",
    name = "QrErrorCorrection",
    eq,
    hash,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PyQrErrorCorrection {
    /// Low, roughly 7% recovery.
    #[pyo3(name = "L")]
    Low,
    /// Medium, roughly 15% recovery.
    #[pyo3(name = "M")]
    Medium,
    /// Quartile, roughly 25% recovery.
    #[pyo3(name = "Q")]
    Quartile,
    /// High, roughly 30% recovery.
    #[pyo3(name = "H")]
    High,
}

impl From<PyQrErrorCorrection> for core::QrErrorCorrection {
    fn from(level: PyQrErrorCorrection) -> Self {
        match level {
            PyQrErrorCorrection::Low => core::QrErrorCorrection::Low,
            PyQrErrorCorrection::Medium => core::QrErrorCorrection::Medium,
            PyQrErrorCorrection::Quartile => core::QrErrorCorrection::Quartile,
            PyQrErrorCorrection::High => core::QrErrorCorrection::High,
        }
    }
}

impl From<core::QrErrorCorrection> for PyQrErrorCorrection {
    fn from(level: core::QrErrorCorrection) -> Self {
        match level {
            core::QrErrorCorrection::Low => PyQrErrorCorrection::Low,
            core::QrErrorCorrection::Medium => PyQrErrorCorrection::Medium,
            core::QrErrorCorrection::Quartile => PyQrErrorCorrection::Quartile,
            core::QrErrorCorrection::High => PyQrErrorCorrection::High,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyQrErrorCorrection {
    fn __repr__(&self) -> String {
        format!("QrErrorCorrection.{}", self.name())
    }

    fn __str__(&self) -> String {
        self.value().to_string()
    }

    #[getter]
    fn name(&self) -> &str {
        match self {
            PyQrErrorCorrection::Low => "L",
            PyQrErrorCorrection::Medium => "M",
            PyQrErrorCorrection::Quartile => "Q",
            PyQrErrorCorrection::High => "H",
        }
    }

    /// The stable serialization value (`"L"`, `"M"`, `"Q"`, `"H"`).
    #[getter]
    fn value(&self) -> &str {
        core::QrErrorCorrection::from(*self).value()
    }

    /// Parses a serialization value produced by :attr:`value`.
    #[staticmethod]
    fn from_value(value: &str) -> PyResult<Self> {
        core::QrErrorCorrection::from_value(value)
            .map(Into::into)
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown QR error-correction level {value:?}"
                ))
            })
    }

    fn __reduce__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let cls = py.get_type::<Self>();
        let builtins = py.import("builtins")?;
        let getattr = builtins.getattr("getattr")?;
        let args = (cls, self.name());
        Ok((getattr, args).into_pyobject(py)?.into_any().unbind())
    }
}

/// An encoded symbol.
#[gen_stub_pyclass(module = "raygeo.barcode")]
#[pyclass(module = "raygeo.barcode", name = "Encoded", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyEncoded {
    pub inner: core::Encoded,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEncoded {
    /// The symbology that produced this symbol.
    #[getter]
    fn symbology(&self) -> PySymbology {
        self.inner.symbology.into()
    }

    /// The module matrix, row-major, ``True`` for dark modules.
    #[getter]
    fn modules(&self) -> Vec<Vec<bool>> {
        self.inner.modules.clone()
    }

    /// The human-readable text; EAN-13 includes the check digit.
    #[getter]
    fn text(&self) -> &str {
        &self.inner.text
    }

    /// The symbol width in modules.
    #[getter]
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// The symbol height in modules.
    #[getter]
    fn height(&self) -> usize {
        self.inner.height()
    }

    /// The columns covered by EAN-13 guard bars.
    #[getter]
    fn guards(&self) -> Vec<usize> {
        self.inner.guards.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Encoded(symbology={}, width={}, height={})",
            self.inner.symbology.value(),
            self.inner.width(),
            self.inner.height(),
        )
    }
}

/// Encodes text in the requested symbology.
///
/// ``error_correction`` is used by QR only.
#[gen_stub_pyfunction(module = "raygeo.barcode")]
#[pyfunction(name = "encode")]
#[pyo3(signature = (
    text,
    symbology = PySymbology::Qr,
    error_correction = PyQrErrorCorrection::Medium
))]
fn encode_py(
    text: &str,
    symbology: PySymbology,
    error_correction: PyQrErrorCorrection,
) -> PyResult<PyEncoded> {
    let encoded =
        core::encode(text, symbology.into(), error_correction.into())?;
    Ok(PyEncoded { inner: encoded })
}

/// The minimum quiet zone for a symbology, in modules.
#[gen_stub_pyfunction(module = "raygeo.barcode")]
#[pyfunction(name = "min_quiet_zone")]
fn min_quiet_zone_py(symbology: PySymbology) -> u32 {
    core::min_quiet_zone(symbology.into())
}
