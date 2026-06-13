use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use super::super::geo::flex_point::{poly_to_points, PyPoint2D};
use crate::nest::ifp;
use crate::types::Point;

pyo3_stub_gen::module_doc!("raygeo.nest.ifp", "{}", MODULE_DOC);

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
        """
"#,
    module = "raygeo.nest.ifp"
)]
#[pyfunction(name = "inner_fit_polygon")]
fn inner_fit_polygon_py(
    bin: Vec<PyPoint2D>,
    part: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    ifp::inner_fit_polygon(&poly_to_points(bin), &poly_to_points(part))
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
        """
"#,
    module = "raygeo.nest.ifp"
)]
#[pyfunction(name = "build_no_go_zones")]
fn build_no_go_zones_py(
    bin: Vec<PyPoint2D>,
    part_neg: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    ifp::build_no_go_zones(&poly_to_points(bin), &poly_to_points(part_neg))
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
        """
"#,
    module = "raygeo.nest.ifp"
)]
#[pyfunction(name = "sweep_hull_for_edge")]
fn sweep_hull_for_edge_py(
    p1: PyPoint2D,
    p2: PyPoint2D,
    part_neg: Vec<PyPoint2D>,
) -> Vec<Point> {
    ifp::sweep_hull_for_edge(
        Point(p1.0, p1.1),
        Point(p2.0, p2.1),
        &poly_to_points(part_neg),
    )
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(inner_fit_polygon_py, m)?)?;
    m.add_function(wrap_pyfunction!(build_no_go_zones_py, m)?)?;
    m.add_function(wrap_pyfunction!(sweep_hull_for_edge_py, m)?)?;
    Ok(())
}
