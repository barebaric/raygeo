use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use super::super::geo::flex_point::{poly_to_points, PyPoint2D};
use crate::nest::nfp;
use crate::types::Point;

pyo3_stub_gen::module_doc!("raygeo.nest.nfp", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
No-Fit Polygon calculation for nesting algorithms.

Provides functions for computing No-Fit Polygons (NFP) using Minkowski
sums, both for convex and general polygon pairs.
";

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def no_fit_polygon(
        static_poly: collections.abc.Sequence[types.Point],
        orbiting: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Compute the No-Fit Polygon (NFP) for two polygons.

        :param static_poly: Static polygon as (x, y) points.
        :param orbiting: Orbiting polygon as (x, y) points.
        :returns: List of NFP polygons.
        :complexity: O(n * m) where n, m = vertex counts of input polygons.
        """
"#,
    module = "raygeo.nest.nfp"
)]
#[pyfunction(name = "no_fit_polygon")]
fn no_fit_polygon_py(
    static_poly: Vec<PyPoint2D>,
    orbiting: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    nfp::no_fit_polygon(&poly_to_points(static_poly), &poly_to_points(orbiting))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def nfp_convex_fast(
        static_poly: collections.abc.Sequence[tuple[float, float]],
        orbiting: collections.abc.Sequence[tuple[float, float]],
    ) -> list[list[tuple[float, float]]]:
        """Fast NFP for convex polygon pairs.

        :param static_poly: Static polygon as points.
        :param orbiting: Orbiting polygon as points.
        :returns: List of NFP polygons.
        :complexity: O(n + m) for convex polygon pairs.
        """
"#,
    module = "raygeo.nest.nfp"
)]
#[pyfunction(name = "nfp_convex_fast")]
fn nfp_convex_fast_py(
    static_poly: Vec<PyPoint2D>,
    orbiting: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    nfp::nfp_convex_fast(
        &poly_to_points(static_poly),
        &poly_to_points(orbiting),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def nfp_minkowski(
        static_poly: collections.abc.Sequence[tuple[float, float]],
        orbiting: collections.abc.Sequence[tuple[float, float]],
    ) -> list[list[tuple[float, float]]]:
        """General NFP using Minkowski sum with Clipper union.

        :param static_poly: Static polygon as points.
        :param orbiting: Orbiting polygon as points.
        :returns: List of NFP polygons.
        :complexity: O(n * m) where n, m = vertex counts.
        """
"#,
    module = "raygeo.nest.nfp"
)]
#[pyfunction(name = "nfp_minkowski")]
fn nfp_minkowski_py(
    static_poly: Vec<PyPoint2D>,
    orbiting: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    nfp::nfp_minkowski(&poly_to_points(static_poly), &poly_to_points(orbiting))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def normalize_polygon(
        poly: collections.abc.Sequence[types.Point],
    ) -> tuple[types.Polygon, float, float]:
        """Shift a polygon so its bounding box minimum is at (0, 0).

        :param poly: Input polygon as (x, y) points.
        :returns: (normalized_polygon, offset_x, offset_y).
        :complexity: O(n) where n = vertex count.
        """
"#,
    module = "raygeo.nest.nfp"
)]
#[pyfunction(name = "normalize_polygon")]
fn normalize_polygon_py(poly: Vec<PyPoint2D>) -> (Vec<Point>, f64, f64) {
    nfp::normalize_polygon(&poly_to_points(poly))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def polygon_to_key(
        poly: collections.abc.Sequence[types.Point],
    ) -> list[tuple[int, int]]:
        """Convert a polygon to a rounded integer key for caching.

        :param poly: Input polygon as (x, y) points.
        :returns: List of rounded (x, y) integer tuples.
        :complexity: O(n) where n = vertex count.
        """
"#,
    module = "raygeo.nest.nfp"
)]
#[pyfunction(name = "polygon_to_key")]
fn polygon_to_key_py(poly: Vec<PyPoint2D>) -> Vec<(i64, i64)> {
    nfp::polygon_to_key(&poly_to_points(poly))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(no_fit_polygon_py, m)?)?;
    m.add_function(wrap_pyfunction!(nfp_convex_fast_py, m)?)?;
    m.add_function(wrap_pyfunction!(nfp_minkowski_py, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_polygon_py, m)?)?;
    m.add_function(wrap_pyfunction!(polygon_to_key_py, m)?)?;
    Ok(())
}
