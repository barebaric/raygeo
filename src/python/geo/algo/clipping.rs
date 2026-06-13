pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.clipping",
    "{}",
    MODULE_DOC_CLIPPING
);

pub(crate) const MODULE_DOC_CLIPPING: &str = "\
Line and polygon clipping operations.

Provides functions for clipping line segments against rectangles and
polygon regions, as well as converting between float and Clipper
integer coordinate systems.
";

use super::super::flex_point::{
    extract_polygon, extract_polygons, int_poly_to_points,
};
use crate::geo::algo::clipping::{
    clip_line_segment_with_polygons, clip_line_segment_with_rect,
    subtract_polygons_from_line_segment,
};
use crate::Segment3D;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

type Point = (f64, f64);

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "clipping")?;
    m.setattr("__doc__", MODULE_DOC_CLIPPING)?;

    register_functions!(
        m,
        clip_line_segment_py,
        clip_line_segment_to_regions_py,
        subtract_polygons_from_line_segment_py,
        to_clipper_py,
        from_clipper_py,
    );

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def clip_line_segment_with_rect(
        p1: types.Point3D,
        p2: types.Point3D,
        rect: types.Rect,
    ) -> typing.Optional[tuple[types.Point3D, types.Point3D]]:
        """Clip a line segment with a rectangle.

        :param p1: Start point of the line segment.
        :param p2: End point of the line segment.
        :param rect: Clipping rectangle (x_min, y_min, x_max, y_max).
        :returns: Clipped segment or None if fully outside.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "clip_line_segment_with_rect")]
fn clip_line_segment_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    rect: (f64, f64, f64, f64),
) -> Option<Segment3D> {
    clip_line_segment_with_rect(p1, p2, rect)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def subtract_polygons_from_line_segment(
        p1: types.Point3D,
        p2: types.Point3D,
        regions: collections.abc.Sequence[collections.abc.Sequence[types.Point]],
    ) -> list[tuple[types.Point3D, types.Point3D]]:
        """Subtract polygon regions from a line segment.

        :param p1: Start point of the line segment.
        :param p2: End point of the line segment.
        :param regions: List of polygon regions to subtract.
        :returns: List of remaining segments after subtraction.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "subtract_polygons_from_line_segment")]
fn subtract_polygons_from_line_segment_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    regions: &Bound<'_, PyAny>,
) -> PyResult<Vec<Segment3D>> {
    let regions = extract_polygons(regions)?;
    Ok(subtract_polygons_from_line_segment(p1, p2, &regions))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def clip_line_segment_with_polygons(
        p1: types.Point3D,
        p2: types.Point3D,
        regions: collections.abc.Sequence[collections.abc.Sequence[types.Point]],
    ) -> list[tuple[types.Point3D, types.Point3D]]:
        """Clip line segments that fall within polygon regions.

        :param p1: Start point of the line segment.
        :param p2: End point of the line segment.
        :param regions: Polygon regions to clip against.
        :returns: List of clipped segments.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "clip_line_segment_with_polygons")]
fn clip_line_segment_to_regions_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    regions: &Bound<'_, PyAny>,
) -> PyResult<Vec<Segment3D>> {
    let regions = extract_polygons(regions)?;
    Ok(clip_line_segment_with_polygons(p1, p2, &regions))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def to_clipper(
        polygon: types.Polygon,
    ) -> list[tuple[int, int]]:
        """Convert a polygon to Clipper coordinates.

        :param polygon: Input polygon as a list of (x, y) points.
        :returns: Polygon with integer coordinates for Clipper.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "to_clipper")]
fn to_clipper_py(polygon: &Bound<'_, PyAny>) -> PyResult<Vec<(i64, i64)>> {
    let poly = extract_polygon(polygon)?;
    Ok(poly
        .iter()
        .map(|(x, y)| {
            (
                (x * crate::CLIPPER_SCALE) as i64,
                (y * crate::CLIPPER_SCALE) as i64,
            )
        })
        .collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def from_clipper(
        polygon: types.IntPolygon,
    ) -> types.Polygon:
        """Convert a polygon from Clipper coordinates.

        :param polygon: Integer polygon from Clipper.
        :returns: Polygon with float coordinates.
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "from_clipper")]
fn from_clipper_py(
    polygon: Vec<crate::python::geo::flex_point::PyIntPoint2D>,
) -> Vec<Point> {
    let scale = crate::CLIPPER_SCALE;
    let poly = int_poly_to_points(polygon);
    poly.iter()
        .map(|(x, y)| (*x as f64 / scale, *y as f64 / scale))
        .collect()
}
