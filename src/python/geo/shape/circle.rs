pyo3_stub_gen::module_doc!("raygeo.geo.shape.circle", "{}", MODULE_DOC_CIRCLE);

pub(crate) const MODULE_DOC_CIRCLE: &str = "\
Circle geometry queries.

Provides circle-circle and circle-rectangle intersection detection,
line-segment-vs-circle intersection points, circle-rectangle full-containment
checks, line-segment-vs-circle intersection, and point projection onto a
circle's circumference.
";

use crate::geo::shape::circle::{
    does_circle_intersect_rect, get_circle_circle_intersections,
    get_line_circle_intersections, is_circle_inside_rect,
    line_segment_intersects_circle, project_point_onto_circle,
};
use crate::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "circle")?;
    m.setattr("__doc__", MODULE_DOC_CIRCLE)?;

    register_functions!(
        m,
        get_circle_circle_intersections_py,
        get_line_circle_intersections_py,
        is_circle_inside_rect_py,
        does_circle_intersect_rect_py,
        line_segment_intersects_circle_py,
        project_point_onto_circle_py,
    );

    shape_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_circle_circle_intersections(
        c1: types.Point,
        r1: float,
        c2: types.Point,
        r2: float,
    ) -> types.Polygon:
        """Get intersection points of two circles.

        :param c1: Center of first circle (x, y).
        :param r1: Radius of first circle.
        :param c2: Center of second circle (x, y).
        :param r2: Radius of second circle.
        :returns: List of intersection points (x, y).
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "get_circle_circle_intersections")]
fn get_circle_circle_intersections_py(
    c1: Point,
    r1: f64,
    c2: Point,
    r2: f64,
) -> Vec<Point> {
    get_circle_circle_intersections(c1, r1, c2, r2)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_line_circle_intersections(
        p1: types.Point,
        p2: types.Point,
        center: types.Point,
        radius: float,
    ) -> types.Polygon:
        """Get intersection points of a line segment with a circle.

        :param p1: Start point of the line segment (x, y).
        :param p2: End point of the line segment (x, y).
        :param center: Circle center (x, y).
        :param radius: Circle radius.
        :returns: List of intersection points (x, y).
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "get_line_circle_intersections")]
fn get_line_circle_intersections_py(
    p1: Point,
    p2: Point,
    center: Point,
    radius: f64,
) -> Vec<Point> {
    get_line_circle_intersections(p1, p2, center, radius)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def is_circle_inside_rect(
        center: types.Point,
        radius: float,
        rect: types.Rect,
    ) -> bool:
        """Check if a circle is inside a rectangle.

        :param center: Circle center (x, y).
        :param radius: Circle radius.
        :param rect: Rectangle (x_min, y_min, x_max, y_max).
        :returns: True if the circle is fully inside the rectangle.
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "is_circle_inside_rect")]
fn is_circle_inside_rect_py(
    center: Point,
    radius: f64,
    rect: (f64, f64, f64, f64),
) -> bool {
    is_circle_inside_rect(center, radius, rect)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def does_circle_intersect_rect(
        center: types.Point,
        radius: float,
        rect: types.Rect,
    ) -> bool:
        """Check if a circle intersects a rectangle.

        :param center: Circle center (x, y).
        :param radius: Circle radius.
        :param rect: Rectangle (x_min, y_min, x_max, y_max).
        :returns: True if the circle intersects the rectangle.
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "does_circle_intersect_rect")]
fn does_circle_intersect_rect_py(
    center: Point,
    radius: f64,
    rect: (f64, f64, f64, f64),
) -> bool {
    does_circle_intersect_rect(center, radius, rect)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def line_segment_intersects_circle(
        p1: types.Point,
        p2: types.Point,
        circle_center: types.Point,
        circle_radius: float,
    ) -> bool:
        """Check if a line segment intersects a circle.

        :param p1: Start point of the line segment (x, y).
        :param p2: End point of the line segment (x, y).
        :param circle_center: Circle center (x, y).
        :param circle_radius: Circle radius.
        :returns: True if the line segment intersects the circle.
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "line_segment_intersects_circle")]
fn line_segment_intersects_circle_py(
    p1: Point,
    p2: Point,
    circle_center: Point,
    circle_radius: f64,
) -> bool {
    line_segment_intersects_circle(p1, p2, circle_center, circle_radius)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def project_point_onto_circle(
        point: types.Point,
        center: types.Point,
        radius: float,
    ) -> typing.Optional[types.Point]:
        """Project a point onto a circle.

        :param point: Point to project (x, y).
        :param center: Circle center (x, y).
        :param radius: Circle radius.
        :returns: Projected point on the circle (x, y).
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "project_point_onto_circle")]
fn project_point_onto_circle_py(
    point: Point,
    center: Point,
    radius: f64,
) -> Option<Point> {
    project_point_onto_circle(point, center, radius)
}
