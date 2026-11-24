use pyo3::prelude::*;

pub(crate) mod reader;
pub(crate) mod writer;

pyo3_stub_gen::module_doc!("raygeo.formats.dxf", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
DXF (Drawing Exchange Format) reading and writing.

Reads the ASCII DXF files CAD applications export for laser cutting:
lines, arcs, circles, ellipses, polylines with bulges, splines and the
block insertions that 3D CAD tools wrap their export in.

Results are in native drawing units with Y pointing up. Multiply by
``ReadOutput.units_to_mm`` to convert, which reads the ``$INSUNITS``
header value and falls back to millimetres for unitless files.

Sub-modules mirror the Rust layout: ``raygeo.formats.dxf.reader`` reads, and
``raygeo.formats.dxf.writer`` writes.
";

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let dxf_mod = PyModule::new(py, "dxf")?;

    dxf_mod.setattr("__doc__", MODULE_DOC)?;
    dxf_mod.add("__all__", vec!["reader", "writer"])?;

    reader::register(&dxf_mod)?;
    writer::register(&dxf_mod)?;

    m.add_submodule(&dxf_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.formats.dxf", &dxf_mod)?;

    Ok(())
}
