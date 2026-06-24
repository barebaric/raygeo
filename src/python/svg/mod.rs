use glam::DMat3;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::python::geo::geometry::Geometry;
use crate::svg;

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
    ) -> list[raygeo.Geometry]:
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
        // Flat array is in row-major order; convert to column-major DMat3
        DMat3::from_cols(
            glam::DVec3::new(
                flat.first().copied().unwrap_or(1.0),
                flat.get(3).copied().unwrap_or(0.0),
                flat.get(6).copied().unwrap_or(0.0),
            ),
            glam::DVec3::new(
                flat.get(1).copied().unwrap_or(0.0),
                flat.get(4).copied().unwrap_or(1.0),
                flat.get(7).copied().unwrap_or(0.0),
            ),
            glam::DVec3::new(
                flat.get(2).copied().unwrap_or(0.0),
                flat.get(5).copied().unwrap_or(0.0),
                flat.get(8).copied().unwrap_or(1.0),
            ),
        )
    } else {
        svg::parse_svg_transform("")
    };

    let geos = svg::parse_svg_path_data(path_data, matrix, scale_x, scale_y)?;
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
        :complexity: O(1)
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
    // Convert from column-major DMat3 to row-major flat array
    let flat = vec![
        m.x_axis.x, m.y_axis.x, m.z_axis.x, m.x_axis.y, m.y_axis.y, m.z_axis.y,
        m.x_axis.z, m.y_axis.z, m.z_axis.z,
    ];
    let arr = numpy.call_method("array", (flat,), None)?;
    let reshaped = arr.call_method1("reshape", (3, 3))?;
    Ok(reshaped.unbind())
}

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.geo import Geometry

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
    let geos = svg::svg_string_to_geometries(svg_str, scale_x, scale_y)?;
    Ok(geos.into_iter().map(|g| Geometry { inner: g }).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def svg_string_to_geometry(
        svg_str: str,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
    ) -> raygeo.Geometry:
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
    let geo = svg::svg_string_to_geometry(svg_str, scale_x, scale_y)?;
    Ok(Geometry { inner: geo })
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
    crate::svg::geometry_to_svg_path(&geometry.inner, width, height)
}

// ── parse_svg_length ──────────────────────────────────────────────

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
    module = "raygeo.svg"
)]
#[pyfunction(name = "parse_svg_length")]
fn py_parse_svg_length(length_str: &str) -> PyResult<(f64, String)> {
    let sl = svg::parse_svg_length(length_str)?;
    Ok((sl.value, sl.unit))
}

// ── svg_length_to_mm ──────────────────────────────────────────────

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
    module = "raygeo.svg"
)]
#[pyfunction(name = "svg_length_to_mm")]
#[pyo3(signature = (length_str, dpi=96.0))]
fn py_svg_length_to_mm(length_str: &str, dpi: f64) -> PyResult<f64> {
    let sl = svg::parse_svg_length(length_str)?;
    Ok(sl.to_mm(dpi))
}

// ── svg_length_to_px ──────────────────────────────────────────────

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
    module = "raygeo.svg"
)]
#[pyfunction(name = "svg_length_to_px")]
#[pyo3(signature = (length_str, dpi=96.0))]
fn py_svg_length_to_px(length_str: &str, dpi: f64) -> PyResult<f64> {
    let sl = svg::parse_svg_length(length_str)?;
    Ok(sl.to_px(dpi))
}

// ── SvgMetadata Python class ──────────────────────────────────────

/// SVG document metadata extracted from an SVG string.
///
/// Provides width, height, units and viewBox values parsed from the
/// root ``<svg>`` element.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.svg", name = "SvgMetadata", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct SvgMetadata {
    inner: svg::SvgMetadata,
}

impl From<svg::SvgMetadata> for SvgMetadata {
    fn from(inner: svg::SvgMetadata) -> Self {
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
            let sl = crate::svg::SvgLength {
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
            let sl = crate::svg::SvgLength {
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
            let sl = crate::svg::SvgLength {
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
            let sl = crate::svg::SvgLength {
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

// ── extract_svg_metadata ──────────────────────────────────────────

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
    module = "raygeo.svg"
)]
#[pyfunction(name = "extract_svg_metadata")]
fn py_extract_svg_metadata(svg_str: &str) -> PyResult<SvgMetadata> {
    let meta = svg::extract_svg_metadata(svg_str)?;
    Ok(SvgMetadata::from(meta))
}

// ── svg_string_to_geometries_by_layer ─────────────────────────────

#[gen_stub_pyfunction(
    python = r#"
    from raygeo.geo import Geometry

    def svg_string_to_geometries_by_layer(
        svg_str: str,
        scale_x: float = 1.0,
        scale_y: float = 1.0,
    ) -> list[tuple[str, list[raygeo.Geometry]]]:
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
    let layers =
        svg::svg_string_to_geometries_by_layer(svg_str, scale_x, scale_y)?;
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
    ) -> list[tuple[str, raygeo.Geometry]]:
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
    let layers =
        svg::svg_string_to_geometry_by_layer(svg_str, scale_x, scale_y)?;
    Ok(layers
        .into_iter()
        .map(|(id, g)| (id, Geometry { inner: g }))
        .collect())
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let svg_mod = PyModule::new(m.py(), "svg")?;

    svg_mod.add(
        "__all__",
        vec![
            "SvgMetadata",
            "extract_svg_metadata",
            "geometry_to_svg_path",
            "parse_svg_length",
            "parse_svg_path_data",
            "parse_svg_transform",
            "svg_length_to_mm",
            "svg_length_to_px",
            "svg_string_to_geometries",
            "svg_string_to_geometry",
            "svg_string_to_geometry_by_layer",
            "svg_string_to_geometries_by_layer",
        ],
    )?;

    svg_mod.add_class::<SvgMetadata>()?;
    svg_mod
        .add_function(wrap_pyfunction!(py_extract_svg_metadata, &svg_mod)?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_geometry_to_svg_path, &svg_mod)?)?;
    svg_mod.add_function(wrap_pyfunction!(py_parse_svg_length, &svg_mod)?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_parse_svg_path_data, &svg_mod)?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_parse_svg_transform, &svg_mod)?)?;
    svg_mod.add_function(wrap_pyfunction!(py_svg_length_to_mm, &svg_mod)?)?;
    svg_mod.add_function(wrap_pyfunction!(py_svg_length_to_px, &svg_mod)?)?;
    svg_mod.add_function(wrap_pyfunction!(
        py_svg_string_to_geometries,
        &svg_mod
    )?)?;
    svg_mod
        .add_function(wrap_pyfunction!(py_svg_string_to_geometry, &svg_mod)?)?;
    svg_mod.add_function(wrap_pyfunction!(
        py_svg_string_to_geometry_by_layer,
        &svg_mod
    )?)?;
    svg_mod.add_function(wrap_pyfunction!(
        py_svg_string_to_geometries_by_layer,
        &svg_mod
    )?)?;

    m.add_submodule(&svg_mod)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.svg", &svg_mod)?;

    Ok(())
}
