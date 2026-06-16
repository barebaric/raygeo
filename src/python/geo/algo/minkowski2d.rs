pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.minkowski2d",
    "{}",
    MODULE_DOC_MINKOWSKI
);

pub(crate) const MODULE_DOC_MINKOWSKI: &str = "\
Minkowski sum operations for 2D polygon toolpath generation.

Provides convolution of point sequences and segments, Minkowski sums
for convex polygons, and no-fit polygon / inner fit polygon calculations
used in nesting and packing algorithms.
";

use super::super::flex_point::{
    extract_polygons, poly_to_points, polygons_to_tuples, PyPoint2D,
};
use crate::geo::algo::minkowski2d::{
    calculate_input_scale, convolve_point_sequences, convolve_two_segments,
    get_inner_fit_polygon, get_no_fit_polygon,
    get_polygon_minkowski_sum_convex,
};
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "minkowski2d")?;
    m.setattr("__doc__", MODULE_DOC_MINKOWSKI)?;

    register_functions!(
        m,
        minkowski_sum_convex_py,
        get_inner_fit_polygon_py,
        get_no_fit_polygon_py,
        calculate_input_scale_py,
        convolve_two_segments_py,
        convolve_point_sequences_py,
    );

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def get_polygon_minkowski_sum_convex(
        poly_a: collections.abc.Sequence[tuple[float, float]],
        poly_b: collections.abc.Sequence[tuple[float, float]],
    ) -> list[list[tuple[float, float]]]:
        """Compute the Minkowski sum of two convex polygons.

        :param poly_a: First convex polygon as points.
        :param poly_b: Second convex polygon as points.
        :returns: Minkowski sum as list of polygons.
        :complexity: O(n + m) time, O(n + m) space
        """
"#,
    module = "raygeo.geo.algo.minkowski2d"
)]
#[pyfunction(name = "get_polygon_minkowski_sum_convex")]
fn minkowski_sum_convex_py(
    poly_a: Vec<(f64, f64)>,
    poly_b: Vec<(f64, f64)>,
) -> Vec<Vec<(f64, f64)>> {
    let poly_a_pts: Vec<Point> =
        poly_a.iter().map(|(x, y)| Point::new(*x, *y)).collect();
    let poly_b_pts: Vec<Point> =
        poly_b.iter().map(|(x, y)| Point::new(*x, *y)).collect();
    polygons_to_tuples(get_polygon_minkowski_sum_convex(
        &poly_a_pts,
        &poly_b_pts,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_inner_fit_polygon(
        outer: collections.abc.Sequence[types.Point],
        inner: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Compute the inner fit polygon (no-fit polygon for nesting).

        :param outer: Outer polygon as (x, y) points.
        :param inner: Inner polygon as (x, y) points.
        :returns: Inner fit polygon.
        :complexity: O(n * m) time, O(n + m) space
        """
"#,
    module = "raygeo.geo.algo.minkowski2d"
)]
#[pyfunction(name = "get_inner_fit_polygon")]
fn get_inner_fit_polygon_py(
    outer: Vec<PyPoint2D>,
    inner: Vec<PyPoint2D>,
) -> Vec<Vec<(f64, f64)>> {
    polygons_to_tuples(get_inner_fit_polygon(
        &poly_to_points(outer),
        &poly_to_points(inner),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_no_fit_polygon(
        subject: collections.abc.Sequence[types.Point],
        tool: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Compute the no-fit polygon for two 2D polygons.

        :param subject: Subject polygon as (x, y) points.
        :param tool: Tool polygon as (x, y) points.
        :returns: No-fit polygon.
        :complexity: O(n * m) time, O(n + m) space
        """
"#,
    module = "raygeo.geo.algo.minkowski2d"
)]
#[pyfunction(name = "get_no_fit_polygon")]
fn get_no_fit_polygon_py(
    subject: Vec<PyPoint2D>,
    tool: Vec<PyPoint2D>,
) -> Vec<Vec<(f64, f64)>> {
    polygons_to_tuples(get_no_fit_polygon(
        &poly_to_points(subject),
        &poly_to_points(tool),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def calculate_input_scale(
        polygons: collections.abc.Sequence[collections.abc.Sequence[types.Point]],
        max_int: int = 2147483647,
    ) -> float:
        """Calculate the optimal input scale for clipper operations.

        :param polygons: List of polygons to scale.
        :param max_int: Maximum integer value for Clipper.
        :returns: Optimal scale factor.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.minkowski2d"
)]
#[pyfunction(name = "calculate_input_scale")]
#[pyo3(signature = (polygons, max_int=2147483647))]
fn calculate_input_scale_py(
    polygons: &Bound<'_, PyAny>,
    max_int: i64,
) -> PyResult<f64> {
    let polys = extract_polygons(polygons)?;
    Ok(calculate_input_scale(&polys, max_int))
}

#[gen_stub_pyfunction(
    python = r#"
    def convolve_two_segments(
        a1: tuple[float, float],
        a2: tuple[float, float],
        b1: tuple[float, float],
        b2: tuple[float, float],
    ) -> list[tuple[float, float]]:
        """Convolve two line segments.

        :param a1: Start point of segment A.
        :param a2: End point of segment A.
        :param b1: Start point of segment B.
        :param b2: End point of segment B.
        :returns: Convolved point sequence.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.minkowski2d"
)]
#[pyfunction(name = "convolve_two_segments")]
fn convolve_two_segments_py(
    a1: (f64, f64),
    a2: (f64, f64),
    b1: (f64, f64),
    b2: (f64, f64),
) -> Vec<(f64, f64)> {
    let result = convolve_two_segments(
        Point::new(a1.0, a1.1),
        Point::new(a2.0, a2.1),
        Point::new(b1.0, b1.1),
        Point::new(b2.0, b2.1),
    );
    result.into_iter().map(|p| (p.x, p.y)).collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def convolve_point_sequences(
        seq_a: collections.abc.Sequence[tuple[float, float]],
        seq_b: collections.abc.Sequence[tuple[float, float]],
    ) -> list[list[tuple[float, float]]]:
        """Convolve two sequences of points.

        :param seq_a: First sequence of points.
        :param seq_b: Second sequence of points.
        :returns: Convolved point sequences.
        :complexity: O(n * m) time, O(n * m) space
        """
"#,
    module = "raygeo.geo.algo.minkowski2d"
)]
#[pyfunction(name = "convolve_point_sequences")]
fn convolve_point_sequences_py(
    seq_a: Vec<(f64, f64)>,
    seq_b: Vec<(f64, f64)>,
) -> Vec<Vec<(f64, f64)>> {
    let seq_a_pts: Vec<Point> =
        seq_a.iter().map(|(x, y)| Point::new(*x, *y)).collect();
    let seq_b_pts: Vec<Point> =
        seq_b.iter().map(|(x, y)| Point::new(*x, *y)).collect();
    let result = convolve_point_sequences(&seq_a_pts, &seq_b_pts);
    result
        .into_iter()
        .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
        .collect()
}
