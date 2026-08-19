//! Python bindings for polygon operations.

use super::super::flex_point::{
    edge_pairs_to_tuples, extract_polygons, point_to_tuple, points_to_tuples,
    poly_to_points, polygons_to_tuples, PyPoint2D,
};
use super::super::types::NormalizePolygonsResult;
use crate::geo::matrix::Matrix as CoreMatrix;
use crate::geo::shape::polygon::{
    apply_minimum_curvature, clean_polygon, compute_polygon_bounds,
    do_polygons_intersect, does_path_sweep_intersect_polygon,
    does_polygon_enclose_circle, find_entry_edges, find_polygon_corners,
    flip_polygon, flip_polygons, get_circle_polygon,
    get_miter_offset_intersection, get_point_line_distance,
    get_polygon_boundary_distance, get_polygon_bounds, get_polygon_centroid,
    get_polygon_closest_point, get_polygon_convex_hull, get_polygon_edges,
    get_polygon_from_points, get_polygon_group_bounds, get_polygon_heading_at,
    get_polygon_perimeter, get_polygon_signed_area,
    get_polygon_vertex_centroid, get_polygons_closest_point,
    get_polygons_difference, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_intersection,
    get_polygons_union, get_polyline_swept_polygon, get_segment_swept_polygon,
    get_signed_boundary_distance, is_almost_equal,
    is_path_confined_to_boundary, is_point_inside_polygon,
    is_polygon_clockwise, is_polygon_convex, normalize_polygons,
    offset_polygon, resample_polygon, rotate_polygon, rotate_polygons,
    scale_polygon, transform_polygons, translate_bounds, translate_polygon,
    translate_polygons, CornerType, JoinStyle,
};
use crate::geo::types::{Point, Rect};
use crate::python::geo::matrix::Matrix as PyMatrix;
use numpy::{PyArray2, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList};
use pyo3_stub_gen::derive::{gen_stub_pyclass_enum, gen_stub_pyfunction};

// -- numpy wrapper helpers --

fn _polygon_from_numpy(arr: &Bound<'_, PyArray2<f64>>) -> Vec<Point> {
    let readonly = arr.readonly();
    let view = readonly.as_array();
    view.rows()
        .into_iter()
        .map(|row| Point::new(row[0], row[1]))
        .collect()
}

fn _polygon_to_numpy(py: Python<'_>, poly: Vec<Point>) -> Py<PyAny> {
    let vecs: Vec<Vec<f64>> =
        poly.into_iter().map(|p| vec![p.x, p.y]).collect();
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

#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.geo.shape.polygon",
    name = "JoinStyle",
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Corner join style for polygon offset operations.
///
/// - ``JoinStyle.MITER``: Extends edges until they meet (default).
/// - ``JoinStyle.ROUND``: Adds a circular arc at the corner.
/// - ``JoinStyle.SQUARE``: Extends edges by the offset distance.
pub enum PyJoinStyle {
    #[pyo3(name = "MITER")]
    Miter,
    #[pyo3(name = "ROUND")]
    Round,
    #[pyo3(name = "SQUARE")]
    Square,
}

impl From<PyJoinStyle> for JoinStyle {
    fn from(s: PyJoinStyle) -> Self {
        match s {
            PyJoinStyle::Miter => JoinStyle::Miter,
            PyJoinStyle::Round => JoinStyle::Round,
            PyJoinStyle::Square => JoinStyle::Square,
        }
    }
}

#[pymethods]
impl PyJoinStyle {
    fn __repr__(&self) -> String {
        match self {
            PyJoinStyle::Miter => "JoinStyle.MITER".to_string(),
            PyJoinStyle::Round => "JoinStyle.ROUND".to_string(),
            PyJoinStyle::Square => "JoinStyle.SQUARE".to_string(),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.geo.shape.polygon",
    name = "CornerType",
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
/// Which corner type to find in [`find_polygon_corners`].
///
/// - ``CornerType.CONVEX``: convex corners (interior angle < 180°).
/// - ``CornerType.CONCAVE``: concave / reflex corners (default).
pub enum PyCornerType {
    #[pyo3(name = "CONVEX")]
    Convex,
    #[default]
    #[pyo3(name = "CONCAVE")]
    Concave,
}

impl From<PyCornerType> for CornerType {
    fn from(c: PyCornerType) -> Self {
        match c {
            PyCornerType::Convex => CornerType::Convex,
            PyCornerType::Concave => CornerType::Concave,
        }
    }
}

#[pymethods]
impl PyCornerType {
    fn __repr__(&self) -> String {
        match self {
            PyCornerType::Convex => "CornerType.CONVEX".to_string(),
            PyCornerType::Concave => "CornerType.CONCAVE".to_string(),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "polygon")?;

    m.add_class::<PyJoinStyle>()?;
    m.add_class::<PyCornerType>()?;

    register_functions!(
        m,
        apply_minimum_curvature_py,
        clean_polygon_py,
        find_entry_edges_py,
        find_polygon_corners_py,
        does_path_sweep_intersect_polygon_py,
        does_polygon_enclose_circle_py,
        flip_polygon_numpy_py,
        flip_polygon_py,
        flip_polygons_numpy_py,
        flip_polygons_py,
        get_polygon_area_py,
        get_circle_polygon_py,
        get_polygon_bounds_py,
        get_polygon_boundary_distance_py,
        get_polygon_centroid_py,
        get_polygon_closest_point_py,
        get_polygon_from_points_py,
        get_polygon_vertex_centroid_py,
        get_polygons_closest_point_py,
        get_polyline_swept_polygon_py,
        get_segment_swept_polygon_py,
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
        get_signed_boundary_distance_py,
        is_almost_equal_py,
        is_path_confined_to_boundary_py,
        is_point_inside_polygon_py,
        is_polygon_clockwise_py,
        is_polygon_convex_py,
        miter_offset_intersection_py,
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
        resample_polygon_py,
        get_polygon_heading_at_py,
        walk_polygon_from_point_py,
        walk_polygon_vertices_py,
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
        transform_polygons_py,
    );

    shape_mod.add_submodule(&m)?;
    let sys_modules = shape_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.polygon", &m)?;
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
) -> Option<Vec<(f64, f64)>> {
    clean_polygon(&poly_to_points(polygon), tolerance.unwrap_or(1e-6))
        .map(points_to_tuples)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def get_polygon_from_points(
        points: collections.abc.Sequence[types.Point],
        tolerance: typing.Optional[float] = None,
    ) -> typing.Optional[types.Polygon]:
        """Convert a run of points into a cleaned polygon.

        :param points: Vertices as (x, y) points.
        :param tolerance: Cleaning tolerance.
        :returns: Cleaned polygon or the raw points if cleaning fails,
            or None for fewer than 3 points.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_from_points")]
#[pyo3(signature = (points, tolerance=None))]
fn get_polygon_from_points_py(
    points: Vec<PyPoint2D>,
    tolerance: Option<f64>,
) -> Option<Vec<(f64, f64)>> {
    get_polygon_from_points(&poly_to_points(points), tolerance.unwrap_or(1e-6))
        .map(points_to_tuples)
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

    def normalize_polygons(polygons: collections.abc.Sequence[types.Polygon]) -> tuple[list[types.Polygon], float, float]:
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
) -> PyResult<NormalizePolygonsResult> {
    let p = extract_polygons(polygons)?;
    let (result, min_x, min_y) = normalize_polygons(&p);
    Ok((polygons_to_tuples(result), min_x, min_y))
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
    let r = translate_bounds(
        Rect::new(bounds.0, bounds.1, bounds.2, bounds.3),
        dx,
        dy,
    );
    (r.min.x, r.min.y, r.max.x, r.max.y)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def translate_polygons(polygons: collections.abc.Sequence[types.Polygon], dx: float, dy: float) -> list[types.Polygon]:
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
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let p = extract_polygons(polygons)?;
    Ok(polygons_to_tuples(translate_polygons(&p, dx, dy)))
}

/// Extract a core 3x3 affine matrix from a `Matrix` or a 3x3/4x4
/// list-of-lists.
fn extract_core_matrix(obj: &Bound<'_, PyAny>) -> PyResult<CoreMatrix> {
    if let Ok(py_m) = obj.extract::<PyMatrix>() {
        return Ok(py_m.inner);
    }
    let rows = obj.extract::<Vec<Vec<f64>>>().map_err(|_| {
        pyo3::exceptions::PyTypeError::new_err(
            "expected a Matrix or a 3x3/4x4 list of lists",
        )
    })?;
    // 4x4 input: keep the 2D affine part (drop the Z row/col).
    let rows = if rows.len() == 4 {
        vec![
            vec![rows[0][0], rows[0][1], rows[0][3]],
            vec![rows[1][0], rows[1][1], rows[1][3]],
            vec![0.0, 0.0, 1.0],
        ]
    } else {
        rows
    };
    if rows.len() != 3 || rows.iter().any(|r| r.len() != 3) {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "expected a 3x3 or 4x4 matrix",
        ));
    }
    Ok(CoreMatrix::from_cols_arrays([
        [rows[0][0], rows[0][1], rows[0][2]],
        [rows[1][0], rows[1][1], rows[1][2]],
        [rows[2][0], rows[2][1], rows[2][2]],
    ]))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def transform_polygons(
        polygons: collections.abc.Sequence[types.Polygon],
        matrix: types.TransformMatrix,
    ) -> list[types.Polygon]:
        """Transform a list of polygons by an affine matrix.

        :param polygons: List of polygons to transform.
        :param matrix: A :class:`~raygeo.geo.Matrix` or a 3x3/4x4
            matrix as a list of lists.
        :returns: Transformed polygons.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "transform_polygons")]
fn transform_polygons_py(
    polygons: &Bound<'_, PyAny>,
    matrix: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let p = extract_polygons(polygons)?;
    let m = extract_core_matrix(matrix)?;
    Ok(polygons_to_tuples(transform_polygons(&p, &m)))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_point_line_distance(
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
#[pyfunction(name = "get_point_line_distance")]
fn point_line_distance_py(
    point: (f64, f64),
    line_start: (f64, f64),
    line_end: (f64, f64),
) -> f64 {
    get_point_line_distance(
        Point::new(point.0, point.1),
        Point::new(line_start.0, line_start.1),
        Point::new(line_end.0, line_end.1),
    )
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
    (r.min.x, r.min.y, r.max.x, r.max.y)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_group_bounds(
        polygons: collections.abc.Sequence[types.Polygon],
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
    Ok((r.min.x, r.min.y, r.max.x, r.max.y))
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
fn get_polygon_centroid_py(polygon: Vec<PyPoint2D>) -> (f64, f64) {
    point_to_tuple(get_polygon_centroid(&poly_to_points(polygon)))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_closest_point(
        polygon: collections.abc.Sequence[types.Point],
        x: float,
        y: float,
    ) -> tuple[float, tuple[float, float], float] | None:
        """Find the closest point on a polygon boundary to (x, y).

        :param polygon: Polygon as (x, y) points.
        :param x: X coordinate.
        :param y: Y coordinate.
        :returns: (t, (cx, cy), distance_squared) or None if degenerate.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_closest_point")]
fn get_polygon_closest_point_py(
    polygon: Vec<PyPoint2D>,
    x: f64,
    y: f64,
) -> Option<(f64, (f64, f64), f64)> {
    let pts = poly_to_points(polygon);
    get_polygon_closest_point(&pts, x, y)
        .map(|(t, pt, d2)| (t, (pt.x, pt.y), d2))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_closest_point(
        polygons: collections.abc.Sequence[types.Polygon],
        x: float,
        y: float,
    ) -> tuple[int, float, tuple[float, float], float] | None:
        """Find the closest point on any polygon in a list to (x, y).

        :param polygons: List of polygons as (x, y) points.
        :param x: X coordinate.
        :param y: Y coordinate.
        :returns: (polygon_index, t, (cx, cy), distance_squared) or None.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[allow(clippy::type_complexity)]
#[pyfunction(name = "get_polygons_closest_point")]
fn get_polygons_closest_point_py(
    polygons: &Bound<'_, PyAny>,
    x: f64,
    y: f64,
) -> PyResult<Option<(usize, f64, (f64, f64), f64)>> {
    let polys = extract_polygons(polygons)?;
    Ok(get_polygons_closest_point(&polys, Point::new(x, y))
        .map(|(pi, t, pt, d2)| (pi, t, (pt.x, pt.y), d2)))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def get_signed_boundary_distance(
        point: tuple[float, float],
        polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
    ) -> float:
        """Signed perpendicular distance from point to nearest polygon boundary.

        Positive = outside all polygons, Negative = inside any polygon,
        Zero = exactly on a boundary.

        :param point: Query point ``(x, y)``.
        :param polygons: List of polygons.
        :returns: Signed distance (mm).
        """
    "#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_signed_boundary_distance")]
fn get_signed_boundary_distance_py(
    point: (f64, f64),
    polygons: Vec<Vec<(f64, f64)>>,
) -> f64 {
    let polys: Vec<Vec<Point>> = polygons
        .into_iter()
        .map(|v| v.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    get_signed_boundary_distance(Point::new(point.0, point.1), &polys)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_circle_polygon(
        center: types.Point,
        radius: float,
        n: int = 64,
    ) -> types.Polygon:
        """Approximate a circle as an n-gon polygon.

        :param center: Centre point (x, y).
        :param radius: Circle radius.
        :param n: Number of sides (default 64).
        :returns: Polygon as list of (x, y) points.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_circle_polygon")]
#[pyo3(signature = (center, radius, n = 64))]
fn get_circle_polygon_py(
    center: (f64, f64),
    radius: f64,
    n: usize,
) -> Vec<(f64, f64)> {
    points_to_tuples(get_circle_polygon(
        Point::new(center.0, center.1),
        radius,
        n,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_segment_swept_polygon(
        a: types.Point,
        b: types.Point,
        radius: float,
    ) -> list[types.Polygon]:
        """Compute the swept area of a line segment with a given radius.

        Returns a rectangle (the Minkowski sum of the segment with a disk
        of *radius*) plus two disks at the endpoints.  Useful for toolpath
        clearance tracking and roughing simulation.

        :param a: Start point (x, y).
        :param b: End point (x, y).
        :param radius: Offset radius.
        :returns: List of polygons (rectangle + two end-caps).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_segment_swept_polygon")]
fn get_segment_swept_polygon_py(
    a: (f64, f64),
    b: (f64, f64),
    radius: f64,
) -> Vec<Vec<(f64, f64)>> {
    polygons_to_tuples(get_segment_swept_polygon(
        Point::new(a.0, a.1),
        Point::new(b.0, b.1),
        radius,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polyline_swept_polygon(
        path: collections.abc.Sequence[types.Point],
        radius: float,
    ) -> list[types.Polygon]:
        """Compute the Minkowski sum of a polyline path with a disk.

        Returns a single polygon covering the swept area — the union of
        segment-wide rectangular strips capped with half-circles at the
        first and last endpoints.

        :param path: Open polyline as (x, y) points.
        :param radius: Offset radius.
        :returns: A single swept polygon.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polyline_swept_polygon")]
fn get_polyline_swept_polygon_py(
    path: Vec<(f64, f64)>,
    radius: f64,
) -> Vec<Vec<(f64, f64)>> {
    let path_pts: Vec<Point> =
        path.iter().map(|&(x, y)| Point::new(x, y)).collect();
    polygons_to_tuples(get_polyline_swept_polygon(&path_pts, radius))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_miter_offset_intersection(
        v: types.Point,
        off_a: types.Point,
        dir_a: types.Point,
        off_b: types.Point,
        dir_b: types.Point,
    ) -> types.Point:
        """Intersect two offset lines at a vertex for miter join.

        Line A: ``v + off_a + t * dir_a``
        Line B: ``v + off_b + s * dir_b``

        Returns the intersection point.  When the lines are nearly parallel
        falls back to ``v + off_a``.

        :param v: Vertex point (x, y).
        :param off_a: Offset from *v* along line A.
        :param dir_a: Unit direction vector of line A.
        :param off_b: Offset from *v* along line B.
        :param dir_b: Unit direction vector of line B.
        :returns: Intersection point (x, y).
        """
    "#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_miter_offset_intersection")]
fn miter_offset_intersection_py(
    v: (f64, f64),
    off_a: (f64, f64),
    dir_a: (f64, f64),
    off_b: (f64, f64),
    dir_b: (f64, f64),
) -> (f64, f64) {
    point_to_tuple(get_miter_offset_intersection(
        Point::new(v.0, v.1),
        Point::new(off_a.0, off_a.1),
        Point::new(dir_a.0, dir_a.1),
        Point::new(off_b.0, off_b.1),
        Point::new(dir_b.0, dir_b.1),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def does_path_sweep_intersect_polygon(
        path: collections.abc.Sequence[types.Point],
        radius: float,
        obstacles: collections.abc.Sequence[types.Polygon],
    ) -> bool:
        """Check if a disk swept along a path intersects any obstacle polygon.

        Returns True when the Minkowski sweep of a disk of *radius* along
        *path* intersects any polygon in *obstacles*.

        :param path: Open polyline as (x, y) points.
        :param radius: Disk radius.
        :param obstacles: List of obstacle polygons.
        :returns: True if any obstacle intersects the sweep.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "does_path_sweep_intersect_polygon")]
fn does_path_sweep_intersect_polygon_py(
    path: Vec<(f64, f64)>,
    radius: f64,
    obstacles: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    let path_pts: Vec<Point> =
        path.iter().map(|&(x, y)| Point::new(x, y)).collect();
    let obs = extract_polygons(obstacles)?;
    let obs_bounds = compute_polygon_bounds(&obs);
    Ok(does_path_sweep_intersect_polygon(
        &path_pts,
        radius,
        &obs,
        &obs_bounds,
    ))
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

    def find_polygon_corners(
        polygon: collections.abc.Sequence[tuple[float, float]],
        corner_type: CornerType = CornerType.CONCAVE,
        threshold_deg: float = 90.0,
    ) -> list[tuple[int, float]]:
        """Find corners of a polygon matching *corner_type*.

        Returns a list of (vertex_index, interior_angle_deg) for each
        vertex whose interior angle is at least *threshold_deg*.
        Winding is auto-detected from the signed area.

        :param polygon: Polygon vertices (closed or open; treated as closed).
        :param corner_type: ``CornerType.CONCAVE`` (default) or ``CornerType.CONVEX``.
        :param threshold_deg: Minimum interior angle in degrees (default 90).
        :returns: List of (vertex_index, interior_angle_deg) tuples.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "find_polygon_corners")]
#[pyo3(signature = (polygon, corner_type = PyCornerType::Concave, threshold_deg = 90.0))]
fn find_polygon_corners_py(
    polygon: Vec<PyPoint2D>,
    corner_type: PyCornerType,
    threshold_deg: f64,
) -> Vec<(usize, f64)> {
    find_polygon_corners(
        &poly_to_points(polygon),
        corner_type.into(),
        threshold_deg,
    )
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
fn get_polygon_convex_hull_py(polygon: Vec<PyPoint2D>) -> Vec<(f64, f64)> {
    points_to_tuples(get_polygon_convex_hull(&poly_to_points(polygon)))
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
fn get_polygon_edges_py(
    polygon: Vec<PyPoint2D>,
) -> Vec<((f64, f64), (f64, f64))> {
    edge_pairs_to_tuples(get_polygon_edges(&poly_to_points(polygon)))
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
fn is_point_inside_polygon_py(
    point: (f64, f64),
    polygon: Vec<PyPoint2D>,
) -> bool {
    is_point_inside_polygon(
        Point::new(point.0, point.1),
        &poly_to_points(polygon),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def is_path_confined_to_boundary(
        path: collections.abc.Sequence[types.Point],
        boundary: collections.abc.Sequence[types.Point],
        clearance: float,
    ) -> bool:
        """Check if a path stays within clearance of a pocket boundary.

        Returns True when every vertex of *path* is inside *boundary* and
        no segment approaches within *clearance* of any boundary edge.

        :param path: Open polyline as (x, y) points.
        :param boundary: Pocket boundary polygon as (x, y) points.
        :param clearance: Minimum distance to boundary edges.
        :returns: True if path is safely inside the boundary.
        """
    "#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "is_path_confined_to_boundary")]
fn is_path_confined_to_boundary_py(
    path: Vec<PyPoint2D>,
    boundary: Vec<PyPoint2D>,
    clearance: f64,
) -> bool {
    is_path_confined_to_boundary(
        &poly_to_points(path),
        &poly_to_points(boundary),
        clearance,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def does_polygon_enclose_circle(
        center: types.Point,
        radius: float,
        polygon: collections.abc.Sequence[types.Point],
    ) -> bool:
        """Check if a polygon fully encloses a circle.

        Uses a conservative fast check: the polygon's AABB must contain the
        circle's AABB, and the circle center must be inside the polygon.

        :param center: Circle center (x, y).
        :param radius: Circle radius.
        :param polygon: Polygon as (x, y) points.
        :returns: True if the polygon fully encloses the circle.
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "does_polygon_enclose_circle")]
fn does_polygon_enclose_circle_py(
    center: (f64, f64),
    radius: f64,
    polygon: Vec<PyPoint2D>,
) -> bool {
    does_polygon_enclose_circle(
        Point::new(center.0, center.1),
        radius,
        &poly_to_points(polygon),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def offset_polygon(
        polygon: collections.abc.Sequence[types.Point],
        offset: float,
        join_style: JoinStyle = JoinStyle.MITER,
    ) -> list[types.Polygon]:
        """Offset (inflate/deflate) a polygon.

        :param polygon: Polygon as (x, y) points.
        :param offset: Offset distance (positive to inflate, negative to deflate).
        :param join_style: Corner join style (default: ``JoinStyle.MITER``).
        :returns: Offset polygon(s).
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "offset_polygon")]
#[pyo3(signature = (polygon, offset, join_style = PyJoinStyle::Miter))]
fn offset_polygon_py(
    polygon: Vec<PyPoint2D>,
    offset: f64,
    join_style: PyJoinStyle,
) -> Vec<Vec<(f64, f64)>> {
    polygons_to_tuples(offset_polygon(
        &poly_to_points(polygon),
        offset,
        join_style.into(),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def apply_minimum_curvature(
        polygon: collections.abc.Sequence[types.Point],
        r_min: float,
    ) -> list[types.Polygon]:
        """Fillet tight internal corners to a minimum radius.

        Offsets inward by ``r_min`` (Miter), then outward by ``r_min``
        (Round). Acts as a high-pass curvature filter — sharp corners
        are rounded to exactly ``r_min`` while the overall shape is
        preserved.

        :param polygon: Polygon as (x, y) points.
        :param r_min: Minimum allowed curvature radius.
        :returns: Filleted polygon(s).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "apply_minimum_curvature")]
fn apply_minimum_curvature_py(
    polygon: Vec<PyPoint2D>,
    r_min: f64,
) -> Vec<Vec<(f64, f64)>> {
    polygons_to_tuples(apply_minimum_curvature(&poly_to_points(polygon), r_min))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygons_union(polygons: collections.abc.Sequence[types.Polygon]) -> list[types.Polygon]:
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
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let p = extract_polygons(polygons)?;
    Ok(polygons_to_tuples(get_polygons_union(&p)))
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
) -> Vec<Vec<(f64, f64)>> {
    polygons_to_tuples(get_polygons_intersection(
        &poly_to_points(poly1),
        &poly_to_points(poly2),
    ))
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
) -> Vec<Vec<(f64, f64)>> {
    polygons_to_tuples(get_polygons_difference(
        &poly_to_points(poly1),
        &poly_to_points(poly2),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_group_intersection(
        subject: collections.abc.Sequence[types.Polygon],
        clip: collections.abc.Sequence[types.Polygon],
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
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let subject_polys = extract_polygons(subject)?;
    let clip_polys = extract_polygons(clip)?;
    Ok(polygons_to_tuples(get_polygons_group_intersection(
        &subject_polys,
        &clip_polys,
    )))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_group_difference(
        subject: collections.abc.Sequence[types.Polygon],
        clip: collections.abc.Sequence[types.Polygon],
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
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let subject_polys = extract_polygons(subject)?;
    let clip_polys = extract_polygons(clip)?;
    Ok(polygons_to_tuples(get_polygons_group_difference(
        &subject_polys,
        &clip_polys,
    )))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def do_polygons_intersect(
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
#[pyfunction(name = "do_polygons_intersect")]
#[pyo3(signature = (p1, p2, min_area=0.0))]
fn polygons_intersect_py(
    p1: Vec<PyPoint2D>,
    p2: Vec<PyPoint2D>,
    min_area: f64,
) -> bool {
    do_polygons_intersect(&poly_to_points(p1), &poly_to_points(p2), min_area)
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
) -> Vec<(f64, f64)> {
    points_to_tuples(flip_polygon(&poly_to_points(polygon), flip_h, flip_v))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def flip_polygons(
        polygons: collections.abc.Sequence[types.Polygon],
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
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let p = extract_polygons(polygons)?;
    Ok(polygons_to_tuples(flip_polygons(&p, flip_h, flip_v)))
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
fn rotate_polygon_py(polygon: Vec<PyPoint2D>, angle: f64) -> Vec<(f64, f64)> {
    points_to_tuples(rotate_polygon(&poly_to_points(polygon), angle))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def rotate_polygons(polygons: collections.abc.Sequence[types.Polygon], angle: float) -> list[types.Polygon]:
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
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let p = extract_polygons(polygons)?;
    Ok(polygons_to_tuples(rotate_polygons(&p, angle)))
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
) -> Vec<(f64, f64)> {
    points_to_tuples(scale_polygon(&poly_to_points(polygon), scale, scale_y))
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
) -> Vec<(f64, f64)> {
    points_to_tuples(translate_polygon(&poly_to_points(polygon), dx, dy))
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
    (r.min.x, r.min.y, r.max.x, r.max.y)
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
    (r.min.x, r.min.y, r.max.x, r.max.y)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import numpy.typing

    def flip_polygon_numpy(
        polygon: numpy.typing.NDArray,
        flip_h: bool,
        flip_v: bool,
    ) -> numpy.typing.NDArray:
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

    def flip_polygons_numpy(polygons: collections.abc.Sequence[numpy.typing.NDArray], flip_h: bool, flip_v: bool) -> list[numpy.typing.NDArray]:
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
    point: (f64, f64),
    polygon: Bound<'_, PyArray2<f64>>,
) -> bool {
    let p = _polygon_from_numpy(&polygon);
    is_point_inside_polygon(Point::new(point.0, point.1), &p)
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
    do_polygons_intersect(&p1, &p2, min_area)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import numpy.typing

    def rotate_polygon_numpy(
        polygon: numpy.typing.NDArray,
        angle: float,
    ) -> numpy.typing.NDArray:
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
    ) -> list[numpy.typing.NDArray]:
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
    ) -> numpy.typing.NDArray:
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
    ) -> list[numpy.typing.NDArray]:
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
                    ((p.x * scale) as i64, (p.y * scale) as i64)
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
        points.iter().map(|p| Point::new(p.0, p.1)).collect();
    is_polygon_clockwise(&points_2d)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def get_polygon_vertex_centroid(
        polygon: collections.abc.Sequence[tuple[float, float]],
    ) -> tuple[float, float]:
        """Arithmetic mean of polygon vertices (vertex-average centroid).

        Unlike :func:`get_polygon_centroid` (area-weighted shoelace centroid),
        this is useful for concave polygons where the area centroid lies
        outside the boundary.

        :param polygon: Polygon as (x, y) points.
        :returns: Vertex-average centroid (x, y).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_vertex_centroid")]
fn get_polygon_vertex_centroid_py(polygon: Vec<PyPoint2D>) -> (f64, f64) {
    let pts = poly_to_points(polygon);
    let c = get_polygon_vertex_centroid(&pts);
    (c.x, c.y)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def get_polygon_boundary_distance(
        a: collections.abc.Sequence[tuple[float, float]],
        b: collections.abc.Sequence[tuple[float, float]],
    ) -> float:
        """Minimum midpoint-to-segment distance between the boundaries of
        two polygons.

        Uses segment midpoints rather than raw segment-segment distance
        to avoid false positives from polygons that merely touch at a
        shared vertex.

        :param a: First polygon as (x, y) points.
        :param b: Second polygon as (x, y) points.
        :returns: Minimum boundary distance.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_boundary_distance")]
fn get_polygon_boundary_distance_py(
    a: Vec<PyPoint2D>,
    b: Vec<PyPoint2D>,
) -> f64 {
    let pts_a = poly_to_points(a);
    let pts_b = poly_to_points(b);
    get_polygon_boundary_distance(&pts_a, &pts_b)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def resample_polygon(
        polygon: collections.abc.Sequence[tuple[float, float]],
        spacing: float,
    ) -> list[tuple[float, float]]:
        """Resample a closed polygon by inserting evenly-spaced points along each
        edge so that no segment is longer than *spacing*.

        The result is a closed polyline (last point connects back to first
        conceptually, but is not duplicated).

        :param polygon: Polygon as (x, y) points.
        :param spacing: Maximum allowed segment length.
        :returns: Resampled polygon as list of (x, y) points.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "resample_polygon")]
fn resample_polygon_py(
    polygon: Vec<PyPoint2D>,
    spacing: f64,
) -> Vec<(f64, f64)> {
    let pts = poly_to_points(polygon);
    points_to_tuples(resample_polygon(&pts, spacing))
}

/// Outward-facing heading angle (radians) at the vertex of a closed polygon.
///
/// Finds the polygon edge closest to the vertex, then returns the angle
/// of the outward-facing normal via ``atan2``.
///
/// CCW winding → right normal (outward).
/// CW winding  → left normal  (outward).
///
/// :param polygon: Closed polygon as a list of ``(x, y)`` vertices.
/// :param vertex:  Query vertex ``(x, y)``.
/// :returns: Heading angle in radians in ``[-π, π]``.
#[gen_stub_pyfunction(
    python = r#"
    def get_polygon_heading_at(
        polygon: list[tuple[float, float]],
        vertex: tuple[float, float],
    ) -> float:
        ...
    "#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_heading_at")]
fn get_polygon_heading_at_py(
    polygon: Vec<PyPoint2D>,
    vertex: (f64, f64),
) -> f64 {
    let poly = poly_to_points(polygon);
    get_polygon_heading_at(&poly, Point::new(vertex.0, vertex.1))
}

/// Walk polygon vertices forward (in storage order, wrapping around),
/// starting from the vertex closest to *start*.
///
/// Returns a list of ``(index, x, y)`` tuples for every vertex in walk
/// order.  The caller can iterate and break when their predicate is met.
///
/// "Forward" follows storage order with wrap-around.
/// For CCW polygons this walks counter-clockwise (interior on left).
///
/// :param polygon: Closed polygon as a list of ``(x, y)`` vertices.
/// :param start:   Reference point — the closest vertex starts the walk.
/// :returns: List of ``(index, x, y)`` tuples in walk order.
#[gen_stub_pyfunction(
    python = r#"
    def walk_polygon_from_point(
        polygon: list[tuple[float, float]],
        start: tuple[float, float],
    ) -> list[tuple[int, float, float]]:
        ...
    "#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "walk_polygon_from_point")]
fn walk_polygon_from_point_py(
    polygon: Vec<PyPoint2D>,
    start: (f64, f64),
) -> Vec<(usize, f64, f64)> {
    let poly = poly_to_points(polygon);
    let start_pt = Point::new(start.0, start.1);

    let n = poly.len();
    if n < 3 {
        return vec![];
    }

    let start_idx = poly
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(start_pt)
                .partial_cmp(&b.distance_squared(start_pt))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    let mut result = Vec::with_capacity(n);
    for offset in 0..n {
        let idx = (start_idx + offset) % n;
        let pt = &poly[idx];
        result.push((idx, pt.x, pt.y));
    }
    result
}

/// Walk polygon vertices in order from a given start index.
///
/// Unlike :func:`walk_polygon_from_point`, this function takes a
/// starting vertex index directly and supports walking forward or
/// backward.  Returns all visited vertices as ``(index, x, y)``
/// tuples.
///
/// :param polygon: Closed polygon as a list of ``(x, y)`` vertices.
/// :param start_idx: Starting vertex index.
/// :param forward: ``True`` to walk forward (increasing index),
///    ``False`` to walk backward (decreasing index).
/// :param stride: Visit every `stride`-th vertex (default ``1`` = all).
/// :returns: List of ``(index, x, y)`` tuples in walk order.
#[gen_stub_pyfunction(
    python = r#"
    def walk_polygon_vertices(
        polygon: list[tuple[float, float]],
        start_idx: int,
        forward: bool,
        stride: int = 1,
    ) -> list[tuple[int, float, float]]:
        ...
    "#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "walk_polygon_vertices")]
#[pyo3(signature = (polygon, start_idx, forward, stride = None))]
fn walk_polygon_vertices_py(
    polygon: Vec<PyPoint2D>,
    start_idx: usize,
    forward: bool,
    stride: Option<usize>,
) -> Vec<(usize, f64, f64)> {
    let stride = stride.unwrap_or(1);
    let poly = poly_to_points(polygon);
    let mut result = Vec::with_capacity(poly.len());
    let _ = crate::geo::shape::polygon::walk_polygon_vertices(
        &poly,
        start_idx,
        forward,
        stride,
        |idx, pt| {
            result.push((idx, pt.x, pt.y));
            None::<()>
        },
    );
    result
}

/// Find edges of *polygon* that are not collinear with any edge of
/// the given *boundaries*.
///
/// An edge is considered collinear when its midpoint lies within
/// *dist_tol* of any boundary edge.  Returns the starting vertex indices
/// of all non-collinear (entry) edges.
///
/// :param polygon:   Input polygon as a list of ``(x, y)`` vertices.
/// :param boundaries: List of boundary polygons (pocket + islands).
/// :param dist_tol:   Distance tolerance in mm.
/// :returns: List of vertex indices (0-based) of entry edges.
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_entry_edges(
        polygon: collections.abc.Sequence[tuple[float, float]],
        boundaries: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        dist_tol: float = 1.0,
    ) -> list[int]:
        ...
    "#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "find_entry_edges")]
#[pyo3(signature = (polygon, boundaries, dist_tol = 1.0))]
fn find_entry_edges_py(
    polygon: Vec<PyPoint2D>,
    boundaries: Vec<Vec<PyPoint2D>>,
    dist_tol: f64,
) -> Vec<usize> {
    let poly = poly_to_points(polygon);
    let bds: Vec<Vec<Point>> =
        boundaries.into_iter().map(poly_to_points).collect();
    find_entry_edges(&poly, &bds, dist_tol)
}
