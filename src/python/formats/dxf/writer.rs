use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::formats::dxf::writer::write as write_core;
use crate::python::geo::geometry::Geometry;

pyo3_stub_gen::module_doc!("raygeo.formats.dxf.writer", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Writing DXF documents.

Emits R12-compatible ASCII DXF with millimetres declared, the variant
every CAD tool and laser cutter controller accepts.
";

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def write(geometry: raygeo.geo.Geometry) -> str:
        """Write geometry as a DXF document.

        Consecutive lines become one ``LWPOLYLINE`` per contour, arcs
        remain ``ARC`` entities, and beziers are written as splines with
        control points and knots, so a geometry round-trips through
        ``write()`` and ``reader.read()`` without loss.

        :param geometry: The geometry to write, in millimetres.
        :returns: The DXF document as text.
        :complexity: O(n) where n = number of commands
        """
"#,
    module = "raygeo.formats.dxf.writer"
)]
#[pyfunction(name = "write")]
fn py_write(geometry: &Geometry) -> String {
    write_core(&geometry.inner)
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub = PyModule::new(m.py(), "writer")?;
    sub.setattr("__doc__", MODULE_DOC)?;
    sub.add("__all__", vec!["write"])?;
    sub.add_function(wrap_pyfunction!(py_write, &sub)?)?;
    m.add_submodule(&sub)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.formats.dxf.writer", &sub)?;
    Ok(())
}
