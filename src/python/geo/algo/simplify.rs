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

use super::super::flex_point::{points3d_to_tuples, PyPoint3D};
use crate::geo::algo::simplify::simplify_polyline_3d;
use crate::types::Point3D;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "simplify")?;
    m.setattr("__doc__", MODULE_DOC_SIMPLIFY)?;

    register_functions!(m, simplify_polyline_3d_py,);

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def simplify_polyline_3d(
        points: collections.abc.Sequence[types.Point3D],
        tolerance: float,
    ) -> types.Polygon3D:
        """Simplify a 3D polyline using the Ramer-Douglas-Peucker algorithm.

        The simplification uses XY distance, but preserves Z coordinates
        of kept points.

        :param points: Sequence of (x, y, z) points.
        :param tolerance: Simplification tolerance.
        :returns: Simplified 3D point sequence.
        :complexity: O(n log n) average time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.simplify"
)]
#[pyfunction(name = "simplify_polyline_3d")]
fn simplify_polyline_3d_py(
    points: Vec<PyPoint3D>,
    tolerance: f64,
) -> Vec<(f64, f64, f64)> {
    let pts: Vec<Point3D> = points
        .into_iter()
        .map(|p| Point3D::new(p.0, p.1, p.2))
        .collect();
    let result = simplify_polyline_3d(&pts, tolerance);
    points3d_to_tuples(result)
}
