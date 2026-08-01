use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::nest2d::ifp;
use crate::geo::types::Point;
use crate::python::geo::flex_point::{
    points_to_tuples, poly_to_points, polygons_to_tuples, PyPoint2D,
};

pyo3_stub_gen::module_doc!("raygeo.geo.algo.nest2d.ifp", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Inner-Fit Polygon (IFP) calculation for nesting algorithms.

Provides functions for computing Inner-Fit Polygons, which define the valid
placement region for a part inside a bin.
";

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def inner_fit_polygon(
        bin: collections.abc.Sequence[types.Point],
        part: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Compute the Inner-Fit Polygon (IFP) for a part inside a bin.

        :param bin: Bin polygon as (x, y) points.
        :param part: Part polygon as (x, y) points.
        :returns: List of IFP polygons.
        :complexity: O(n * m) where n, m = vertex counts of bin and part.
        """
"#,
    module = "raygeo.geo.algo.nest2d.ifp"
)]
#[pyfunction(name = "inner_fit_polygon")]
fn inner_fit_polygon_py(
    bin: Vec<PyPoint2D>,
    part: Vec<PyPoint2D>,
) -> Vec<Vec<(f64, f64)>> {
    polygons_to_tuples(ifp::inner_fit_polygon(
        &poly_to_points(bin),
        &poly_to_points(part),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def build_no_go_zones(
        bin: collections.abc.Sequence[types.Point],
        part_neg: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Build the no-go zones for a bin-part pair.

        :param bin: Bin polygon as (x, y) points.
        :param part_neg: Orbiting polygon negated as (x, y) points.
        :returns: List of no-go zone polygons.
        :complexity: O(n * m) where n, m = vertex counts.
        """
"#,
    module = "raygeo.geo.algo.nest2d.ifp"
)]
#[pyfunction(name = "build_no_go_zones")]
fn build_no_go_zones_py(
    bin: Vec<PyPoint2D>,
    part_neg: Vec<PyPoint2D>,
) -> Vec<Vec<(f64, f64)>> {
    polygons_to_tuples(ifp::build_no_go_zones(
        &poly_to_points(bin),
        &poly_to_points(part_neg),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def sweep_hull_for_edge(
        p1: types.Point,
        p2: types.Point,
        part_neg: collections.abc.Sequence[types.Point],
    ) -> types.Polygon:
        """Compute the convex hull sweep of part_neg along the edge p1->p2.

        :param p1: First edge endpoint.
        :param p2: Second edge endpoint.
        :param part_neg: Orbiting polygon negated as (x, y) points.
        :returns: Convex hull polygon.
        :complexity: O(n log n) for convex hull computation.
        """
"#,
    module = "raygeo.geo.algo.nest2d.ifp"
)]
#[pyfunction(name = "sweep_hull_for_edge")]
fn sweep_hull_for_edge_py(
    p1: PyPoint2D,
    p2: PyPoint2D,
    part_neg: Vec<PyPoint2D>,
) -> Vec<(f64, f64)> {
    points_to_tuples(ifp::sweep_hull_for_edge(
        Point::new(p1.0, p1.1),
        Point::new(p2.0, p2.1),
        &poly_to_points(part_neg),
    ))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(inner_fit_polygon_py, m)?)?;
    m.add_function(wrap_pyfunction!(build_no_go_zones_py, m)?)?;
    m.add_function(wrap_pyfunction!(sweep_hull_for_edge_py, m)?)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.nest2d.ifp", m)?;
    Ok(())
}
