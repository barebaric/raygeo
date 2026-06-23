pyo3_stub_gen::module_doc!("raygeo.geo.algo.offset", "{}", MODULE_DOC_OFFSET);

pub(crate) const MODULE_DOC_OFFSET: &str = "\
Polygon offsetting operations for geometry data.

Provides concentric inward offset generation for adaptive clearing
and pocketing toolpath generation.
";

use super::super::flex_point::{poly_to_points, polygons_to_tuples, PyPoint2D};
use super::super::shape::polygon::PyJoinStyle;
use crate::geo::algo::offset::{
    compute_inset_region, concentric_offsets, find_deepest_cores,
    offset_contour_group,
};
use crate::types::{Point as GeoPoint, Polygon as GeoPolygon};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "offset")?;
    m.setattr("__doc__", MODULE_DOC_OFFSET)?;

    register_functions!(
        m,
        concentric_offsets_py,
        find_deepest_cores_py,
        offset_contour_group_py,
        compute_inset_region_py,
    );

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def concentric_offsets(
        geom: raygeo.Geometry,
        step: float,
        max_passes: int = 10,
        min_area: float = 1.0,
    ) -> list[raygeo.Geometry]:
        """Generate concentric inward offsets of a geometry.

        Each successive offset shrinks the boundary by `step`. Stops early
        when the enclosed area drops below `min_area` or `max_passes` is
        reached. Returns offsets outermost-first.

        :param geom: A closed geometry.
        :param step: Inward offset distance per pass.
        :param max_passes: Maximum number of offset passes (default 10).
        :param min_area: Minimum area to stop at (default 1.0).
        :returns: List of offset geometries, outermost first.
        :complexity: O(n * p) time, O(n) space where n is the number of contour vertices and p the number of passes
        """
"#,
    module = "raygeo.geo.algo.offset"
)]
#[pyfunction(name = "concentric_offsets")]
#[pyo3(signature = (geom, step, max_passes=10, min_area=1.0))]
fn concentric_offsets_py(
    py: Python<'_>,
    geom: pyo3::Py<super::super::Geometry>,
    step: f64,
    max_passes: usize,
    min_area: f64,
) -> Vec<super::super::Geometry> {
    let inner = geom.borrow(py).inner.clone();
    let results = concentric_offsets(&inner, step, max_passes, min_area);
    results
        .into_iter()
        .map(|g| super::super::Geometry { inner: g })
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def offset_contour_group(
        solid_path: collections.abc.Sequence[raygeo.geo.types.Point],
        hole_paths: collections.abc.Sequence[collections.abc.Sequence[raygeo.geo.types.Point]],
        offset: float,
        join_style: raygeo.geo.shape.polygon.JoinStyle = raygeo.geo.shape.polygon.JoinStyle.Miter,
    ) -> list[raygeo.geo.types.Polygon]:
        """Offset a solid contour with its hole contours.

        Offsets the solid outward (or inward for negative offset) while
        offsetting holes in the opposite direction and subtracting them
        from the solid result.

        :param solid_path: Outer boundary polygon as (x, y) points.
        :param hole_paths: List of hole polygons.
        :param offset: Offset distance (positive to inflate, negative to deflate).
        :param join_style: Corner join style (default: ``JoinStyle.Miter``).
        :returns: Offset polygon(s) with holes subtracted.
        :complexity: O(n log n)
        """
"#,
    module = "raygeo.geo.algo.offset"
)]
#[pyfunction(name = "offset_contour_group")]
#[pyo3(signature = (solid_path, hole_paths, offset, join_style = PyJoinStyle::Miter))]
fn offset_contour_group_py(
    solid_path: Vec<PyPoint2D>,
    hole_paths: Vec<Vec<(f64, f64)>>,
    offset: f64,
    join_style: PyJoinStyle,
) -> Vec<Vec<(f64, f64)>> {
    let solid = poly_to_points(solid_path);
    let holes: Vec<GeoPolygon> = hole_paths
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| GeoPoint::new(x, y)).collect())
        .collect();
    polygons_to_tuples(offset_contour_group(
        &solid,
        &holes,
        offset,
        join_style.into(),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def find_deepest_cores(
        regions: collections.abc.Sequence[raygeo.geo.types.Polygon],
        step_over: float,
    ) -> list[raygeo.geo.types.Point]:
        """Find the deepest (most open) regions of a polygon set.

        Iteratively offsets each polygon inward by step_over until all
        polygons collapse. Returns the centroids of the final polygons.

        :param regions: List of polygons to search.
        :param step_over: Inward offset distance per iteration.
        :returns: List of (x, y) centroid points.
        :complexity: O(n * k) where k is the number of iterations
        """
"#,
    module = "raygeo.geo.algo.offset"
)]
#[pyfunction(name = "find_deepest_cores")]
fn find_deepest_cores_py(
    regions: Vec<Vec<(f64, f64)>>,
    step_over: f64,
) -> Vec<(f64, f64)> {
    let polys: Vec<GeoPolygon> = regions
        .into_iter()
        .map(|v| v.into_iter().map(|(x, y)| GeoPoint::new(x, y)).collect())
        .collect();
    let cores = find_deepest_cores(&polys, step_over);
    cores.into_iter().map(|p| (p.x, p.y)).collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def compute_inset_region(
        boundary: collections.abc.Sequence[tuple[float, float]],
        radius: float,
        obstacles: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
    ) -> tuple[list[list[tuple[float, float]]], float]:
        """Compute the inset region: boundary shrunk by *radius*, minus
        obstacle buffers (each obstacle expanded by *radius*).

        :param boundary: Outer boundary polygon as a list of ``(x, y)`` points.
        :param radius: Inset / expansion radius.
        :param obstacles: List of obstacle polygons (default []).
        :returns: ``(region_polygons, total_area)``.
        :complexity: O((n + m) log(n + m)) where n and m are boundary and obstacle point counts
        """
"#,
    module = "raygeo.geo.algo.offset"
)]
#[pyfunction(name = "compute_inset_region")]
fn compute_inset_region_py(
    boundary: Vec<(f64, f64)>,
    radius: f64,
    obstacles: Vec<Vec<(f64, f64)>>,
) -> (Vec<Vec<(f64, f64)>>, f64) {
    let bnd: Vec<GeoPoint> = boundary
        .into_iter()
        .map(|(x, y)| GeoPoint::new(x, y))
        .collect();
    let obs: Vec<Vec<GeoPoint>> = obstacles
        .into_iter()
        .map(|o| o.into_iter().map(|(x, y)| GeoPoint::new(x, y)).collect())
        .collect();
    let (region, total) = compute_inset_region(&bnd, radius, &obs);
    (polygons_to_tuples(region), total)
}
