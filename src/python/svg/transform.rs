use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::svg::transform::parse_svg_transform;

pyo3_stub_gen::module_doc!("raygeo.svg.transform", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
SVG transform attribute parsing.

Parses a transform attribute value (translate, scale, rotate, skewX,
skewY, matrix, or a space-separated combination) into a 3x3 matrix.
";

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing

    def parse_svg_transform(
        transform_str: str,
    ) -> numpy.typing.NDArray[numpy.float64]:
        """Parse an SVG transform attribute string (translate only).

        Returns a 3x3 identity matrix with translation applied.

        :param transform_str: SVG transform attribute value.
        :returns: 3x3 affine transformation matrix as numpy array.
        :complexity: O(1)
        """
"#,
    module = "raygeo.svg.transform"
)]
#[pyfunction(name = "parse_svg_transform")]
fn py_parse_svg_transform(
    py: Python<'_>,
    transform_str: &str,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let m = parse_svg_transform(transform_str);
    // Convert from column-major DMat3 to row-major flat array
    let flat = vec![
        m.m.x_axis.x,
        m.m.y_axis.x,
        m.m.z_axis.x,
        m.m.x_axis.y,
        m.m.y_axis.y,
        m.m.z_axis.y,
        m.m.x_axis.z,
        m.m.y_axis.z,
        m.m.z_axis.z,
    ];
    let arr = numpy.call_method("array", (flat,), None)?;
    let reshaped = arr.call_method1("reshape", (3, 3))?;
    Ok(reshaped.unbind())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub_mod = PyModule::new(m.py(), "transform")?;
    sub_mod.setattr("__doc__", MODULE_DOC)?;
    sub_mod
        .add_function(wrap_pyfunction!(py_parse_svg_transform, &sub_mod)?)?;
    m.add_submodule(&sub_mod)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.svg.transform", &sub_mod)?;
    Ok(())
}
