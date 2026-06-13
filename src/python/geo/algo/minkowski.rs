pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.minkowski",
    "{}",
    MODULE_DOC_MINKOWSKI
);

pub(crate) const MODULE_DOC_MINKOWSKI: &str = "\
Minkowski sum operations for 2D polygon toolpath generation.

Provides convolution of point sequences and segments, Minkowski sums
for convex polygons, and no-fit polygon / inner fit polygon calculations
used in nesting and packing algorithms.
";

use super::super::flex_point::{extract_polygons, poly_to_points, PyPoint2D};
use crate::geo::algo::minkowski::{
    calculate_input_scale, convolve_point_sequences, convolve_two_segments,
    get_inner_fit_polygon, get_no_fit_polygon,
    get_polygon_minkowski_sum_convex,
};
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "minkowski")?;
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
        poly_a: collections.abc.Sequence[tuple[int, int]],
        poly_b: collections.abc.Sequence[tuple[int, int]],
    ) -> list[list[tuple[int, int]]]:
        """Compute the Minkowski sum of two convex polygons.

        :param poly_a: First convex polygon as integer points.
        :param poly_b: Second convex polygon as integer points.
        :returns: Minkowski sum as list of polygons.
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "get_polygon_minkowski_sum_convex")]
fn minkowski_sum_convex_py(
    poly_a: Vec<(i64, i64)>,
    poly_b: Vec<(i64, i64)>,
) -> Vec<Vec<(i64, i64)>> {
    get_polygon_minkowski_sum_convex(&poly_a, &poly_b)
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
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "get_inner_fit_polygon")]
fn get_inner_fit_polygon_py(
    outer: Vec<PyPoint2D>,
    inner: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    get_inner_fit_polygon(&poly_to_points(outer), &poly_to_points(inner))
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
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "get_no_fit_polygon")]
fn get_no_fit_polygon_py(
    subject: Vec<PyPoint2D>,
    tool: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    get_no_fit_polygon(&poly_to_points(subject), &poly_to_points(tool))
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
        """
"#,
    module = "raygeo.geo.algo.minkowski"
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
        a1: tuple[int, int],
        a2: tuple[int, int],
        b1: tuple[int, int],
        b2: tuple[int, int],
    ) -> list[tuple[int, int]]:
        """Convolve two line segments.

        :param a1: Start point of segment A.
        :param a2: End point of segment A.
        :param b1: Start point of segment B.
        :param b2: End point of segment B.
        :returns: Convolved point sequence.
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "convolve_two_segments")]
fn convolve_two_segments_py(
    a1: (i64, i64),
    a2: (i64, i64),
    b1: (i64, i64),
    b2: (i64, i64),
) -> Vec<(i64, i64)> {
    convolve_two_segments(a1, a2, b1, b2)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def convolve_point_sequences(
        seq_a: collections.abc.Sequence[tuple[int, int]],
        seq_b: collections.abc.Sequence[tuple[int, int]],
    ) -> list[list[tuple[int, int]]]:
        """Convolve two sequences of points.

        :param seq_a: First sequence of integer points.
        :param seq_b: Second sequence of integer points.
        :returns: Convolved point sequences.
        """
"#,
    module = "raygeo.geo.algo.minkowski"
)]
#[pyfunction(name = "convolve_point_sequences")]
fn convolve_point_sequences_py(
    seq_a: Vec<(i64, i64)>,
    seq_b: Vec<(i64, i64)>,
) -> Vec<Vec<(i64, i64)>> {
    convolve_point_sequences(&seq_a, &seq_b)
}
