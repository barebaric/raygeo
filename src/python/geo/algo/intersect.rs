pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.intersect",
    "{}",
    MODULE_DOC_INTERSECT
);

pub(crate) const MODULE_DOC_INTERSECT: &str = "\
Geometry intersection utilities.

Low-level intersection primitives for ray-segment and segment-segment
tests, plus higher-level self-intersection and cross-intersection checks
on geometry command arrays.
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::intersect::ray_line_intersection as rust_ray_line_intersection;

#[gen_stub_pyfunction(
    python = r#"
    def ray_line_intersection(
        origin: tuple[float, float],
        direction: tuple[float, float],
        a: tuple[float, float],
        b: tuple[float, float],
    ) -> tuple[float, float] | None:
        """Intersect a ray with a line segment.

        Given a ray starting at origin in the given direction, and a line
        segment from a to b, returns the intersection point if the ray hits
        the segment (including endpoints) in the forward direction, or None
        if there is no intersection.

        :param origin: Ray start point (x, y).
        :param direction: Ray direction vector (dx, dy).
        :param a: Line segment start point (x, y).
        :param b: Line segment end point (x, y).
        :returns: Intersection point (x, y), or None.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.intersect"
)]
#[pyfunction(name = "ray_line_intersection")]
fn ray_line_intersection_py(
    origin: (f64, f64),
    direction: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
) -> Option<(f64, f64)> {
    rust_ray_line_intersection(
        crate::types::Point::new(origin.0, origin.1),
        crate::types::Point::new(direction.0, direction.1),
        crate::types::Point::new(a.0, a.1),
        crate::types::Point::new(b.0, b.1),
    )
    .map(|p| (p.x, p.y))
}

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "intersect")?;
    m.setattr("__doc__", MODULE_DOC_INTERSECT)?;

    register_functions!(m, ray_line_intersection_py);

    algo_mod.add_submodule(&m)?;
    Ok(())
}
