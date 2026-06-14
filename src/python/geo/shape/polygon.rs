//! Python bindings for polygon operations.

use super::super::flex_point::{extract_polygons, poly_to_points, PyPoint2D};
use crate::geo::shape::polygon::{
    clean_polygon, flip_polygon, flip_polygons, get_polygon_bounds,
    get_polygon_centroid, get_polygon_convex_hull, get_polygon_edges,
    get_polygon_group_bounds, get_polygon_perimeter, get_polygon_signed_area,
    get_polygons_difference, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_intersection,
    get_polygons_union, is_almost_equal, is_point_inside_polygon,
    is_polygon_clockwise, is_polygon_convex, normalize_polygons,
    offset_polygon, point_line_distance, polygons_intersect, rotate_polygon,
    rotate_polygons, scale_polygon, translate_bounds, translate_polygon,
    translate_polygons,
};
use crate::types::{Point, Rect};
use numpy::{PyArray2, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList};
use pyo3_stub_gen::derive::gen_stub_pyfunction;

// -- numpy wrapper helpers --

fn _polygon_from_numpy(arr: &Bound<'_, PyArray2<f64>>) -> Vec<Point> {
    let readonly = arr.readonly();
    let view = readonly.as_array();
    view.rows()
        .into_iter()
        .map(|row| Point(row[0], row[1]))
        .collect()
}

fn _polygon_to_numpy(py: Python<'_>, poly: Vec<Point>) -> Py<PyAny> {
    let vecs: Vec<Vec<f64>> =
        poly.into_iter().map(|p| vec![p.0, p.1]).collect();
    let np_arr = PyArray2::<f64>::from_vec2(py, &vecs)
        .expect("failed to create numpy array");
    np_arr.into_any().unbind()
}

fn _polygons_from_numpy_list(
    polys: Vec<Bound<'_, PyArray2<f64>>>,
) -> Vec<Vec<Point>> {
    polys.into_iter().map(|a| _polygon_from_numpy(&a)).collect()
}

fn _polygons_to_numpy_list(
    py: Python<'_>,
    polys: Vec<Vec<Point>>,
) -> Vec<Py<PyAny>> {
    polys
        .into_iter()
        .map(|p| _polygon_to_numpy(py, p))
        .collect()
}

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "polygon")?;

    register_functions!(
        m,
        clean_polygon_py,
        flip_polygon_numpy_py,
        flip_polygon_py,
        flip_polygons_numpy_py,
        flip_polygons_py,
        get_polygon_area_py,
        get_polygon_bounds_py,
        get_polygon_centroid_py,
        get_polygon_convex_hull_py,
        get_polygon_edges_py,
        get_polygon_group_bounds_py,
        get_polygon_perimeter_py,
        get_polygon_signed_area_py,
        get_polygons_difference_py,
        get_polygons_group_difference_py,
        get_polygons_group_intersection_py,
        get_polygons_intersection_py,
        get_polygons_union_py,
        is_almost_equal_py,
        is_point_inside_polygon_py,
        is_polygon_clockwise_py,
        is_polygon_convex_py,
        normalize_polygons_numpy_py,
        normalize_polygons_py,
        offset_polygon_py,
        point_in_polygon_numpy_py,
        point_line_distance_py,
        polygon_area_numpy_py,
        polygon_bounds_numpy_py,
        polygon_group_bounds_numpy_py,
        polygon_perimeter_numpy_py,
        polygons_intersect_numpy_py,
        polygons_intersect_py,
        rotate_polygon_numpy_py,
        rotate_polygon_py,
        rotate_polygons_numpy_py,
        rotate_polygons_py,
        scale_polygon_py,
        to_clipper_numpy_py,
        translate_bounds_py,
        translate_polygon_numpy_py,
        translate_polygon_py,
        translate_polygons_numpy_py,
        translate_polygons_py,
    );

    shape_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def clean_polygon(
        polygon: collections.abc.Sequence[types.Point],
        tolerance: typing.Optional[float] = None,
    ) -> typing.Optional[types.Polygon]:
        """Clean a polygon by removing near-duplicate points.

        :param polygon: Input polygon as (x, y) points.
        :param tolerance: Distance tolerance for deduplication.
        :returns: Cleaned polygon or None.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "clean_polygon")]
#[pyo3(signature = (polygon, tolerance=None))]
fn clean_polygon_py(
    polygon: Vec<PyPoint2D>,
    tolerance: Option<f64>,
) -> Option<Vec<Point>> {
    clean_polygon(&poly_to_points(polygon), tolerance.unwrap_or(1e-6))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def is_almost_equal(
        a: float,
        b: float,
        tolerance: typing.Optional[float] = None,
    ) -> bool:
        """Check if two floats are almost equal.

        :param a: First float.
        :param b: Second float.
        :param tolerance: Comparison tolerance.
        :returns: True if |a - b| < tolerance.
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "is_almost_equal")]
#[pyo3(signature = (a, b, tolerance=None))]
fn is_almost_equal_py(a: f64, b: f64, tolerance: Option<f64>) -> bool {
    is_almost_equal(a, b, tolerance.unwrap_or(1e-9))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def normalize_polygons(polygons: typing.Any) -> tuple[list[types.Polygon], float, float]:
        """Normalize polygons (outer CCW, inner CW).

        :param polygons: List of polygons to normalize.
        :returns: Tuple of (normalized_polygons, min_x, min_y).
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "normalize_polygons")]
fn normalize_polygons_py(
    polygons: &Bound<'_, PyAny>,
) -> PyResult<(Vec<Vec<Point>>, f64, f64)> {
    let p = extract_polygons(polygons)?;
    Ok(normalize_polygons(&p))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def translate_bounds(
        bounds: types.Rect,
        dx: float,
        dy: float,
    ) -> types.Rect:
        """Translate a bounding rectangle.

        :param bounds: Bounding rectangle (x_min, y_min, x_max, y_max).
        :param dx: X translation.
        :param dy: Y translation.
        :returns: Translated bounding rectangle.
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_bounds")]
fn translate_bounds_py(
    bounds: (f64, f64, f64, f64),
    dx: f64,
    dy: f64,
) -> (f64, f64, f64, f64) {
    let r =
        translate_bounds(Rect(bounds.0, bounds.1, bounds.2, bounds.3), dx, dy);
    (r.0, r.1, r.2, r.3)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def translate_polygons(polygons: typing.Any, dx: float, dy: float) -> list[types.Polygon]:
        """Translate a list of polygons.

        :param polygons: List of polygons to translate.
        :param dx: X translation.
        :param dy: Y translation.
        :returns: Translated polygons.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_polygons")]
fn translate_polygons_py(
    polygons: &Bound<'_, PyAny>,
    dx: f64,
    dy: f64,
) -> PyResult<Vec<Vec<Point>>> {
    let p = extract_polygons(polygons)?;
    Ok(translate_polygons(&p, dx, dy))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def point_line_distance(
        point: types.Point,
        line_start: types.Point,
        line_end: types.Point,
    ) -> float:
        """Compute the distance from a point to a line.

        :param point: Point (x, y).
        :param line_start: Line start point (x, y).
        :param line_end: Line end point (x, y).
        :returns: Perpendicular distance.
        :complexity: O(1)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "point_line_distance")]
fn point_line_distance_py(
    point: Point,
    line_start: Point,
    line_end: Point,
) -> f64 {
    point_line_distance(point, line_start, line_end)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_area(polygon: collections.abc.Sequence[types.Point]) -> float:
        """Get the unsigned area of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Unsigned area.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_area")]
fn get_polygon_area_py(polygon: Vec<PyPoint2D>) -> f64 {
    get_polygon_signed_area(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_signed_area(
        polygon: collections.abc.Sequence[types.Point],
    ) -> float:
        """Get the signed area of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Signed area (positive for CCW, negative for CW).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_signed_area")]
fn get_polygon_signed_area_py(polygon: Vec<PyPoint2D>) -> f64 {
    get_polygon_signed_area(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_perimeter(
        polygon: collections.abc.Sequence[types.Point],
    ) -> float:
        """Get the perimeter of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Perimeter length.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_perimeter")]
fn get_polygon_perimeter_py(polygon: Vec<PyPoint2D>) -> f64 {
    get_polygon_perimeter(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_bounds(
        polygon: collections.abc.Sequence[types.Point],
    ) -> types.Rect:
        """Get the bounding rectangle of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_bounds")]
fn get_polygon_bounds_py(polygon: Vec<PyPoint2D>) -> (f64, f64, f64, f64) {
    let r = get_polygon_bounds(&poly_to_points(polygon));
    (r.0, r.1, r.2, r.3)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygon_group_bounds(
        polygons: typing.Any,
    ) -> types.Rect:
        """Get the bounding rectangle of a group of polygons.

        :param polygons: List of polygons.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_group_bounds")]
fn get_polygon_group_bounds_py(
    polygons: &Bound<'_, PyAny>,
) -> PyResult<(f64, f64, f64, f64)> {
    let p = extract_polygons(polygons)?;
    let r = get_polygon_group_bounds(&p);
    Ok((r.0, r.1, r.2, r.3))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_centroid(
        polygon: collections.abc.Sequence[types.Point],
    ) -> types.Point:
        """Get the centroid of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Centroid point (x, y).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_centroid")]
fn get_polygon_centroid_py(polygon: Vec<PyPoint2D>) -> Point {
    get_polygon_centroid(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def is_polygon_convex(
        polygon: collections.abc.Sequence[types.Point],
    ) -> bool:
        """Check if a polygon is convex.

        :param polygon: Polygon as (x, y) points.
        :returns: True if the polygon is convex.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "is_polygon_convex")]
fn is_polygon_convex_py(polygon: Vec<PyPoint2D>) -> bool {
    is_polygon_convex(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_convex_hull(
        polygon: collections.abc.Sequence[types.Point],
    ) -> types.Polygon:
        """Get the convex hull of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Convex hull as list of points.
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_convex_hull")]
fn get_polygon_convex_hull_py(polygon: Vec<PyPoint2D>) -> Vec<Point> {
    get_polygon_convex_hull(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_edges(
        polygon: collections.abc.Sequence[types.Point],
    ) -> list[tuple[types.Point, types.Point]]:
        """Get the edges of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: List of ((x1, y1), (x2, y2)) edges.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_edges")]
fn get_polygon_edges_py(polygon: Vec<PyPoint2D>) -> Vec<(Point, Point)> {
    get_polygon_edges(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def is_point_inside_polygon(
        point: types.Point,
        polygon: collections.abc.Sequence[types.Point],
    ) -> bool:
        """Check if a point is inside a polygon.

        :param point: Point (x, y) to test.
        :param polygon: Polygon as (x, y) points.
        :returns: True if point is inside the polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "is_point_inside_polygon")]
fn is_point_inside_polygon_py(point: Point, polygon: Vec<PyPoint2D>) -> bool {
    is_point_inside_polygon(point, &poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def offset_polygon(
        polygon: collections.abc.Sequence[types.Point],
        offset: float,
    ) -> list[types.Polygon]:
        """Offset (inflate/deflate) a polygon.

        :param polygon: Polygon as (x, y) points.
        :param offset: Offset distance (positive to inflate, negative to deflate).
        :returns: Offset polygon(s).
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "offset_polygon")]
fn offset_polygon_py(polygon: Vec<PyPoint2D>, offset: f64) -> Vec<Vec<Point>> {
    offset_polygon(&poly_to_points(polygon), offset)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygons_union(polygons: typing.Any) -> list[types.Polygon]:
        """Get the union of multiple polygons.

        :param polygons: List of polygons to union.
        :returns: Union polygon(s).
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_union")]
fn get_polygons_union_py(
    polygons: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<Point>>> {
    let p = extract_polygons(polygons)?;
    Ok(get_polygons_union(&p))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_intersection(
        poly1: collections.abc.Sequence[types.Point],
        poly2: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Get the intersection of two polygons.

        :param poly1: First polygon as (x, y) points.
        :param poly2: Second polygon as (x, y) points.
        :returns: Intersection polygon(s).
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_intersection")]
fn get_polygons_intersection_py(
    poly1: Vec<PyPoint2D>,
    poly2: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    get_polygons_intersection(&poly_to_points(poly1), &poly_to_points(poly2))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_difference(
        poly1: collections.abc.Sequence[types.Point],
        poly2: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Get the difference of two polygons.

        :param poly1: First polygon as (x, y) points.
        :param poly2: Second polygon to subtract.
        :returns: Difference polygon(s).
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_difference")]
fn get_polygons_difference_py(
    poly1: Vec<PyPoint2D>,
    poly2: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    get_polygons_difference(&poly_to_points(poly1), &poly_to_points(poly2))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygons_group_intersection(
        subject: typing.Sequence[types.Polygon],
        clip: typing.Sequence[types.Polygon],
    ) -> list[types.Polygon]:
        """Intersect two groups of polygons (subject & clip).

        :param subject: Subject polygons.
        :param clip: Clip polygons.
        :returns: Intersection polygon(s).
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_group_intersection")]
fn get_polygons_group_intersection_py(
    subject: &Bound<'_, PyAny>,
    clip: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<Point>>> {
    let subject_polys = extract_polygons(subject)?;
    let clip_polys = extract_polygons(clip)?;
    Ok(get_polygons_group_intersection(&subject_polys, &clip_polys))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygons_group_difference(
        subject: typing.Sequence[types.Polygon],
        clip: typing.Sequence[types.Polygon],
    ) -> list[types.Polygon]:
        """Subtract clip polygons from subject polygons.

        :param subject: Subject polygons.
        :param clip: Clip polygons to subtract.
        :returns: Difference polygon(s).
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_group_difference")]
fn get_polygons_group_difference_py(
    subject: &Bound<'_, PyAny>,
    clip: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<Point>>> {
    let subject_polys = extract_polygons(subject)?;
    let clip_polys = extract_polygons(clip)?;
    Ok(get_polygons_group_difference(&subject_polys, &clip_polys))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def polygons_intersect(
        p1: collections.abc.Sequence[types.Point],
        p2: collections.abc.Sequence[types.Point],
        min_area: float = 0.0,
    ) -> bool:
        """Check if two polygons intersect.

        :param p1: First polygon as (x, y) points.
        :param p2: Second polygon as (x, y) points.
        :param min_area: Minimum intersection area threshold.
        :returns: True if polygons intersect.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygons_intersect")]
#[pyo3(signature = (p1, p2, min_area=0.0))]
fn polygons_intersect_py(
    p1: Vec<PyPoint2D>,
    p2: Vec<PyPoint2D>,
    min_area: f64,
) -> bool {
    polygons_intersect(&poly_to_points(p1), &poly_to_points(p2), min_area)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def flip_polygon(
        polygon: collections.abc.Sequence[types.Point],
        flip_h: bool,
        flip_v: bool,
    ) -> types.Polygon:
        """Flip a polygon horizontally and/or vertically.

        :param polygon: Polygon as (x, y) points.
        :param flip_h: Whether to flip horizontally.
        :param flip_v: Whether to flip vertically.
        :returns: Flipped polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "flip_polygon")]
fn flip_polygon_py(
    polygon: Vec<PyPoint2D>,
    flip_h: bool,
    flip_v: bool,
) -> Vec<Point> {
    flip_polygon(&poly_to_points(polygon), flip_h, flip_v)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def flip_polygons(
        polygons: typing.Any,
        flip_h: bool,
        flip_v: bool,
    ) -> list[types.Polygon]:
        """Flip multiple polygons.

        :param polygons: List of polygons to flip.
        :param flip_h: Whether to flip horizontally.
        :param flip_v: Whether to flip vertically.
        :returns: Flipped polygons.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "flip_polygons")]
fn flip_polygons_py(
    polygons: &Bound<'_, PyAny>,
    flip_h: bool,
    flip_v: bool,
) -> PyResult<Vec<Vec<Point>>> {
    let p = extract_polygons(polygons)?;
    Ok(flip_polygons(&p, flip_h, flip_v))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def rotate_polygon(
        polygon: collections.abc.Sequence[types.Point],
        angle: float,
    ) -> types.Polygon:
        """Rotate a polygon by an angle.

        :param polygon: Polygon as (x, y) points.
        :param angle: Rotation angle in degrees.
        :returns: Rotated polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "rotate_polygon")]
fn rotate_polygon_py(polygon: Vec<PyPoint2D>, angle: f64) -> Vec<Point> {
    rotate_polygon(&poly_to_points(polygon), angle)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def rotate_polygons(polygons: typing.Any, angle: float) -> list[types.Polygon]:
        """Rotate multiple polygons by an angle.

        :param polygons: List of polygons to rotate.
        :param angle: Rotation angle in degrees.
        :returns: Rotated polygons.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "rotate_polygons")]
fn rotate_polygons_py(
    polygons: &Bound<'_, PyAny>,
    angle: f64,
) -> PyResult<Vec<Vec<Point>>> {
    let p = extract_polygons(polygons)?;
    Ok(rotate_polygons(&p, angle))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def scale_polygon(
        polygon: collections.abc.Sequence[types.Point],
        scale: float,
        scale_y: typing.Optional[float] = None,
    ) -> types.Polygon:
        """Scale a polygon.

        :param polygon: Polygon as (x, y) points.
        :param scale: X (and Y if scale_y is None) scale factor.
        :param scale_y: Y scale factor (optional).
        :returns: Scaled polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "scale_polygon")]
#[pyo3(signature = (polygon, scale, scale_y=None))]
fn scale_polygon_py(
    polygon: Vec<PyPoint2D>,
    scale: f64,
    scale_y: Option<f64>,
) -> Vec<Point> {
    scale_polygon(&poly_to_points(polygon), scale, scale_y)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def translate_polygon(
        polygon: collections.abc.Sequence[types.Point],
        dx: float,
        dy: float,
    ) -> types.Polygon:
        """Translate a polygon.

        :param polygon: Polygon as (x, y) points.
        :param dx: X translation.
        :param dy: Y translation.
        :returns: Translated polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_polygon")]
fn translate_polygon_py(
    polygon: Vec<PyPoint2D>,
    dx: f64,
    dy: f64,
) -> Vec<Point> {
    translate_polygon(&poly_to_points(polygon), dx, dy)
}

// -- numpy variants --

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing

    def polygon_area_numpy(polygon: numpy.typing.NDArray) -> float:
        """Get the area of a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :returns: Signed area.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygon_area_numpy")]
fn polygon_area_numpy_py(polygon: Bound<'_, PyArray2<f64>>) -> f64 {
    let p = _polygon_from_numpy(&polygon);
    get_polygon_signed_area(&p)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing
    import raygeo.geo.types

    def polygon_bounds_numpy(
        polygon: numpy.typing.NDArray,
    ) -> types.Rect:
        """Get bounds of a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygon_bounds_numpy")]
fn polygon_bounds_numpy_py(
    polygon: Bound<'_, PyArray2<f64>>,
) -> (f64, f64, f64, f64) {
    let p = _polygon_from_numpy(&polygon);
    let r = get_polygon_bounds(&p);
    (r.0, r.1, r.2, r.3)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing

    def polygon_perimeter_numpy(polygon: numpy.typing.NDArray) -> float:
        """Get the perimeter of a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :returns: Perimeter length.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygon_perimeter_numpy")]
fn polygon_perimeter_numpy_py(polygon: Bound<'_, PyArray2<f64>>) -> f64 {
    let p = _polygon_from_numpy(&polygon);
    get_polygon_perimeter(&p)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import numpy.typing
    import raygeo.geo.types

    def polygon_group_bounds_numpy(
        polygons: collections.abc.Sequence[numpy.typing.NDArray],
    ) -> types.Rect:
        """Get bounds of polygon group from numpy arrays.

        :param polygons: Sequence of 2D numpy arrays.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygon_group_bounds_numpy")]
fn polygon_group_bounds_numpy_py(
    polygons: Vec<Bound<'_, PyArray2<f64>>>,
) -> (f64, f64, f64, f64) {
    let p = _polygons_from_numpy_list(polygons);
    let r = get_polygon_group_bounds(&p);
    (r.0, r.1, r.2, r.3)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import numpy.typing

    def flip_polygon_numpy(
        polygon: numpy.typing.NDArray,
        flip_h: bool,
        flip_v: bool,
    ) -> typing.Any:
        """Flip a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :param flip_h: Whether to flip horizontally.
        :param flip_v: Whether to flip vertically.
        :returns: Flipped polygon as numpy array.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "flip_polygon_numpy")]
fn flip_polygon_numpy_py(
    py: Python<'_>,
    polygon: Bound<'_, PyArray2<f64>>,
    flip_h: bool,
    flip_v: bool,
) -> Py<PyAny> {
    let p = _polygon_from_numpy(&polygon);
    let result = flip_polygon(&p, flip_h, flip_v);
    _polygon_to_numpy(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def flip_polygons_numpy(polygons: list, flip_h: bool, flip_v: bool) -> typing.Any:
        """Flip polygons from numpy arrays.

        :param polygons: List of 2D numpy arrays.
        :param flip_h: Whether to flip horizontally.
        :param flip_v: Whether to flip vertically.
        :returns: List of flipped numpy arrays.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "flip_polygons_numpy")]
fn flip_polygons_numpy_py<'py>(
    py: Python<'py>,
    polygons: Bound<'py, PyList>,
    flip_h: bool,
    flip_v: bool,
) -> PyResult<Bound<'py, PyAny>> {
    if !flip_h && !flip_v {
        return Ok(polygons.as_any().clone());
    }
    let mut p = Vec::new();
    for item in polygons.iter() {
        let arr = item.cast::<PyArray2<f64>>()?;
        p.push(_polygon_from_numpy(arr));
    }
    let result = flip_polygons(&p, flip_h, flip_v);
    let np_list = _polygons_to_numpy_list(py, result);
    Ok(PyList::new(py, np_list)?.as_any().clone())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import numpy.typing

    def normalize_polygons_numpy(
        polygons: collections.abc.Sequence[numpy.typing.NDArray],
    ) -> tuple[list[numpy.typing.NDArray], float, float]:
        """Normalize polygons from numpy arrays.

        :param polygons: Sequence of 2D numpy arrays.
        :returns: Tuple of (normalized_arrays, min_x, min_y).
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "normalize_polygons_numpy")]
fn normalize_polygons_numpy_py(
    py: Python<'_>,
    polygons: Vec<Bound<'_, PyArray2<f64>>>,
) -> (Vec<Py<PyAny>>, f64, f64) {
    let p = _polygons_from_numpy_list(polygons);
    let (result, min_x, min_y) = normalize_polygons(&p);
    let result_np = _polygons_to_numpy_list(py, result);
    (result_np, min_x, min_y)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing
    import raygeo.geo.types

    def point_in_polygon_numpy(
        point: types.Point,
        polygon: numpy.typing.NDArray,
    ) -> bool:
        """Check if point is in polygon from numpy array.

        :param point: Point (x, y) to test.
        :param polygon: Polygon as a 2D numpy array.
        :returns: True if point is inside the polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "point_in_polygon_numpy")]
fn point_in_polygon_numpy_py(
    point: Point,
    polygon: Bound<'_, PyArray2<f64>>,
) -> bool {
    let p = _polygon_from_numpy(&polygon);
    is_point_inside_polygon(point, &p)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing

    def polygons_intersect_numpy(
        poly1: numpy.typing.NDArray,
        poly2: numpy.typing.NDArray,
        min_area: float = 0.0,
    ) -> bool:
        """Check if polygons intersect from numpy arrays.

        :param poly1: First polygon as a 2D numpy array.
        :param poly2: Second polygon as a 2D numpy array.
        :param min_area: Minimum intersection area threshold.
        :returns: True if polygons intersect.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygons_intersect_numpy")]
#[pyo3(signature = (poly1, poly2, min_area=0.0))]
fn polygons_intersect_numpy_py(
    poly1: Bound<'_, PyArray2<f64>>,
    poly2: Bound<'_, PyArray2<f64>>,
    min_area: f64,
) -> bool {
    let p1 = _polygon_from_numpy(&poly1);
    let p2 = _polygon_from_numpy(&poly2);
    polygons_intersect(&p1, &p2, min_area)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import numpy.typing

    def rotate_polygon_numpy(
        polygon: numpy.typing.NDArray,
        angle: float,
    ) -> typing.Any:
        """Rotate a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :param angle: Rotation angle in degrees.
        :returns: Rotated polygon as numpy array.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "rotate_polygon_numpy")]
fn rotate_polygon_numpy_py(
    py: Python<'_>,
    polygon: Bound<'_, PyArray2<f64>>,
    angle: f64,
) -> Py<PyAny> {
    let p = _polygon_from_numpy(&polygon);
    let result = rotate_polygon(&p, angle);
    _polygon_to_numpy(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import numpy.typing

    def rotate_polygons_numpy(
        polygons: collections.abc.Sequence[numpy.typing.NDArray],
        angle: float,
    ) -> typing.Any:
        """Rotate polygons from numpy arrays.

        :param polygons: Sequence of 2D numpy arrays.
        :param angle: Rotation angle in degrees.
        :returns: List of rotated numpy arrays.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "rotate_polygons_numpy")]
fn rotate_polygons_numpy_py(
    py: Python<'_>,
    polygons: Vec<Bound<'_, PyArray2<f64>>>,
    angle: f64,
) -> Vec<Py<PyAny>> {
    let p = _polygons_from_numpy_list(polygons);
    let result = rotate_polygons(&p, angle);
    _polygons_to_numpy_list(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing

    def translate_polygon_numpy(
        polygon: numpy.typing.NDArray,
        dx: float,
        dy: float,
    ) -> typing.Any:
        """Translate a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :param dx: X translation.
        :param dy: Y translation.
        :returns: Translated polygon as numpy array.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_polygon_numpy")]
fn translate_polygon_numpy_py(
    py: Python<'_>,
    polygon: Bound<'_, PyArray2<f64>>,
    dx: f64,
    dy: f64,
) -> Py<PyAny> {
    let p = _polygon_from_numpy(&polygon);
    let result = translate_polygon(&p, dx, dy);
    _polygon_to_numpy(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import numpy.typing

    def translate_polygons_numpy(
        polygons: collections.abc.Sequence[numpy.typing.NDArray],
        dx: float,
        dy: float,
    ) -> typing.Any:
        """Translate polygons from numpy arrays.

        :param polygons: Sequence of 2D numpy arrays.
        :param dx: X translation.
        :param dy: Y translation.
        :returns: List of translated numpy arrays.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_polygons_numpy")]
fn translate_polygons_numpy_py(
    py: Python<'_>,
    polygons: Vec<Bound<'_, PyArray2<f64>>>,
    dx: f64,
    dy: f64,
) -> Vec<Py<PyAny>> {
    let p = _polygons_from_numpy_list(polygons);
    let result = translate_polygons(&p, dx, dy);
    _polygons_to_numpy_list(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import numpy.typing

    def to_clipper_numpy(polygon: collections.abc.Sequence[numpy.typing.NDArray]) -> list[tuple[int, int]]:
        """Convert a numpy polygon to Clipper integer coordinates.

        :param polygon: Sequence of 2D numpy arrays.
        :returns: List of (x, y) integer tuples.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "to_clipper_numpy")]
fn to_clipper_numpy_py(
    polygon: Vec<Bound<'_, PyArray2<f64>>>,
) -> Vec<Vec<(i64, i64)>> {
    let p = _polygons_from_numpy_list(polygon);
    p.into_iter()
        .map(|poly| {
            poly.into_iter()
                .map(|p| {
                    let scale = 10_000_000.0;
                    ((p.0 * scale) as i64, (p.1 * scale) as i64)
                })
                .collect()
        })
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def is_polygon_clockwise(
        points: collections.abc.Sequence[types.Point2DOr3D],
    ) -> bool:
        """Check if a polygon has clockwise winding.

        :param points: Sequence of (x, y) or (x, y, z) points.
        :returns: True if the polygon is clockwise.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "is_polygon_clockwise")]
fn is_polygon_clockwise_py(points: Vec<PyPoint2D>) -> bool {
    let points_2d: Vec<Point> =
        points.iter().map(|p| Point(p.0, p.1)).collect();
    is_polygon_clockwise(&points_2d)
}
