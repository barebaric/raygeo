//! Python bindings for barcode geometry rendering.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::barcode::geometry as core;
use crate::python::geo::geometry::Geometry as PyGeometry;

use super::PyEncoded;

pyo3_stub_gen::module_doc!("raygeo.barcode.geometry", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Barcode geometry rendering.

Turns encoded modules into closed outlines in millimetres, merges dark
modules into polygons with holes, adds optional human-readable text and
renders the inverted variant. The result is suitable both as a fill and
as a contour.
";

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let module = PyModule::new(py, "geometry")?;
    module.setattr("__doc__", MODULE_DOC)?;
    module.add("__all__", vec!["BarcodeStyle", "to_geometry"])?;
    module.add_class::<PyBarcodeStyle>()?;
    register_functions!(module, to_geometry_py);
    parent.add_submodule(&module)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.barcode.geometry", &module)?;
    Ok(())
}

/// How a barcode symbol is drawn.
#[gen_stub_pyclass(module = "raygeo.barcode.geometry")]
#[pyclass(
    module = "raygeo.barcode.geometry",
    name = "BarcodeStyle",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyBarcodeStyle {
    pub inner: core::BarcodeStyle,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyBarcodeStyle {
    #[new]
    #[pyo3(signature = (
        module_mm = 0.5,
        bar_height_mm = 15.0,
        quiet_zone = None,
        show_text = false,
        invert = false
    ))]
    fn new(
        module_mm: f64,
        bar_height_mm: f64,
        quiet_zone: Option<u32>,
        show_text: bool,
        invert: bool,
    ) -> Self {
        PyBarcodeStyle {
            inner: core::BarcodeStyle {
                module_mm,
                bar_height_mm,
                quiet_zone,
                show_text,
                invert,
            },
        }
    }

    /// Width of the narrowest bar, or one QR module, in millimetres.
    #[getter]
    fn module_mm(&self) -> f64 {
        self.inner.module_mm
    }

    /// Height of the bars, in millimetres.
    #[getter]
    fn bar_height_mm(&self) -> f64 {
        self.inner.bar_height_mm
    }

    /// Quiet zone in modules; ``None`` uses the symbology minimum.
    #[getter]
    fn quiet_zone(&self) -> Option<u32> {
        self.inner.quiet_zone
    }

    /// Whether to draw the human-readable text under a 1D code.
    #[getter]
    fn show_text(&self) -> bool {
        self.inner.show_text
    }

    /// Whether to draw a light code on a dark quiet-zone border.
    #[getter]
    fn invert(&self) -> bool {
        self.inner.invert
    }

    fn __repr__(&self) -> String {
        format!(
            "BarcodeStyle(module_mm={}, bar_height_mm={}, quiet_zone={:?}, \
             show_text={}, invert={})",
            self.inner.module_mm,
            self.inner.bar_height_mm,
            self.inner.quiet_zone,
            self.inner.show_text,
            self.inner.invert,
        )
    }
}

/// Renders an encoded symbol as closed outlines in millimetres.
#[gen_stub_pyfunction(module = "raygeo.barcode.geometry")]
#[pyfunction(name = "to_geometry")]
fn to_geometry_py(
    encoded: &PyEncoded,
    style: &PyBarcodeStyle,
) -> PyResult<PyGeometry> {
    let geometry = core::to_geometry(&encoded.inner, &style.inner)?;
    Ok(PyGeometry { inner: geometry })
}
