use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::python::geo::geometry::Geometry;
use crate::svg;

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing
    from raygeo import Geometry

    def parse_svg_path_data(
        path_data: str,
        transform: numpy.typing.NDArray[numpy.float64] | None = None,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
    ) -> list[raygeo.Geometry]:
        """Parse an SVG path d attribute into a list of Geometry objects.

        Supports M/m, L/l, H/h, V/v, C/c, Z/z commands.
        Cubic Bezier curves are flattened to line segments (20 steps).

        :param path_data: SVG path d attribute string.
        :param transform: 3x3 affine transformation matrix, or None for identity.
        :param scale_x: X-axis scale factor for coordinate transform.
        :param scale_y: Y-axis scale factor for coordinate transform.
        :returns: List of Geometry objects, one per subpath.
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "parse_svg_path_data")]
#[pyo3(signature = (path_data, transform=None, scale_x=1.0, scale_y=1.0))]
fn py_parse_svg_path_data(
    py: Python<'_>,
    path_data: &str,
    transform: Option<&Bound<'_, PyAny>>,
    scale_x: f64,
    scale_y: f64,
) -> PyResult<Vec<Geometry>> {
    let matrix = if let Some(t) = transform {
        let numpy = py.import("numpy")?;
        let arr = numpy.call_method1("asarray", (t,))?;
        let flat: Vec<f64> = arr
            .call_method("flatten", (), None)?
            .call_method0("tolist")?
            .extract()?;
        let mut m = [[0.0f64; 3]; 3];
        m[0][0] = flat.first().copied().unwrap_or(1.0);
        m[0][1] = flat.get(1).copied().unwrap_or(0.0);
        m[0][2] = flat.get(2).copied().unwrap_or(0.0);
        m[1][0] = flat.get(3).copied().unwrap_or(0.0);
        m[1][1] = flat.get(4).copied().unwrap_or(1.0);
        m[1][2] = flat.get(5).copied().unwrap_or(0.0);
        m[2][0] = flat.get(6).copied().unwrap_or(0.0);
        m[2][1] = flat.get(7).copied().unwrap_or(0.0);
        m[2][2] = flat.get(8).copied().unwrap_or(1.0);
        m
    } else {
        svg::parse_svg_transform("")
    };

    let geos = svg::parse_svg_path_data(path_data, &matrix, scale_x, scale_y)?;
    Ok(geos.into_iter().map(|g| Geometry { inner: g }).collect())
}

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
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "parse_svg_transform")]
fn py_parse_svg_transform(
    py: Python<'_>,
    transform_str: &str,
) -> PyResult<Py<PyAny>> {
    let numpy = py.import("numpy")?;
    let m = svg::parse_svg_transform(transform_str);
    let flat: Vec<f64> = m.iter().flat_map(|row| row.iter().copied()).collect();
    let arr = numpy.call_method("array", (flat,), None)?;
    let reshaped = arr.call_method1("reshape", (3, 3))?;
    Ok(reshaped.unbind())
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo import Geometry

    def svg_string_to_geometries(
        svg_str: str,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
    ) -> list[raygeo.Geometry]:
        """Parse an SVG string and extract all path elements as Geometry objects.

        Recursively traverses the SVG XML tree, extracting d attributes
        from path elements and converting them to Geometry.

        :param svg_str: SVG document as a string.
        :param scale_x: X-axis scale factor for coordinate transform.
        :param scale_y: Y-axis scale factor for coordinate transform.
        :returns: List of Geometry objects from all path elements.
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "svg_string_to_geometries")]
#[pyo3(signature = (svg_str, scale_x=1.0, scale_y=1.0))]
fn py_svg_string_to_geometries(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
) -> PyResult<Vec<Geometry>> {
    let geos = svg::svg_string_to_geometries(svg_str, scale_x, scale_y)?;
    Ok(geos.into_iter().map(|g| Geometry { inner: g }).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def geometry_to_svg_path(
        geometry: raygeo.Geometry,
        width: int,
        height: int,
    ) -> str:
        """Convert a normalized Geometry to an SVG path d attribute string.

        The geometry coordinates should be in normalized [0, 1] space.
        Coordinates are scaled to pixel dimensions via width and height,
        with the Y axis flipped (SVG Y increases downward).

        :param geometry: A Geometry object with normalized coordinates.
        :param width: Target pixel width.
        :param height: Target pixel height.
        :returns: SVG path d attribute string.
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "geometry_to_svg_path")]
fn py_geometry_to_svg_path(
    geometry: &Geometry,
    width: i32,
    height: i32,
) -> String {
    crate::svg::geometry_to_svg_path(&geometry.inner, width, height)
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let svg_mod = PyModule::new(m.py(), "svg")?;

    svg_mod.add(
        "__all__",
        vec![
            "geometry_to_svg_path",
            "parse_svg_path_data",
            "parse_svg_transform",
            "svg_string_to_geometries",
        ],
    )?;

    svg_mod
        .add_function(wrap_pyfunction!(py_geometry_to_svg_path, &svg_mod)?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_parse_svg_path_data, &svg_mod)?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_parse_svg_transform, &svg_mod)?)?;
    svg_mod.add_function(wrap_pyfunction!(
        py_svg_string_to_geometries,
        &svg_mod
    )?)?;

    m.add_submodule(&svg_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.svg", &svg_mod)?;

    Ok(())
}
