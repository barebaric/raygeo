pyo3_stub_gen::module_doc!("raygeo.geo.shape.point", "{}", MODULE_DOC_POINT);

pub(crate) const MODULE_DOC_POINT: &str = "\
Individual point operations.

Provides equality testing within a configurable tolerance, midpoint
computation between two points, and applying a 4x4 affine transformation
matrix to a single point.
";

use super::super::flex_point::PyPoint3D;
use crate::geo::shape::point::are_points_equal;
use crate::geo::shape::point::midpoint;
use crate::geo::shape::point::transform_point;
use crate::types::Point3D;
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
    );

    shape_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def are_points_equal(
        p1: types.Point3D,
        p2: types.Point3D,
        tolerance: float,
    ) -> bool:
        """Check if two 3D points are equal within tolerance.

        :param p1: First point (x, y, z).
        :param p2: Second point (x, y, z).
        :param tolerance: Maximum allowed difference.
        :returns: True if points are equal within tolerance.
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "are_points_equal")]
fn are_points_equal_py(
    p1: (f64, f64, f64),
    p2: (f64, f64, f64),
    tolerance: f64,
) -> bool {
    let arr1 = [p1.0, p1.1, p1.2];
    let arr2 = [p2.0, p2.1, p2.2];
    are_points_equal(&arr1, &arr2, tolerance)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def transform_point(
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
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "transform_point")]
fn transform_point_py(
    matrix: Vec<Vec<f64>>,
    x: f64,
    y: f64,
    z: f64,
) -> Point3D {
    let mat: [[f64; 4]; 4] = [
        [matrix[0][0], matrix[0][1], matrix[0][2], matrix[0][3]],
        [matrix[1][0], matrix[1][1], matrix[1][2], matrix[1][3]],
        [matrix[2][0], matrix[2][1], matrix[2][2], matrix[2][3]],
        [matrix[3][0], matrix[3][1], matrix[3][2], matrix[3][3]],
    ];
    transform_point(&mat, x, y, z)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def midpoint(
        p1: types.Point3D,
        p2: types.Point3D,
    ) -> types.Point3D:
        """Get the midpoint between two 3D points.

        :param p1: First point (x, y, z).
        :param p2: Second point (x, y, z).
        :returns: Midpoint (x, y, z).
        """
"#,
    module = "raygeo.geo.shape.point"
)]
#[pyfunction(name = "midpoint")]
fn midpoint_py(p1: PyPoint3D, p2: PyPoint3D) -> Point3D {
    let p1_3d = Point3D(p1.0, p1.1, p1.2);
    let p2_3d = Point3D(p2.0, p2.1, p2.2);
    midpoint(p1_3d, p2_3d)
}
