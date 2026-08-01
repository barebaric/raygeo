//! Python bindings for polyline operations.

use super::super::flex_point::{
    points_to_tuples, poly_to_points, polygons_to_tuples, PyPoint2D,
};
use crate::geo::shape::polyline::{
    get_polyline_bounds, get_polyline_closest_point, resample_polyline,
    split_polyline_at_v_junctions, trim_polyline_angular_ends,
    trim_polyline_at,
};
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "polyline")?;

    register_functions!(
        m,
        get_polyline_bounds_py,
        get_polyline_closest_point_py,
        resample_polyline_py,
        split_polyline_at_v_junctions_py,
        trim_polyline_angular_ends_py,
        trim_polyline_at_py,
    );

    shape_mod.add_submodule(&m)?;
    let sys_modules = shape_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.polyline", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polyline_bounds(
        polyline: collections.abc.Sequence[types.Point],
    ) -> types.Rect:
        """Get the bounding rectangle of an open polyline.

        :param polyline: Polyline as (x, y) points.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        :complexity: O(n)
        """
"#,
    module = "raygeo.geo.shape.polyline"
)]
#[pyfunction(name = "get_polyline_bounds")]
fn get_polyline_bounds_py(polyline: Vec<PyPoint2D>) -> (f64, f64, f64, f64) {
    let r = get_polyline_bounds(&poly_to_points(polyline));
    (r.min.x, r.min.y, r.max.x, r.max.y)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def get_polyline_closest_point(
        polyline: collections.abc.Sequence[tuple[float, float]],
        point: tuple[float, float],
    ) -> tuple[int, float] | None:
        """Find the closest edge and parametric position on an open polyline.

        Each edge of the polyline is tested, and the closest one is
        returned as ``(edge_index, t)`` where ``t`` in [0, 1] is the
        parametric position along that edge.

        :param polyline: Open polyline as (x, y) points.
        :param point: Query point (x, y).
        :returns: ``(edge_index, t)`` or None if the polyline has fewer
                  than 2 points.
        """
    "#,
    module = "raygeo.geo.shape.polyline"
)]
#[pyfunction(name = "get_polyline_closest_point")]
fn get_polyline_closest_point_py(
    polyline: Vec<(f64, f64)>,
    point: (f64, f64),
) -> Option<(usize, f64)> {
    let pts: Vec<Point> =
        polyline.iter().map(|&(x, y)| Point::new(x, y)).collect();
    get_polyline_closest_point(&pts, Point::new(point.0, point.1))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def trim_polyline_at(
        polyline: collections.abc.Sequence[tuple[float, float]],
        a: tuple[float, float],
        b: tuple[float, float],
    ) -> list[tuple[float, float]]:
        """Trim a polyline to the portion between two points.

        Each point is projected onto the nearest edge of the polyline.
        The returned polyline goes from the projection of *a* to the
        projection of *b*, preserving intermediate vertices.

        :param polyline: Open polyline as (x, y) points.
        :param a: Start point to trim at.
        :param b: End point to trim at.
        :returns: Trimmed polyline.
        """
    "#,
    module = "raygeo.geo.shape.polyline"
)]
#[pyfunction(name = "trim_polyline_at")]
fn trim_polyline_at_py(
    polyline: Vec<(f64, f64)>,
    a: (f64, f64),
    b: (f64, f64),
) -> Vec<(f64, f64)> {
    let pts: Vec<Point> =
        polyline.iter().map(|&(x, y)| Point::new(x, y)).collect();
    trim_polyline_at(&pts, Point::new(a.0, a.1), Point::new(b.0, b.1))
        .into_iter()
        .map(|p| (p.x, p.y))
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def trim_polyline_angular_ends(
        polygon: collections.abc.Sequence[tuple[float, float]],
        start: int,
        length: int,
        angle_threshold_rad: float,
    ) -> tuple[int, int]:
        """Trim vertices from both ends of a contiguous subsequence where the
        interior angle jumps sharply.

        Detects "transition" vertices at the boundary between two differently-
        curved regions of a closed polygon.  The function iteratively trims
        such vertices from the start and end of the subsequence until no more
        trimming occurs or the sequence is too short.

        :param polygon: Closed polygon as (x, y) points.
        :param start: Start index of the subsequence.
        :param length: Length of the subsequence.
        :param angle_threshold_rad: Angle threshold in radians.
        :returns: ``(new_start, new_length)`` within the original polygon.
        """
    "#,
    module = "raygeo.geo.shape.polyline"
)]
#[pyfunction(name = "trim_polyline_angular_ends")]
fn trim_polyline_angular_ends_py(
    polygon: Vec<(f64, f64)>,
    start: usize,
    length: usize,
    angle_threshold_rad: f64,
) -> (usize, usize) {
    let pts: Vec<Point> =
        polygon.iter().map(|&(x, y)| Point::new(x, y)).collect();
    trim_polyline_angular_ends(&pts, start, length, angle_threshold_rad)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def resample_polyline(
        polyline: collections.abc.Sequence[tuple[float, float]],
        max_len: float,
    ) -> list[tuple[float, float]]:
        """Resample an open 2D polyline so consecutive points are at most
        *max_len* apart.

        New points are linearly interpolated along each segment that
        exceeds the threshold.  The first and last points are always
        preserved.

        :param polyline: Open polyline as (x, y) points.
        :param max_len: Maximum allowed segment length.
        :returns: Resampled polyline.
        :complexity: O(n * m)
        """
"#,
    module = "raygeo.geo.shape.polyline"
)]
#[pyfunction(name = "resample_polyline")]
fn resample_polyline_py(
    polyline: Vec<PyPoint2D>,
    max_len: f64,
) -> Vec<(f64, f64)> {
    let pts = poly_to_points(polyline);
    points_to_tuples(resample_polyline(&pts, max_len))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def split_polyline_at_v_junctions(
        polyline: collections.abc.Sequence[tuple[float, float]],
        angle_threshold: float,
    ) -> list[list[tuple[float, float]]]:
        """Split a polyline at V-junction vertices where the interior
        angle is much sharper than both neighbours.

        Each resulting sub-polyline is trimmed with
        ``trim_polyline_angular_ends``.

        :param polyline: Sequence of (x, y) points.
        :param angle_threshold: Angle threshold in radians.
        :returns: List of sub-polylines.
        :complexity: O(n) time, O(n) space
        """
"#,
    module = "raygeo.geo.shape.polyline"
)]
#[pyfunction(name = "split_polyline_at_v_junctions")]
fn split_polyline_at_v_junctions_py(
    polyline: Vec<PyPoint2D>,
    angle_threshold: f64,
) -> Vec<Vec<(f64, f64)>> {
    let pts = poly_to_points(polyline);
    let result = split_polyline_at_v_junctions(&pts, angle_threshold);
    polygons_to_tuples(result)
}
