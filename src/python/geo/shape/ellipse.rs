pyo3_stub_gen::module_doc!(
    "raygeo.geo.shape.ellipse",
    "{}",
    MODULE_DOC_ELLIPSE
);

pub(crate) const MODULE_DOC_ELLIPSE: &str = "\
Ellipse geometry queries.

Provides parametric point evaluation, uniform sampling over a full
turn, circumference membership and containment tests, and the
enclosed area, for an ellipse given by center, semi-axis lengths and
rotation.
";

use crate::geo::shape::ellipse::{
    get_ellipse_area, get_ellipse_point_at, get_ellipse_samples,
    is_point_inside_ellipse, is_point_on_ellipse,
};
use crate::geo::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "ellipse")?;
    m.setattr("__doc__", MODULE_DOC_ELLIPSE)?;

    register_functions!(
        m,
        get_ellipse_point_at_py,
        get_ellipse_samples_py,
        is_point_on_ellipse_py,
        is_point_inside_ellipse_py,
        get_ellipse_area_py,
    );

    shape_mod.add_submodule(&m)?;
    let sys_modules = shape_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.ellipse", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_ellipse_point_at(
        center: types.Point,
        rx: float,
        ry: float,
        rotation: float,
        t: float,
    ) -> types.Point:
        """Point on the ellipse at parameter ``t``.

        Evaluates ``center + R(rotation) * (rx * cos(t), ry * sin(t))``.

        :param center: Ellipse center (x, y).
        :param rx: Semi-axis length along the rotated x direction.
        :param ry: Semi-axis length along the rotated y direction.
        :param rotation: Rotation of the x axis in radians.
        :param t: Curve parameter in radians.
        :returns: Point (x, y) on the ellipse.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.ellipse"
)]
#[pyfunction(name = "get_ellipse_point_at")]
fn get_ellipse_point_at_py(
    center: (f64, f64),
    rx: f64,
    ry: f64,
    rotation: f64,
    t: f64,
) -> (f64, f64) {
    let point = get_ellipse_point_at(
        Point::new(center.0, center.1),
        rx,
        ry,
        rotation,
        t,
    );
    (point.x, point.y)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_ellipse_samples(
        center: types.Point,
        rx: float,
        ry: float,
        rotation: float,
        segments: int,
    ) -> list[types.Point]:
        """Sample the ellipse uniformly over a full turn.

        Returns ``segments + 1`` points from t = 0 through the closing
        point at t = 2*pi.

        :param center: Ellipse center (x, y).
        :param rx: Semi-axis length along the rotated x direction.
        :param ry: Semi-axis length along the rotated y direction.
        :param rotation: Rotation of the x axis in radians.
        :param segments: Number of segments (sample count - 1).
        :returns: List of points (x, y).
        :complexity: O(n) time, O(n) space for n samples
        """
"#,
    module = "raygeo.geo.shape.ellipse"
)]
#[pyfunction(name = "get_ellipse_samples")]
fn get_ellipse_samples_py(
    center: (f64, f64),
    rx: f64,
    ry: f64,
    rotation: f64,
    segments: usize,
) -> Vec<(f64, f64)> {
    get_ellipse_samples(
        Point::new(center.0, center.1),
        rx,
        ry,
        rotation,
        segments,
    )
    .into_iter()
    .map(|p| (p.x, p.y))
    .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def is_point_on_ellipse(
        point: types.Point,
        center: types.Point,
        rx: float,
        ry: float,
        rotation: float,
        tolerance: float,
    ) -> bool:
        """Check whether a point lies on the ellipse circumference.

        The tolerance is interpreted in world units along the radial
        direction, scaled by the smaller semi-axis.

        :param point: Query point (x, y).
        :param center: Ellipse center (x, y).
        :param rx: Semi-axis length along the rotated x direction.
        :param ry: Semi-axis length along the rotated y direction.
        :param rotation: Rotation of the x axis in radians.
        :param tolerance: World-unit distance tolerance.
        :returns: True if the point lies on the circumference within tolerance.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.ellipse"
)]
#[pyfunction(name = "is_point_on_ellipse")]
fn is_point_on_ellipse_py(
    point: (f64, f64),
    center: (f64, f64),
    rx: f64,
    ry: f64,
    rotation: f64,
    tolerance: f64,
) -> bool {
    is_point_on_ellipse(
        Point::new(point.0, point.1),
        Point::new(center.0, center.1),
        rx,
        ry,
        rotation,
        tolerance,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def is_point_inside_ellipse(
        point: types.Point,
        center: types.Point,
        rx: float,
        ry: float,
        rotation: float,
    ) -> bool:
        """Check whether a point lies inside or on the ellipse.

        :param point: Query point (x, y).
        :param center: Ellipse center (x, y).
        :param rx: Semi-axis length along the rotated x direction.
        :param ry: Semi-axis length along the rotated y direction.
        :param rotation: Rotation of the x axis in radians.
        :returns: True if the point is inside or on the ellipse.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.ellipse"
)]
#[pyfunction(name = "is_point_inside_ellipse")]
fn is_point_inside_ellipse_py(
    point: (f64, f64),
    center: (f64, f64),
    rx: f64,
    ry: f64,
    rotation: f64,
) -> bool {
    is_point_inside_ellipse(
        Point::new(point.0, point.1),
        Point::new(center.0, center.1),
        rx,
        ry,
        rotation,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    def get_ellipse_area(rx: float, ry: float) -> float:
        """Area enclosed by the ellipse.

        :param rx: Semi-axis length along x.
        :param ry: Semi-axis length along y.
        :returns: Enclosed area (pi * rx * ry).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.ellipse"
)]
#[pyfunction(name = "get_ellipse_area")]
fn get_ellipse_area_py(rx: f64, ry: f64) -> f64 {
    get_ellipse_area(rx, ry)
}
