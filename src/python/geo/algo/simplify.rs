pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.simplify",
    "{}",
    MODULE_DOC_SIMPLIFY
);

pub(crate) const MODULE_DOC_SIMPLIFY: &str = "\
Polyline simplification using the Ramer-Douglas-Peucker algorithm.

Reduces the number of points in a polyline while preserving the overall
shape within a given tolerance.
";

use super::super::flex_point::{poly_to_points, PyPoint2D};
use crate::geo::algo::simplify::simplify_polyline;
use crate::types::{Point, Point3D};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "simplify")?;
    m.setattr("__doc__", MODULE_DOC_SIMPLIFY)?;

    register_functions!(m, simplify_polyline_py,);

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def simplify_polyline(
        points: collections.abc.Sequence[types.Point],
        tolerance: float,
    ) -> types.Polygon:
        """Simplify a polyline using the Ramer-Douglas-Peucker algorithm.

        :param points: Sequence of (x, y) points.
        :param tolerance: Simplification tolerance.
        :returns: Simplified point sequence.
        :complexity: O(n log n) average time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.simplify"
)]
#[pyfunction(name = "simplify_polyline")]
fn simplify_polyline_py(points: Vec<PyPoint2D>, tolerance: f64) -> Vec<Point> {
    let pts = poly_to_points(points);
    let points_3d: Vec<crate::Point3D> =
        pts.iter().map(|p| Point3D(p.0, p.1, 0.0)).collect();
    let result = simplify_polyline(&points_3d, tolerance);
    result.iter().map(|p| Point(p.0, p.1)).collect()
}
