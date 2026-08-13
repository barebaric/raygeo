pyo3_stub_gen::module_doc!("raygeo.geo.shape.point", "{}", MODULE_DOC_POINT);

pub(crate) const MODULE_DOC_POINT: &str = "\
Individual point operations.

Provides equality testing within a configurable tolerance, midpoint
computation between two points, 2D/3D get_circumcenter of three points,
applying a 4x4 affine transformation matrix to a single point, point
interpolation along a sequence, and moving-average smoothing of point
sequences.
";

use glam::{DMat4, DVec4};

use super::super::flex_point::{
    point3d_to_tuple, point_to_tuple, PyPoint2D, PyPoint3D,
};
use crate::geo::shape::point::are_points_equal_3d;
use crate::geo::shape::point::get_circumcenter;
use crate::geo::shape::point::get_circumcenter_3d;
use crate::geo::shape::point::get_midpoint_3d;
use crate::geo::shape::point::get_point_at_fraction;
use crate::geo::shape::point::get_points_moving_average;
use crate::geo::shape::point::rotate_point;
use crate::geo::shape::point::transform_point_3d;
use crate::geo::types::{Point, Point3D};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "point")?;
    m.setattr("__doc__", MODULE_DOC_POINT)?;

    register_functions!(
        m,
        midpoint_py,
        are_points_equal_py,
        transform_point_py,
        circumcenter_py,
        circumcenter_3d_py,
        rotate_point_py,
        point_at_fraction_py,
        points_moving_average_py,
    );

    shape_mod.add_submodule(&m)?;
    let sys_modules = shape_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.point", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def are_points_equal_3d(
        p1: types.Point3D,
        p2: types.Point3D,
        tolerance: float,
    ) -> bool:
        """Check if two 3D points are equal within tolerance.

        :param p1: First point (x, y, z).
        :param p2: Second point (x, y, z).
        :param tolerance: Maximum allowed difference.
        :returns: True if points are equal within tolerance.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "are_points_equal_3d")]
fn are_points_equal_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    tolerance: f64,
) -> bool {
    let arr1 = [p1.0, p1.1, p1.2];
    let arr2 = [p2.0, p2.1, p2.2];
    are_points_equal_3d(&arr1, &arr2, tolerance)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def transform_point_3d(
        matrix: collections.abc.Sequence[collections.abc.Sequence[float]],
        x: float,
        y: float,
        z: float,
    ) -> types.Point3D:
        """Apply an affine transformation matrix to a 3D point.

        :param matrix: 4x4 affine transformation matrix.
        :param x: X coordinate.
        :param y: Y coordinate.
        :param z: Z coordinate.
        :returns: Transformed point (x, y, z).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "transform_point_3d")]
fn transform_point_py(
    matrix: Vec<Vec<f64>>,
    x: f64,
    y: f64,
    z: f64,
) -> (f64, f64, f64) {
    let mat = DMat4::from_cols(
        DVec4::new(matrix[0][0], matrix[1][0], matrix[2][0], matrix[3][0]),
        DVec4::new(matrix[0][1], matrix[1][1], matrix[2][1], matrix[3][1]),
        DVec4::new(matrix[0][2], matrix[1][2], matrix[2][2], matrix[3][2]),
        DVec4::new(matrix[0][3], matrix[1][3], matrix[2][3], matrix[3][3]),
    );
    point3d_to_tuple(transform_point_3d(mat, Point3D::new(x, y, z)))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_circumcenter_3d(
        a: types.Point3D,
        b: types.Point3D,
        c: types.Point3D,
    ) -> typing.Optional[types.Point3D]:
        """Compute the get_circumcenter of three 3D points.

        Returns the center of the unique circle passing through all three
        points.  Returns ``None`` when the points are collinear.

        :param a: First point (x, y, z).
        :param b: Second point (x, y, z).
        :param c: Third point (x, y, z).
        :returns: Circumcenter (x, y, z) or ``None`` if collinear.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "get_circumcenter_3d")]
fn circumcenter_3d_py(
    a: PyPoint3D,
    b: PyPoint3D,
    c: PyPoint3D,
) -> Option<(f64, f64, f64)> {
    let a3 = Point3D::new(a.0, a.1, a.2);
    let b3 = Point3D::new(b.0, b.1, b.2);
    let c3 = Point3D::new(c.0, c.1, c.2);
    get_circumcenter_3d(a3, b3, c3).map(point3d_to_tuple)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_circumcenter(
        a: types.Point,
        b: types.Point,
        c: types.Point,
    ) -> tuple[types.Point, float]:
        """Compute the get_circumcenter and radius of three 2D points.

        Returns the center of the unique circle passing through all three
        points along with its radius. Returns ``((0.0, 0.0), -1.0)`` when
        the points are collinear.

        :param a: First point (x, y).
        :param b: Second point (x, y).
        :param c: Third point (x, y).
        :returns: ``(center, radius)`` where center is ``(x, y)``.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "get_circumcenter")]
fn circumcenter_py(
    a: PyPoint2D,
    b: PyPoint2D,
    c: PyPoint2D,
) -> ((f64, f64), f64) {
    let pa = Point::new(a.0, a.1);
    let pb = Point::new(b.0, b.1);
    let pc = Point::new(c.0, c.1);
    let (center, radius) = get_circumcenter(pa, pb, pc);
    (point_to_tuple(center), radius)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def rotate_point(
        point: types.Point,
        angle: float,
    ) -> types.Point:
        """Rotate a 2D point around the origin.

        :param point: Point (x, y) to rotate.
        :param angle: Rotation angle in radians (counter-clockwise).
        :returns: Rotated point (x, y).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "rotate_point")]
fn rotate_point_py(point: PyPoint2D, angle: f64) -> (f64, f64) {
    point_to_tuple(rotate_point(Point::new(point.0, point.1), angle))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_midpoint_3d(
        p1: types.Point3D,
        p2: types.Point3D,
    ) -> types.Point3D:
        """Get the midpoint between two 3D points.

        :param p1: First point (x, y, z).
        :param p2: Second point (x, y, z).
        :returns: Midpoint (x, y, z).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "get_midpoint_3d")]
fn midpoint_py(p1: PyPoint3D, p2: PyPoint3D) -> (f64, f64, f64) {
    let p1_3d = Point3D::new(p1.0, p1.1, p1.2);
    let p2_3d = Point3D::new(p2.0, p2.1, p2.2);
    point3d_to_tuple(get_midpoint_3d(p1_3d, p2_3d))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_point_at_fraction(
        points: list[types.Point],
        fraction: float,
    ) -> types.Point:
        """Get the point at a normalized fraction along a point sequence.

        Linear interpolation along the polyline formed by ``points``:
        fraction 0.0 is the first point, 1.0 is the last point.

        :param points: Sequence of points (x, y).
        :param fraction: Normalized position along the sequence [0, 1].
        :returns: Interpolated point (x, y).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "get_point_at_fraction")]
fn point_at_fraction_py(points: Vec<PyPoint2D>, fraction: f64) -> (f64, f64) {
    let pts: Vec<Point> = points.iter().map(|p| Point::new(p.0, p.1)).collect();
    point_to_tuple(get_point_at_fraction(&pts, fraction))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_points_moving_average(
        points: list[types.Point],
        radius: int,
    ) -> list[types.Point]:
        """Apply a moving average to a point sequence.

        Each output point is the mean of the input points within ``radius``
        positions on either side; the window shrinks and renormalizes near
        the sequence ends.

        :param points: Sequence of points (x, y).
        :param radius: Window radius in points.
        :returns: Smoothed sequence of points (x, y).
        :complexity: O(n * r) time, O(n) space where n is the point count and r the radius
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "get_points_moving_average")]
fn points_moving_average_py(
    points: Vec<PyPoint2D>,
    radius: usize,
) -> Vec<(f64, f64)> {
    let pts: Vec<Point> = points.iter().map(|p| Point::new(p.0, p.1)).collect();
    get_points_moving_average(&pts, radius)
        .into_iter()
        .map(point_to_tuple)
        .collect()
}
