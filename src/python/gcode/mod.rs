use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction};

use crate::gcode::parse_gcode as parse_gcode_core;
use crate::python::geo::geometry::Geometry;

pyo3_stub_gen::module_doc!("raygeo.gcode", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
G-code (RS274/NGC) geometry extraction.

Parses the common subset of RS274/NGC used by laser cutters, CAM tools
and LightBurn into ``Geometry`` objects. Only material-removing moves are
emitted: rapid moves, and feed moves at spindle speed zero, split the
result into separate contours.
";

/// The cutting geometry and skipped instructions of one G-code program.
#[gen_stub_pyclass(module = "raygeo.gcode")]
#[pyclass(
    name = "GcodeParseResult",
    skip_from_py_object,
    module = "raygeo.gcode"
)]
#[derive(Clone)]
pub struct PyGcodeParseResult {
    /// Cutting moves in millimetres, Y-up, one subpath per contour.
    #[pyo3(get)]
    pub geometry: Geometry,
    /// Instructions that were recognized but not applied, sorted.
    #[pyo3(get)]
    pub unsupported: Vec<String>,
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def parse_gcode(
        text: str,
        s0_is_travel: bool = True,
    ) -> raygeo.gcode.GcodeParseResult:
        """Parse a G-code program into cutting geometry in millimetres.

        Supports G0 (rapid), G1 (linear cut) and G2/G3 (arcs in I/J or R
        form), with G20/G21 units, G90/G91 distance mode and G90.1/G91.1
        arc-centre mode. Comments, block delete, ``%`` delimiters and
        CRLF line endings are handled. Feed, spindle speed, coolant and
        work offsets do not affect the geometry.

        :param text: The G-code program as text.
        :param s0_is_travel: Treat feed moves at spindle speed zero as
            travel (LightBurn fill passes and overscan).
        :returns: Cutting geometry in millimetres plus the sorted set of
            instructions that were skipped.
        :complexity: O(n) where n = size of the program
        """
"#,
    module = "raygeo.gcode"
)]
#[pyfunction(name = "parse_gcode")]
#[pyo3(signature = (text, s0_is_travel=true))]
fn py_parse_gcode(text: &str, s0_is_travel: bool) -> PyGcodeParseResult {
    let output = parse_gcode_core(text, s0_is_travel);
    PyGcodeParseResult {
        geometry: Geometry {
            inner: output.geometry,
        },
        unsupported: output.unsupported.into_iter().collect(),
    }
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let gcode_mod = PyModule::new(m.py(), "gcode")?;

    gcode_mod.setattr("__doc__", MODULE_DOC)?;
    gcode_mod.add("__all__", vec!["GcodeParseResult", "parse_gcode"])?;
    gcode_mod.add_class::<PyGcodeParseResult>()?;
    gcode_mod.add_function(wrap_pyfunction!(py_parse_gcode, &gcode_mod)?)?;

    m.add_submodule(&gcode_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.gcode", &gcode_mod)?;

    Ok(())
}
