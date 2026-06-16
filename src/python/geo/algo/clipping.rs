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
    edge_pairs3d_to_tuples, edge_pairs_to_tuples, extract_polygon,
    extract_polygons, point3d_to_tuple, points_to_tuples, tuple_to_point3d,
};
use super::super::types::{Edge2D, Edge3D};
use crate::geo::algo::clipping::{
    clip_line_segment_with_polygons, clip_line_segment_with_polygons_2d,
    clip_line_segment_with_rect, clip_line_segment_with_rect_2d,
    subtract_polygons_from_line_segment,
    subtract_polygons_from_line_segment_2d,
};
use crate::types::{Point, Rect};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "clipping")?;
    m.setattr("__doc__", MODULE_DOC_CLIPPING)?;

    register_functions!(
        m,
        clip_line_segment_py,
        clip_line_segment_to_regions_py,
        subtract_polygons_from_line_segment_py,
        clip_line_segment_with_rect_2d_py,
        clip_line_segment_with_polygons_2d_py,
        subtract_polygons_from_line_segment_2d_py,
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
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "clip_line_segment_with_rect")]
fn clip_line_segment_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    rect: (f64, f64, f64, f64),
) -> Option<Edge3D> {
    clip_line_segment_with_rect(
        tuple_to_point3d(p1),
        tuple_to_point3d(p2),
        Rect(rect.0, rect.1, rect.2, rect.3),
    )
    .map(|(a, b)| (point3d_to_tuple(a), point3d_to_tuple(b)))
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
        :complexity: O(n * m) time, O(n) space where n is the number of regions and m their average vertex count
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "subtract_polygons_from_line_segment")]
fn subtract_polygons_from_line_segment_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    regions: &Bound<'_, PyAny>,
) -> PyResult<Vec<Edge3D>> {
    let regions = extract_polygons(regions)?;
    Ok(edge_pairs3d_to_tuples(subtract_polygons_from_line_segment(
        tuple_to_point3d(p1),
        tuple_to_point3d(p2),
        &regions,
    )))
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
        :complexity: O(n * m) time, O(n) space where n is the number of regions and m their average vertex count
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "clip_line_segment_with_polygons")]
fn clip_line_segment_to_regions_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    regions: &Bound<'_, PyAny>,
) -> PyResult<Vec<Edge3D>> {
    let regions = extract_polygons(regions)?;
    Ok(edge_pairs3d_to_tuples(clip_line_segment_with_polygons(
        tuple_to_point3d(p1),
        tuple_to_point3d(p2),
        &regions,
    )))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def clip_line_segment_with_rect_2d(
        p1: types.Point,
        p2: types.Point,
        rect: types.Rect,
    ) -> typing.Optional[tuple[types.Point, types.Point]]:
        """Clip a 2D line segment with a rectangle (XY-plane only).

        :param p1: Start point (x, y).
        :param p2: End point (x, y).
        :param rect: Clipping rectangle (x_min, y_min, x_max, y_max).
        :returns: Clipped segment or None if fully outside.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "clip_line_segment_with_rect_2d")]
fn clip_line_segment_with_rect_2d_py(
    p1: (f64, f64),
    p2: (f64, f64),
    rect: (f64, f64, f64, f64),
) -> Option<Edge2D> {
    let p1 = Point::new(p1.0, p1.1);
    let p2 = Point::new(p2.0, p2.1);
    clip_line_segment_with_rect_2d(p1, p2, Rect(rect.0, rect.1, rect.2, rect.3))
        .map(|(a, b)| ((a.x, a.y), (b.x, b.y)))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def clip_line_segment_with_polygons_2d(
        p1: types.Point,
        p2: types.Point,
        regions: collections.abc.Sequence[collections.abc.Sequence[types.Point]],
    ) -> list[tuple[types.Point, types.Point]]:
        """Clip 2D line segments that fall within polygon regions (XY-plane only).

        :param p1: Start point (x, y).
        :param p2: End point (x, y).
        :param regions: Polygon regions to clip against.
        :returns: List of clipped segments.
        :complexity: O(n * m) time, O(n) space where n is the number of regions and m their average vertex count
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "clip_line_segment_with_polygons_2d")]
fn clip_line_segment_with_polygons_2d_py(
    p1: (f64, f64),
    p2: (f64, f64),
    regions: &Bound<'_, PyAny>,
) -> PyResult<Vec<Edge2D>> {
    let p1 = Point::new(p1.0, p1.1);
    let p2 = Point::new(p2.0, p2.1);
    let regions = extract_polygons(regions)?;
    Ok(edge_pairs_to_tuples(clip_line_segment_with_polygons_2d(
        p1, p2, &regions,
    )))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def subtract_polygons_from_line_segment_2d(
        p1: types.Point,
        p2: types.Point,
        regions: collections.abc.Sequence[collections.abc.Sequence[types.Point]],
    ) -> list[tuple[types.Point, types.Point]]:
        """Subtract polygon regions from a 2D line segment (XY-plane only).

        :param p1: Start point (x, y).
        :param p2: End point (x, y).
        :param regions: List of polygon regions to subtract.
        :returns: List of remaining segments after subtraction.
        :complexity: O(n * m) time, O(n) space where n is the number of regions and m their average vertex count
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "subtract_polygons_from_line_segment_2d")]
fn subtract_polygons_from_line_segment_2d_py(
    p1: (f64, f64),
    p2: (f64, f64),
    regions: &Bound<'_, PyAny>,
) -> PyResult<Vec<Edge2D>> {
    let p1 = Point::new(p1.0, p1.1);
    let p2 = Point::new(p2.0, p2.1);
    let regions = extract_polygons(regions)?;
    Ok(edge_pairs_to_tuples(
        subtract_polygons_from_line_segment_2d(p1, p2, &regions),
    ))
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
        :complexity: O(n) time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "to_clipper")]
fn to_clipper_py(polygon: &Bound<'_, PyAny>) -> PyResult<Vec<(i64, i64)>> {
    let poly = extract_polygon(polygon)?;
    Ok(poly
        .iter()
        .map(|p| {
            (
                (p.x * crate::CLIPPER_SCALE) as i64,
                (p.y * crate::CLIPPER_SCALE) as i64,
            )
        })
        .collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def from_clipper(
        polygon: list[tuple[int, int]],
    ) -> list[tuple[float, float]]:
        """Convert a polygon from Clipper coordinates.

        :param polygon: Integer polygon from Clipper.
        :returns: Polygon with float coordinates.
        :complexity: O(n) time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.clipping"
)]
#[pyfunction(name = "from_clipper")]
fn from_clipper_py(polygon: Vec<(i64, i64)>) -> Vec<(f64, f64)> {
    let scale = crate::CLIPPER_SCALE;
    points_to_tuples(
        polygon
            .iter()
            .map(|(x, y)| Point::new(*x as f64 / scale, *y as f64 / scale))
            .collect(),
    )
}
