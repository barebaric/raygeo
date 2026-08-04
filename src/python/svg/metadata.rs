use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::svg::length::SvgLength;
use crate::svg::metadata::extract_svg_metadata;
use crate::svg::metadata::SvgMetadata as CoreSvgMetadata;

pyo3_stub_gen::module_doc!("raygeo.svg.metadata", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
SVG metadata extraction.

Extracts width, height, units and viewBox values from the root
<svg> element of an SVG document.
";

/// SVG document metadata extracted from an SVG string.
///
/// Provides width, height, units and viewBox values parsed from the
/// root ``<svg>`` element.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.svg.metadata",
    name = "SvgMetadata",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct SvgMetadata {
    inner: CoreSvgMetadata,
}

impl From<CoreSvgMetadata> for SvgMetadata {
    fn from(inner: CoreSvgMetadata) -> Self {
        SvgMetadata { inner }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl SvgMetadata {
    /// Document width as a numeric value (may be ``None`` if not set).
    #[getter]
    fn get_width(&self) -> Option<f64> {
        self.inner.width
    }

    /// Document height as a numeric value (may be ``None`` if not set).
    #[getter]
    fn get_height(&self) -> Option<f64> {
        self.inner.height
    }

    /// Unit string for the width attribute (e.g. ``"mm"``, ``"in"``, ``"px"``).
    #[getter]
    fn get_width_unit(&self) -> &str {
        &self.inner.width_unit
    }

    /// Unit string for the height attribute.
    #[getter]
    fn get_height_unit(&self) -> &str {
        &self.inner.height_unit
    }

    /// ViewBox as ``(min_x, min_y, width, height)``, or ``None``.
    #[getter]
    fn get_viewbox(&self) -> Option<(f64, f64, f64, f64)> {
        self.inner.viewbox
    }

    /// Convert the document width to millimetres.
    ///
    /// :param dpi: Pixels-per-inch for px/unitless conversion (default 96).
    /// :returns: Width in millimetres, or ``None`` if not set.
    /// :complexity: O(1)
    #[pyo3(signature = (dpi=96.0))]
    fn width_mm(&self, dpi: f64) -> Option<f64> {
        self.inner.width.map(|w| {
            let sl = SvgLength {
                value: w,
                unit: self.inner.width_unit.clone(),
            };
            sl.to_mm(dpi)
        })
    }

    /// Convert the document height to millimetres.
    ///
    /// :param dpi: Pixels-per-inch for px/unitless conversion (default 96).
    /// :returns: Height in millimetres, or ``None`` if not set.
    /// :complexity: O(1)
    #[pyo3(signature = (dpi=96.0))]
    fn height_mm(&self, dpi: f64) -> Option<f64> {
        self.inner.height.map(|h| {
            let sl = SvgLength {
                value: h,
                unit: self.inner.height_unit.clone(),
            };
            sl.to_mm(dpi)
        })
    }

    /// Convert the document width to pixels.
    ///
    /// :param dpi: Pixels-per-inch for conversion (default 96).
    /// :returns: Width in pixels, or ``None`` if not set.
    /// :complexity: O(1)
    #[pyo3(signature = (dpi=96.0))]
    fn width_px(&self, dpi: f64) -> Option<f64> {
        self.inner.width.map(|w| {
            let sl = SvgLength {
                value: w,
                unit: self.inner.width_unit.clone(),
            };
            sl.to_px(dpi)
        })
    }

    /// Convert the document height to pixels.
    ///
    /// :param dpi: Pixels-per-inch for conversion (default 96).
    /// :returns: Height in pixels, or ``None`` if not set.
    /// :complexity: O(1)
    #[pyo3(signature = (dpi=96.0))]
    fn height_px(&self, dpi: f64) -> Option<f64> {
        self.inner.height.map(|h| {
            let sl = SvgLength {
                value: h,
                unit: self.inner.height_unit.clone(),
            };
            sl.to_px(dpi)
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "SvgMetadata(width={:?}, height={:?}, width_unit={:?}, height_unit={:?}, viewbox={:?})",
            self.inner.width,
            self.inner.height,
            self.inner.width_unit,
            self.inner.height_unit,
            self.inner.viewbox,
        )
    }

    fn __richcmp__(
        &self,
        other: &Self,
        op: pyo3::class::basic::CompareOp,
    ) -> bool {
        match op {
            pyo3::class::basic::CompareOp::Eq => {
                self.inner.width == other.inner.width
                    && self.inner.height == other.inner.height
                    && self.inner.width_unit == other.inner.width_unit
                    && self.inner.height_unit == other.inner.height_unit
                    && self.inner.viewbox == other.inner.viewbox
            }
            pyo3::class::basic::CompareOp::Ne => {
                self.inner.width != other.inner.width
                    || self.inner.height != other.inner.height
                    || self.inner.width_unit != other.inner.width_unit
                    || self.inner.height_unit != other.inner.height_unit
                    || self.inner.viewbox != other.inner.viewbox
            }
            _ => unimplemented!(),
        }
    }
}

#[gen_stub_pyfunction(
    python = r#"
    def extract_svg_metadata(
        svg_str: str,
    ) -> SvgMetadata:
        """Extract width, height, units and viewBox from an SVG string.

        :param svg_str: SVG document as a string.
        :returns: SvgMetadata instance with width, height, width_unit,
                  height_unit, and viewbox attributes.
        :complexity: O(n) where n = size of SVG document
        """
"#,
    module = "raygeo.svg.metadata"
)]
#[pyfunction(name = "extract_svg_metadata")]
fn py_extract_svg_metadata(svg_str: &str) -> PyResult<SvgMetadata> {
    let meta = extract_svg_metadata(svg_str)?;
    Ok(SvgMetadata::from(meta))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "metadata")?;
    sub_mod.setattr("__doc__", MODULE_DOC)?;
    sub_mod.add_class::<SvgMetadata>()?;
    sub_mod
        .add_function(wrap_pyfunction!(py_extract_svg_metadata, &sub_mod)?)?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.svg.metadata", &sub_mod)?;
    Ok(())
}
