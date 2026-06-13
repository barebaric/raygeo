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

use super::super::Geometry;
use crate::types::{Point, WindingOrder};
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
    );

    algo_mod.add_submodule(&m)?;
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
        """
    "#,
    module = "raygeo.geo.algo.analysis"
)]
#[pyfunction(name = "get_subpath_vertices")]
fn get_subpath_vertices_py(
    geometry: &Geometry,
    start_cmd_index: usize,
) -> Vec<Point> {
    crate::geo::algo::analysis::get_subpath_vertices_from_array(
        geometry.inner.data(),
        start_cmd_index,
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
