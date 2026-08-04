use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::svg::length::parse_svg_length;

pyo3_stub_gen::module_doc!("raygeo.svg.length", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
SVG length parsing and unit conversion.

Parses SVG length strings such as '10mm' or '2.5in' and converts
between millimetres, pixels and other CSS units.
";

#[gen_stub_pyfunction(
    python = r#"
    def parse_svg_length(
        length_str: str,
    ) -> tuple[float, str]:
        """Parse an SVG length string into a (value, unit) tuple.

        Supports: mm, cm, in, pt, pc, px. Unitless values default to 'px'.

        :param length_str: SVG length string (e.g. '10mm', '2.5in', '100').
        :returns: Tuple of (value, unit).
        :complexity: O(1)
        """
"#,
    module = "raygeo.svg.length"
)]
#[pyfunction(name = "parse_svg_length")]
fn py_parse_svg_length(length_str: &str) -> PyResult<(f64, String)> {
    let sl = parse_svg_length(length_str)?;
    Ok((sl.value, sl.unit))
}

#[gen_stub_pyfunction(
    python = r#"
    def svg_length_to_mm(
        length_str: str,
        dpi: float = 96.0,
    ) -> float:
        """Parse an SVG length string and convert to millimetres.

        :param length_str: SVG length string (e.g. '10mm', '2.5in', '100').
        :param dpi: Pixels per inch used for px/unitless conversion (default 96).
        :returns: Length in millimetres.
        :complexity: O(1)
        """
"#,
    module = "raygeo.svg.length"
)]
#[pyfunction(name = "svg_length_to_mm")]
#[pyo3(signature = (length_str, dpi=96.0))]
fn py_svg_length_to_mm(length_str: &str, dpi: f64) -> PyResult<f64> {
    let sl = parse_svg_length(length_str)?;
    Ok(sl.to_mm(dpi))
}

#[gen_stub_pyfunction(
    python = r#"
    def svg_length_to_px(
        length_str: str,
        dpi: float = 96.0,
    ) -> float:
        """Parse an SVG length string and convert to pixels.

        :param length_str: SVG length string (e.g. '10mm', '2.5in', '100').
        :param dpi: Pixels per inch used for px/unitless conversion (default 96).
        :returns: Length in pixels.
        :complexity: O(1)
        """
"#,
    module = "raygeo.svg.length"
)]
#[pyfunction(name = "svg_length_to_px")]
#[pyo3(signature = (length_str, dpi=96.0))]
fn py_svg_length_to_px(length_str: &str, dpi: f64) -> PyResult<f64> {
    let sl = parse_svg_length(length_str)?;
    Ok(sl.to_px(dpi))
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "length")?;
    sub_mod.setattr("__doc__", MODULE_DOC)?;
    sub_mod.add_function(wrap_pyfunction!(py_parse_svg_length, &sub_mod)?)?;
    sub_mod.add_function(wrap_pyfunction!(py_svg_length_to_mm, &sub_mod)?)?;
    sub_mod.add_function(wrap_pyfunction!(py_svg_length_to_px, &sub_mod)?)?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.svg.length", &sub_mod)?;
    Ok(())
}
