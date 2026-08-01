pyo3_stub_gen::module_doc!("raygeo.geo.algo.fitting", "{}", MODULE_DOC_FITTING);

pub(crate) const MODULE_DOC_FITTING: &str = "\
Curve and primitive fitting algorithms.

Provides functions for fitting arcs, lines, circles, and beziers to
point sequences. Includes recursive fitting with primitives, polyline
linearization, and evaluating fitting quality (line and arc deviation).
";

use super::super::flex_point::{
    point_to_tuple, points3d_to_tuples, PyPoint2D, PyPoint3D,
};
use super::super::Geometry;
use crate::geo::algo::fitting::{
    arc_between_two_points, are_points_collinear_3d, fit_circle_to_3_points,
    fit_circle_to_points_3d, fit_points_recursive, fit_points_with_primitives,
    flatten_to_points_3d, generate_linking_arc, get_polyline_arc_deviation,
    get_polyline_line_deviation, linearize_geometry,
    project_circle_center_to_bisector,
};
use crate::geo::geometry::Geometry as CoreGeometry;
use crate::geo::types::{Point, Point3D};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "fitting")?;
    m.setattr("__doc__", MODULE_DOC_FITTING)?;

    register_functions!(
        m,
        are_points_collinear_py,
        fit_circle_to_3_points_py,
        fit_circle_to_points_py,
        project_circle_center_to_bisector_py,
        flatten_to_points_py,
        linearize_geometry_py,
        fit_points_recursive_py,
        fit_points_with_primitives_py,
        get_polyline_line_deviation_py,
        get_polyline_arc_deviation_py,
        arc_between_two_points_py,
        generate_linking_arc_py,
    );

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.fitting", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    from collections.abc import Sequence
    import raygeo.geo.types

    def are_points_collinear_3d(
        points: collections.abc.Sequence[types.Point3D],
        tolerance: float = 1e-6,
    ) -> bool:
        """Check if three or more points are collinear within tolerance.

        :param points: Sequence of 3D points.
        :param tolerance: Collinearity tolerance.
        :returns: True if points are collinear.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "are_points_collinear_3d")]
#[pyo3(signature = (points, tolerance=1e-6))]
fn are_points_collinear_py(points: Vec<PyPoint3D>, tolerance: f64) -> bool {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    are_points_collinear_3d(&pts, tolerance)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def fit_circle_to_3_points(
        p1: types.Point2DOr3D,
        p2: types.Point2DOr3D,
        p3: types.Point2DOr3D,
    ) -> typing.Optional[tuple[types.Point, float]]:
        """Fit a circle to three points.

        :param p1: First point (x, y) or (x, y, z).
        :param p2: Second point (x, y) or (x, y, z).
        :param p3: Third point (x, y) or (x, y, z).
        :returns: Tuple of (center, radius) or None.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "fit_circle_to_3_points")]
fn fit_circle_to_3_points_py(
    p1: PyPoint3D,
    p2: PyPoint3D,
    p3: PyPoint3D,
) -> Option<((f64, f64), f64)> {
    fit_circle_to_3_points(
        Point3D::new(p1.0, p1.1, p1.2),
        Point3D::new(p2.0, p2.1, p2.2),
        Point3D::new(p3.0, p3.1, p3.2),
    )
    .map(|(center, radius)| ((center.x, center.y), radius))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def fit_circle_to_points_3d(
        points: collections.abc.Sequence[types.Point3D],
    ) -> typing.Optional[tuple[types.Point, float, float]]:
        """Fit a circle to a set of points.

        :param points: Sequence of 3D points to fit.
        :returns: Tuple of (center, radius, error) or None.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "fit_circle_to_points_3d")]
fn fit_circle_to_points_py(
    points: Vec<PyPoint3D>,
) -> Option<((f64, f64), f64, f64)> {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    fit_circle_to_points_3d(&pts)
        .map(|(center, radius, error)| ((center.x, center.y), radius, error))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def project_circle_center_to_bisector(
        p1: types.Point2DOr3D,
        p2: types.Point2DOr3D,
        center: types.Point,
    ) -> types.Point:
        """Project a circle center onto the perpendicular bisector of two points.

        :param p1: First point (x, y) or (x, y, z).
        :param p2: Second point (x, y) or (x, y, z).
        :param center: Circle center to project.
        :returns: Projected center point (x, y).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "project_circle_center_to_bisector")]
fn project_circle_center_to_bisector_py(
    p1: PyPoint3D,
    p2: PyPoint3D,
    center: (f64, f64),
) -> (f64, f64) {
    point_to_tuple(project_circle_center_to_bisector(
        Point3D::new(p1.0, p1.1, p1.2),
        Point3D::new(p2.0, p2.1, p2.2),
        Point::new(center.0, center.1),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo
    import raygeo.geo.types

    def flatten_to_points_3d(
        geometry: geo.Geometry,
        tolerance: float,
    ) -> list[list[types.Point3D]]:
        """Flatten curves into linear segments.

        :param geometry: Geometry to flatten.
        :param tolerance: Flattening tolerance.
        :returns: List of flattened point segments.
        :complexity: O(n + m) time, O(m) space where n is the number of commands and m the number of output points
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "flatten_to_points_3d")]
fn flatten_to_points_py(
    geometry: &Geometry,
    tolerance: f64,
) -> Vec<Vec<(f64, f64, f64)>> {
    flatten_to_points_3d(geometry.inner.data(), tolerance)
        .into_iter()
        .map(points3d_to_tuples)
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo

    def linearize_geometry(
        geometry: geo.Geometry,
        tolerance: float,
    ) -> geo.Geometry:
        """Linearize geometry data into line segments.

        :param geometry: Geometry to linearize.
        :param tolerance: Linearization tolerance.
        :returns: Linearized Geometry.
        :complexity: O(n + m) time, O(m) space where n is the number of commands and m the number of output segments
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "linearize_geometry")]
fn linearize_geometry_py(geometry: &Geometry, tolerance: f64) -> Geometry {
    let mut result = geometry.inner.copy();
    result.data = linearize_geometry(&result.data, tolerance);
    Geometry { inner: result }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo
    import raygeo.geo.types

    def fit_points_recursive(
        points: collections.abc.Sequence[types.Point3D],
        tolerance: float,
        start_idx: int,
        end_idx: int,
    ) -> geo.Geometry:
        """Recursively fit points with line and arc primitives.

        :param points: Sequence of 3D points to fit.
        :param tolerance: Fitting tolerance.
        :param start_idx: Start index in the points array.
        :param end_idx: End index in the points array.
        :returns: Geometry of fitted commands.
        :complexity: O(n log n) average time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "fit_points_recursive")]
fn fit_points_recursive_py(
    points: Vec<PyPoint3D>,
    tolerance: f64,
    start_idx: usize,
    end_idx: usize,
) -> Geometry {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    let core = CoreGeometry {
        data: fit_points_recursive(&pts, tolerance, start_idx, end_idx),
        last_move_to: Point3D::new(0.0, 0.0, 0.0),
        uniform_scalable: true,
    };
    Geometry { inner: core }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo
    import raygeo.geo.types

    def fit_points_with_primitives(
        points: collections.abc.Sequence[types.Point3D],
        tolerance: float,
    ) -> geo.Geometry:
        """Fit a polyline of points with arc and line primitives.

        :param points: Sequence of 3D points to fit.
        :param tolerance: Fitting tolerance.
        :returns: Geometry of fitted commands.
        :complexity: O(n log n) average time, O(n) space
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "fit_points_with_primitives")]
fn fit_points_with_primitives_py(
    points: Vec<PyPoint3D>,
    tolerance: f64,
) -> Geometry {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    let core = CoreGeometry {
        data: fit_points_with_primitives(&pts, tolerance),
        last_move_to: Point3D::new(0.0, 0.0, 0.0),
        uniform_scalable: true,
    };
    Geometry { inner: core }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polyline_line_deviation(
        points: collections.abc.Sequence[types.Point3D],
        start: int,
        end: int,
    ) -> tuple[float, int]:
        """Get the maximum line deviation for a segment of a polyline.

        :param points: Sequence of 3D points.
        :param start: Start index.
        :param end: End index.
        :returns: Tuple of (max_deviation, index_of_max).
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "get_polyline_line_deviation")]
fn get_polyline_line_deviation_py(
    points: Vec<PyPoint3D>,
    start: usize,
    end: usize,
) -> (f64, usize) {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    get_polyline_line_deviation(&pts, start, end)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polyline_arc_deviation(
        points: collections.abc.Sequence[types.Point3D],
        center: types.Point,
        radius: float,
    ) -> float:
        """Get the maximum arc deviation for a set of points.

        :param points: Sequence of 3D points.
        :param center: Arc center (x, y).
        :param radius: Arc radius.
        :returns: Maximum deviation from the arc.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "get_polyline_arc_deviation")]
fn get_polyline_arc_deviation_py(
    points: Vec<PyPoint3D>,
    center: (f64, f64),
    radius: f64,
) -> f64 {
    let pts: Vec<Point3D> =
        points.iter().map(|p| Point3D::new(p.0, p.1, p.2)).collect();
    get_polyline_arc_deviation(&pts, Point::new(center.0, center.1), radius)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def generate_arc_between_two_points(
        p0: tuple[float, float],
        p1: tuple[float, float],
        offset: float,
        min_radius: float,
        z: float,
        resolution: float,
    ) -> typing.Optional[list[tuple[float, float, float]]]:
        """Fit a circular arc through two points with a perpendicular offset.

        Uses a third point offset perpendicularly from the chord midpoint to
        define the arc shape.  Returns a linearized arc polyline, or None if
        the radius would be below min_radius or the geometry is degenerate.

        :param p0: Start point (x, y).
        :param p1: End point (x, y).
        :param offset: Perpendicular offset from the chord midpoint.
        :param min_radius: Minimum allowed arc radius.
        :param z: Z-coordinate for all output points.
        :param resolution: Arc linearization resolution.
        :returns: List of 3D points forming the arc, or None.
        :complexity: O(n) time, O(n) space where n depends on arc length and resolution
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "generate_arc_between_two_points")]
#[pyo3(signature = (p0, p1, offset, min_radius, z, resolution))]
fn arc_between_two_points_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    offset: f64,
    min_radius: f64,
    z: f64,
    resolution: f64,
) -> Option<Vec<(f64, f64, f64)>> {
    arc_between_two_points(
        Point::new(p0.0, p0.1),
        Point::new(p1.0, p1.1),
        offset,
        min_radius,
        z,
        resolution,
    )
    .map(points3d_to_tuples)
}

#[gen_stub_pyfunction(
    python = r#"
    def generate_linking_arc(
        start: tuple[float, float, float],
        end: tuple[float, float, float],
        min_radius: float,
        z: float,
    ) -> list[tuple[float, float, float]]:
        """Generate a smooth linking arc between two points.

        Uses generate_arc_between_two_points internally with an offset derived from
        min_radius.  Falls back to a straight-line interpolation if no valid
        arc can be fit.

        :param start: Start 3D point (x, y, z).
        :param end: End 3D point (x, y, z).
        :param min_radius: Minimum allowed arc radius.
        :param z: Z-coordinate for all output points (overrides start.z / end.z).
        :returns: List of 3D points forming the linking arc.
        :complexity: O(n) time, O(n) space where n scales with chord_length / min_radius
        """
"#,
    module = "raygeo.geo.algo.fitting"
)]
#[pyfunction(name = "generate_linking_arc")]
#[pyo3(signature = (start, end, min_radius, z))]
fn generate_linking_arc_py(
    start: PyPoint3D,
    end: PyPoint3D,
    min_radius: f64,
    z: f64,
) -> Vec<(f64, f64, f64)> {
    points3d_to_tuples(generate_linking_arc(
        Point3D::new(start.0, start.1, start.2),
        Point3D::new(end.0, end.1, end.2),
        min_radius,
        z,
    ))
}
