use pyo3::prelude::*;

pub(crate) mod dxf;
pub(crate) mod gcode;
pub(crate) mod svg;

pyo3_stub_gen::module_doc!("raygeo.formats", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
File-format import and export.

Each sub-module translates between one external drawing format and
geometry: ``raygeo.formats.dxf`` reads and writes DXF,
``raygeo.formats.gcode`` extracts geometry from G-code, and
``raygeo.formats.svg`` parses SVG and exports path data.
";

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let formats_mod = PyModule::new(py, "formats")?;

    formats_mod.setattr("__doc__", MODULE_DOC)?;

    dxf::register(&formats_mod)?;
    gcode::register(&formats_mod)?;
    svg::register(&formats_mod)?;

    m.add_submodule(&formats_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.formats", &formats_mod)?;

    Ok(())
}
