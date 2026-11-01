pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.analysis",
    "{}",
    MODULE_DOC_ANALYSIS
);

pub(crate) const MODULE_DOC_ANALYSIS: &str = "\
Path analysis utilities for inspecting and cleaning geometry data.

Provides functions for removing duplicate points from point sequences,
extracting subpath vertices, computing subpath/geometry area, and
determining path winding order.
";

use super::super::flex_point::{point3d_to_tuple, points_to_tuples};
use super::super::geometry::PyTypedCommand;
use super::super::Geometry;
use crate::geo::types::WindingOrder;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "analysis")?;
    m.setattr("__doc__", MODULE_DOC_ANALYSIS)?;

    register_functions!(
        m,
        remove_duplicates_py,
        get_subpath_vertices_py,
        get_subpath_area_py,
        get_area_py,
        get_path_winding_order_py,
        get_closed_spans_py,
    );

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.analysis", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def remove_duplicates(points: collections.abc.Sequence[types.Point]) -> types.Polygon:
        """Remove duplicate points from a sequence.

        :param points: Sequence of (x, y) points.
        :returns: List of unique points.
        :complexity: O(n) time, O(n) space
        """
    "#,
    module = "raygeo.geo.algo.analysis"
)]
#[pyfunction(name = "remove_duplicates")]
fn remove_duplicates_py(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    crate::geo::algo::analysis::remove_duplicates(&points)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo

    def get_subpath_vertices(
        geometry: geo.Geometry,
        start_cmd_index: int,
    ) -> list[tuple[float, float]]:
        """Extract vertices from a subpath starting at the given command index.

        Linearizes arcs and beziers into vertex sequences.

        :param geometry: Geometry to extract vertices from.
        :param start_cmd_index: Index of the starting command.
        :returns: List of (x, y) vertices.
        :complexity: O(n + m) time, O(m) space where n is the number of commands and m the number of output vertices
        """
    "#,
    module = "raygeo.geo.algo.analysis"
)]
#[pyfunction(name = "get_subpath_vertices")]
fn get_subpath_vertices_py(
    geometry: &Geometry,
    start_cmd_index: usize,
) -> Vec<(f64, f64)> {
    points_to_tuples(
        crate::geo::algo::analysis::get_subpath_vertices_from_array(
            geometry.inner.data(),
            start_cmd_index,
        ),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo

    def get_subpath_area(
        geometry: geo.Geometry,
        start_cmd_index: int,
    ) -> float:
        """Compute the signed area of a subpath using the shoelace formula.

        Positive area is CCW, negative is CW. Returns 0 for unclosed subpaths.

        :param geometry: Geometry to compute area from.
        :param start_cmd_index: Index of the starting command.
        :returns: Signed area.
        :complexity: O(n) time, O(1) space
        """
    "#,
    module = "raygeo.geo.algo.analysis"
)]
#[pyfunction(name = "get_subpath_area")]
fn get_subpath_area_py(geometry: &Geometry, start_cmd_index: usize) -> f64 {
    crate::geo::algo::analysis::get_subpath_area_from_array(
        geometry.inner.data(),
        start_cmd_index,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo

    def get_area(geometry: geo.Geometry) -> float:
        """Compute the total unsigned area enclosed by the geometry.

        Sums all subpaths (outer + inner). Returns 0 for empty or open geometry.

        :param geometry: Geometry to compute area from.
        :returns: Total unsigned area.
        :complexity: O(n) time, O(1) space
        """
    "#,
    module = "raygeo.geo.algo.analysis"
)]
#[pyfunction(name = "get_area")]
fn get_area_py(geometry: &Geometry) -> f64 {
    crate::geo::algo::analysis::get_area_from_array(geometry.inner.data())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo

    def get_path_winding_order(
        geometry: geo.Geometry,
        start_cmd_index: int,
    ) -> str:
        """Determine the winding order of a subpath.

        :param geometry: Geometry to analyze.
        :param start_cmd_index: Index of the starting command.
        :returns: ``"ccw"``, ``"cw"``, or ``"unknown"``.
        :complexity: O(n) time, O(1) space
        """
    "#,
    module = "raygeo.geo.algo.analysis"
)]
#[pyfunction(name = "get_path_winding_order")]
fn get_path_winding_order_py(
    geometry: &Geometry,
    start_cmd_index: usize,
) -> &'static str {
    match crate::geo::algo::analysis::get_path_winding_order_from_array(
        geometry.inner.data(),
        start_cmd_index,
    ) {
        Some(WindingOrder::CCW) => "ccw",
        Some(WindingOrder::CW) => "cw",
        None => "unknown",
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    import raygeo
    import raygeo.geo

    def get_closed_spans(
        geometry: raygeo.geo.Geometry,
        tolerance: float = 0.1,
    ) -> list[list[tuple[tuple[float, float, float], typing.Any]]]:
        """Split a geometry into its closed subpaths as exact spans.

        Each subpath comes back as a list of ``(start point, command)``
        spans. Lines and arcs stay exact; other curves are followed as
        line chains within `tolerance`. Open, degenerate, and non-finite
        subpaths are dropped, and the winding of each loop is kept as
        drawn (counter-clockwise loops bound material, clockwise ones
        holes).

        :param geometry: geometry whose closed subpaths are extracted.
        :param tolerance: line-chain tolerance for non-line-and-arc curves.
        :returns: one span list per closed subpath.
        :complexity: O(n) time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.analysis"
)]
#[pyfunction(name = "get_closed_spans")]
#[pyo3(signature = (geometry, tolerance = 0.1))]
#[allow(clippy::type_complexity)]
fn get_closed_spans_py(
    py: Python<'_>,
    geometry: &Geometry,
    tolerance: f64,
) -> PyResult<Vec<Vec<((f64, f64, f64), Py<PyAny>)>>> {
    crate::geo::algo::analysis::get_closed_spans(&geometry.inner, tolerance)
        .into_iter()
        .map(|spans| {
            spans
                .into_iter()
                .map(|(start, command)| {
                    Ok((
                        point3d_to_tuple(start),
                        PyTypedCommand::from(command).into_py_obj(py)?,
                    ))
                })
                .collect()
        })
        .collect()
}
