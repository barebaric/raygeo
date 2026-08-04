use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::matrix::Matrix;
use crate::python::geo::geometry::Geometry;
use crate::python::svg::color::PyColorAttr;
use crate::svg::color::ColorAttr;
use crate::svg::transform::parse_svg_transform;
use crate::svg::{
    filter_svg_by_color, geometry_to_svg_path, parse_svg_path_data,
    svg_string_to_geometries, svg_string_to_geometries_by_color,
    svg_string_to_geometries_by_layer, svg_string_to_geometry,
    svg_string_to_geometry_by_color, svg_string_to_geometry_by_layer,
};

pub(crate) mod color;
pub(crate) mod length;
pub(crate) mod metadata;
pub(crate) mod transform;

pyo3_stub_gen::module_doc!("raygeo.svg", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
SVG parsing and geometry extraction.

Extracts Geometry objects from SVG documents — either as a flat list
or grouped by layer or by color — and provides parsers for path data
and transforms, length handling, metadata extraction and path export.
";

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.geo import Geometry

    def svg_string_to_geometries(
        svg_str: str,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
    ) -> list[raygeo.geo.Geometry]:
        """Parse an SVG string and extract all path elements as Geometry objects.

        Recursively traverses the SVG XML tree, extracting d attributes
        from path elements and converting them to Geometry.

        :param svg_str: SVG document as a string.
        :param scale_x: X-axis scale factor for coordinate transform.
        :param scale_y: Y-axis scale factor for coordinate transform.
        :returns: List of Geometry objects from all path elements.
        :complexity: O(n) where n = size of SVG document
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
    let geos = svg_string_to_geometries(svg_str, scale_x, scale_y)?;
    Ok(geos.into_iter().map(|g| Geometry { inner: g }).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def svg_string_to_geometry(
        svg_str: str,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
    ) -> raygeo.geo.Geometry:
        """Parse an SVG string and merge all subpaths into a single Geometry.

        Like svg_string_to_geometries but returns one combined Geometry
        instead of a list, avoiding a Python-side merge loop.

        :param svg_str: SVG document as a string.
        :param scale_x: X-axis scale factor for coordinate transform.
        :param scale_y: Y-axis scale factor for coordinate transform.
        :returns: A single Geometry containing all paths.
        :complexity: O(n) where n = size of SVG document
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "svg_string_to_geometry")]
#[pyo3(signature = (svg_str, scale_x=1.0, scale_y=1.0))]
fn py_svg_string_to_geometry(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
) -> PyResult<Geometry> {
    let geo = svg_string_to_geometry(svg_str, scale_x, scale_y)?;
    Ok(Geometry { inner: geo })
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy
    import numpy.typing
    from raygeo.geo import Geometry

    def parse_svg_path_data(
        path_data: str,
        transform: numpy.typing.NDArray[numpy.float64] | None = None,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
    ) -> list[raygeo.geo.Geometry]:
        """Parse an SVG path d attribute into a list of Geometry objects.

        Supports M/m, L/l, H/h, V/v, C/c, Z/z commands.
        Cubic Bezier curves are flattened to line segments (20 steps).

        :param path_data: SVG path d attribute string.
        :param transform: 3x3 affine transformation matrix, or None for identity.
        :param scale_x: X-axis scale factor for coordinate transform.
        :param scale_y: Y-axis scale factor for coordinate transform.
        :returns: List of Geometry objects, one per subpath.
        :complexity: O(n) where n = length of path data
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
        // Flat array is in row-major order; convert to a Matrix
        Matrix::from_cols_array(&[
            flat.first().copied().unwrap_or(1.0),
            flat.get(1).copied().unwrap_or(0.0),
            flat.get(2).copied().unwrap_or(0.0),
            flat.get(3).copied().unwrap_or(0.0),
            flat.get(4).copied().unwrap_or(1.0),
            flat.get(5).copied().unwrap_or(0.0),
            flat.get(6).copied().unwrap_or(0.0),
            flat.get(7).copied().unwrap_or(0.0),
            flat.get(8).copied().unwrap_or(1.0),
        ])
    } else {
        parse_svg_transform("")
    };

    let geos = parse_svg_path_data(path_data, &matrix, scale_x, scale_y)?;
    Ok(geos.into_iter().map(|g| Geometry { inner: g }).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def geometry_to_svg_path(
        geometry: raygeo.geo.Geometry,
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
        :complexity: O(n) where n = number of commands
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
    geometry_to_svg_path(&geometry.inner, width, height)
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.geo import Geometry

    def svg_string_to_geometries_by_layer(
        svg_str: str,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
    ) -> list[tuple[str, list[raygeo.geo.Geometry]]]:
        """Extract geometries grouped by top-level <g> layer.

        Returns a list of (layer_id, geometries) tuples. Only top-level
        <g> elements with an id attribute are treated as layers.

        :param svg_str: SVG document as a string.
        :param scale_x: X-axis scale factor for coordinate transform.
        :param scale_y: Y-axis scale factor for coordinate transform.
        :returns: List of (layer_id, geometry_list) tuples.
        :complexity: O(n) where n = size of SVG document
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "svg_string_to_geometries_by_layer")]
#[pyo3(signature = (svg_str, scale_x=1.0, scale_y=1.0))]
fn py_svg_string_to_geometries_by_layer(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
) -> PyResult<Vec<(String, Vec<Geometry>)>> {
    let layers = svg_string_to_geometries_by_layer(svg_str, scale_x, scale_y)?;
    Ok(layers
        .into_iter()
        .map(|(id, geos)| {
            (
                id,
                geos.into_iter()
                    .map(|g| Geometry { inner: g })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def svg_string_to_geometry_by_layer(
        svg_str: str,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
    ) -> list[tuple[str, raygeo.geo.Geometry]]:
        """Extract geometries grouped by layer, merged into one Geometry each.

        Like svg_string_to_geometries_by_layer but merges each layer's
        subpaths into a single Geometry, avoiding a Python merge loop.

        :param svg_str: SVG document as a string.
        :param scale_x: X-axis scale factor for coordinate transform.
        :param scale_y: Y-axis scale factor for coordinate transform.
        :returns: List of (layer_id, merged_geometry) tuples.
        :complexity: O(n) where n = size of SVG document
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "svg_string_to_geometry_by_layer")]
#[pyo3(signature = (svg_str, scale_x=1.0, scale_y=1.0))]
fn py_svg_string_to_geometry_by_layer(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
) -> PyResult<Vec<(String, Geometry)>> {
    let layers = svg_string_to_geometry_by_layer(svg_str, scale_x, scale_y)?;
    Ok(layers
        .into_iter()
        .map(|(id, g)| (id, Geometry { inner: g }))
        .collect())
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.geo import Geometry

    def svg_string_to_geometries_by_color(
        svg_str: str,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
        color_attr: raygeo.svg.color.ColorAttr = raygeo.svg.color.ColorAttr.FILL,
    ) -> list[tuple[str, list[raygeo.geo.Geometry]]]:
        """Extract geometries grouped by color.

        Walks the entire SVG tree and buckets shapes by their resolved
        fill/stroke color, applying SVG inheritance for presentation
        attributes. Bucket keys are lowercase #rrggbb hex strings; shapes
        with no usable color go into a '_no_color' bucket.

        :param svg_str: SVG document as a string.
        :param scale_x: X-axis scale factor for coordinate transform.
        :param scale_y: Y-axis scale factor for coordinate transform.
        :param color_attr: Color attribute to bucket by.
        :returns: List of (color_key, geometry_list) tuples.
        :complexity: O(n) where n = size of SVG document
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "svg_string_to_geometries_by_color")]
#[pyo3(signature = (svg_str, scale_x=1.0, scale_y=1.0, color_attr=PyColorAttr::Fill))]
fn py_svg_string_to_geometries_by_color(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
    color_attr: PyColorAttr,
) -> PyResult<Vec<(String, Vec<Geometry>)>> {
    let buckets = svg_string_to_geometries_by_color(
        svg_str,
        scale_x,
        scale_y,
        ColorAttr::from(color_attr),
    )?;
    Ok(buckets
        .into_iter()
        .map(|(key, geos)| {
            (
                key,
                geos.into_iter()
                    .map(|g| Geometry { inner: g })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def svg_string_to_geometry_by_color(
        svg_str: str,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
        color_attr: raygeo.svg.color.ColorAttr = raygeo.svg.color.ColorAttr.FILL,
    ) -> list[tuple[str, raygeo.geo.Geometry]]:
        """Extract geometries grouped by color, merged into one Geometry each.

        Like svg_string_to_geometries_by_color but merges each color
        bucket's subpaths into a single Geometry, avoiding a Python merge
        loop.

        :param svg_str: SVG document as a string.
        :param scale_x: X-axis scale factor for coordinate transform.
        :param scale_y: Y-axis scale factor for coordinate transform.
        :param color_attr: Color attribute to bucket by.
        :returns: List of (color_key, merged_geometry) tuples.
        :complexity: O(n) where n = size of SVG document
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "svg_string_to_geometry_by_color")]
#[pyo3(signature = (svg_str, scale_x=1.0, scale_y=1.0, color_attr=PyColorAttr::Fill))]
fn py_svg_string_to_geometry_by_color(
    svg_str: &str,
    scale_x: f64,
    scale_y: f64,
    color_attr: PyColorAttr,
) -> PyResult<Vec<(String, Geometry)>> {
    let buckets = svg_string_to_geometry_by_color(
        svg_str,
        scale_x,
        scale_y,
        ColorAttr::from(color_attr),
    )?;
    Ok(buckets
        .into_iter()
        .map(|(key, g)| (key, Geometry { inner: g }))
        .collect())
}

#[gen_stub_pyfunction(
    python = r#"
    def filter_svg_by_color(
        svg_str: str,
        color_key: str,
        color_attr: raygeo.svg.color.ColorAttr = raygeo.svg.color.ColorAttr.ANY,
    ) -> str:
        """Return a copy of the SVG containing only shapes of one color.

        Non-matching shapes are removed, preserving the rest of the
        document (groups, defs, namespaces) verbatim. Useful for
        rendering a color layer's base image.

        :param svg_str: SVG document as a string.
        :param color_key: Color bucket key to keep (e.g. '#ff0000' or
                          '_no_color').
        :param color_attr: Color attribute to bucket by.
        :returns: The filtered SVG document as a string.
        :complexity: O(n) where n = size of SVG document
        """
"#,
    module = "raygeo.svg"
)]
#[pyfunction(name = "filter_svg_by_color")]
#[pyo3(signature = (svg_str, color_key, color_attr=PyColorAttr::Any))]
fn py_filter_svg_by_color(
    svg_str: &str,
    color_key: &str,
    color_attr: PyColorAttr,
) -> PyResult<String> {
    Ok(filter_svg_by_color(
        svg_str,
        ColorAttr::from(color_attr),
        color_key,
    )?)
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let svg_mod = PyModule::new(m.py(), "svg")?;

    svg_mod.setattr("__doc__", MODULE_DOC)?;
    svg_mod.add(
        "__all__",
        vec![
            "geometry_to_svg_path",
            "parse_svg_path_data",
            "svg_string_to_geometries",
            "svg_string_to_geometry",
            "svg_string_to_geometries_by_layer",
            "svg_string_to_geometry_by_layer",
            "svg_string_to_geometries_by_color",
            "svg_string_to_geometry_by_color",
            "filter_svg_by_color",
            "length",
            "metadata",
            "transform",
        ],
    )?;

    svg_mod.add_function(wrap_pyfunction!(
        py_svg_string_to_geometries,
        &svg_mod
    )?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_svg_string_to_geometry, &svg_mod)?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_parse_svg_path_data, &svg_mod)?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_geometry_to_svg_path, &svg_mod)?)?;
    svg_mod.add_function(wrap_pyfunction!(
        py_svg_string_to_geometries_by_layer,
        &svg_mod
    )?)?;
    svg_mod.add_function(wrap_pyfunction!(
        py_svg_string_to_geometry_by_layer,
        &svg_mod
    )?)?;
    svg_mod.add_function(wrap_pyfunction!(
        py_svg_string_to_geometries_by_color,
        &svg_mod
    )?)?;
    svg_mod.add_function(wrap_pyfunction!(
        py_svg_string_to_geometry_by_color,
        &svg_mod
    )?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_filter_svg_by_color, &svg_mod)?)?;

    length::register(&svg_mod)?;
    metadata::register(&svg_mod)?;
    transform::register(&svg_mod)?;
    color::register(&svg_mod)?;

    m.add_submodule(&svg_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.svg", &svg_mod)?;

    Ok(())
}
