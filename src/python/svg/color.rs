use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyclass_enum;

use crate::svg::color::ColorAttr;

pyo3_stub_gen::module_doc!("raygeo.svg.color", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
SVG color attribute selection.

Selects which color attribute (fill, stroke, fill-else-stroke, or any)
determines the color bucket of a shape.
";

/// Which color attribute of a shape determines its color bucket.
///
/// ``FILL_ELSE_STROKE`` uses the fill color when present, otherwise the
/// stroke color. ``ANY`` buckets a shape by both its fill and its stroke
/// when they differ, producing two layers (one per color).
#[gen_stub_pyclass_enum]
#[pyclass(module = "raygeo.svg.color", name = "ColorAttr", from_py_object)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyColorAttr {
    /// Bucket by the resolved `fill` paint.
    #[pyo3(name = "FILL")]
    Fill,
    /// Bucket by the resolved `stroke` paint.
    #[pyo3(name = "STROKE")]
    Stroke,
    /// Use the fill paint when present, otherwise the stroke paint.
    #[pyo3(name = "FILL_ELSE_STROKE")]
    FillElseStroke,
    /// Bucket by both fill and stroke when they differ, producing two
    /// layers (one per color).
    #[pyo3(name = "ANY")]
    Any,
}

impl From<PyColorAttr> for ColorAttr {
    fn from(mode: PyColorAttr) -> Self {
        match mode {
            PyColorAttr::Fill => ColorAttr::Fill,
            PyColorAttr::Stroke => ColorAttr::Stroke,
            PyColorAttr::FillElseStroke => ColorAttr::FillElseStroke,
            PyColorAttr::Any => ColorAttr::Any,
        }
    }
}

#[pymethods]
impl PyColorAttr {
    fn __repr__(&self) -> String {
        format!("ColorAttr.{}", self.name())
    }

    fn __str__(&self) -> String {
        self.value().to_string()
    }

    #[getter]
    fn name(&self) -> &str {
        match self {
            PyColorAttr::Fill => "FILL",
            PyColorAttr::Stroke => "STROKE",
            PyColorAttr::FillElseStroke => "FILL_ELSE_STROKE",
            PyColorAttr::Any => "ANY",
        }
    }

    #[getter]
    fn value(&self) -> &str {
        match self {
            PyColorAttr::Fill => "fill",
            PyColorAttr::Stroke => "stroke",
            PyColorAttr::FillElseStroke => "fill_else_stroke",
            PyColorAttr::Any => "any",
        }
    }

    /// Enum members are singletons: copying returns the canonical member.
    fn __deepcopy__(
        &self,
        py: Python<'_>,
        _memo: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        Ok(py.get_type::<PyColorAttr>().getattr(self.name())?.unbind())
    }

    /// Reconstruct from ``getattr(ColorAttr, name)`` so members can be
    /// pickled and deep-copied (e.g. inside a serializable spec).
    fn __reduce__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let cls = py.get_type::<PyColorAttr>();
        let builtins = py.import("builtins")?;
        let getattr = builtins.getattr("getattr")?;
        let args = (cls, self.name());
        Ok((getattr, args).into_pyobject(py)?.into_any().unbind())
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "color")?;
    sub_mod.setattr("__doc__", MODULE_DOC)?;
    sub_mod.add_class::<PyColorAttr>()?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.svg.color", &sub_mod)?;
    Ok(())
}
