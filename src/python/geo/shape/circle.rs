pyo3_stub_gen::module_doc!("raygeo.geo.shape.circle", "{}", MODULE_DOC_CIRCLE);

pub(crate) const MODULE_DOC_CIRCLE: &str = "\
Circle geometry queries.

Provides circle-circle and circle-rectangle intersection detection,
line-segment-vs-circle intersection points, circle-rectangle full-containment
checks, line-segment-vs-circle intersection, and point projection onto a
circle's circumference.
";

use super::super::flex_point::{option_point_to_tuple, points_to_tuples};
use crate::geo::shape::circle::{
    does_circle_intersect_rect, find_tangent_circle_centers,
    get_circle_circle_intersections, get_line_circle_intersections,
    is_circle_inside_rect, line_segment_intersects_circle,
    nearest_tangent_circle_on_polyline, project_point_onto_circle,
};
use crate::geo::types::{Point, Polygon, Rect};
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
        find_tangent_circle_centers_py,
        nearest_tangent_circle_on_polyline_py,
    );

    shape_mod.add_submodule(&m)?;
    let sys_modules = shape_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.circle", &m)?;
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
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "get_circle_circle_intersections")]
fn get_circle_circle_intersections_py(
    c1: (f64, f64),
    r1: f64,
    c2: (f64, f64),
    r2: f64,
) -> Vec<(f64, f64)> {
    points_to_tuples(get_circle_circle_intersections(
        Point::new(c1.0, c1.1),
        r1,
        Point::new(c2.0, c2.1),
        r2,
    ))
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
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "get_line_circle_intersections")]
fn get_line_circle_intersections_py(
    p1: (f64, f64),
    p2: (f64, f64),
    center: (f64, f64),
    radius: f64,
) -> Vec<(f64, f64)> {
    points_to_tuples(get_line_circle_intersections(
        Point::new(p1.0, p1.1),
        Point::new(p2.0, p2.1),
        Point::new(center.0, center.1),
        radius,
    ))
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
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "is_circle_inside_rect")]
fn is_circle_inside_rect_py(
    center: (f64, f64),
    radius: f64,
    rect: (f64, f64, f64, f64),
) -> bool {
    is_circle_inside_rect(
        Point::new(center.0, center.1),
        radius,
        Rect::new(rect.0, rect.1, rect.2, rect.3),
    )
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
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "does_circle_intersect_rect")]
fn does_circle_intersect_rect_py(
    center: (f64, f64),
    radius: f64,
    rect: (f64, f64, f64, f64),
) -> bool {
    does_circle_intersect_rect(
        Point::new(center.0, center.1),
        radius,
        Rect::new(rect.0, rect.1, rect.2, rect.3),
    )
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
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "line_segment_intersects_circle")]
fn line_segment_intersects_circle_py(
    p1: (f64, f64),
    p2: (f64, f64),
    circle_center: (f64, f64),
    circle_radius: f64,
) -> bool {
    line_segment_intersects_circle(
        Point::new(p1.0, p1.1),
        Point::new(p2.0, p2.1),
        Point::new(circle_center.0, circle_center.1),
        circle_radius,
    )
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
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "project_point_onto_circle")]
fn project_point_onto_circle_py(
    point: (f64, f64),
    center: (f64, f64),
    radius: f64,
) -> Option<(f64, f64)> {
    option_point_to_tuple(project_point_onto_circle(
        Point::new(point.0, point.1),
        Point::new(center.0, center.1),
        radius,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def find_tangent_circle_centers(
        pass_through: types.Point,
        seg_a: types.Point,
        seg_b: types.Point,
        radius: float,
    ) -> list[tuple[types.Point, types.Point]]:
        """Find circle centres that pass through a point and are tangent to a segment.

        :param pass_through: Point the circle must pass through (x, y).
        :param seg_a: Start of the tangent segment (x, y).
        :param seg_b: End of the tangent segment (x, y).
        :param radius: Circle radius.
        :returns: List of (centre, tangent_point) pairs.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "find_tangent_circle_centers")]
fn find_tangent_circle_centers_py(
    pass_through: (f64, f64),
    seg_a: (f64, f64),
    seg_b: (f64, f64),
    radius: f64,
) -> Vec<((f64, f64), (f64, f64))> {
    find_tangent_circle_centers(
        Point::new(pass_through.0, pass_through.1),
        Point::new(seg_a.0, seg_a.1),
        Point::new(seg_b.0, seg_b.1),
        radius,
    )
    .into_iter()
    .map(|(c, t)| ((c.x, c.y), (t.x, t.y)))
    .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def nearest_tangent_circle_on_polyline(
        point: types.Point,
        polyline: types.Polygon,
        radius: float,
        from_end: bool,
        containment: types.Polygon,
    ) -> typing.Optional[tuple[types.Point, types.Point, int]]:
        """Find nearest circle through a point tangent to a polyline.

        Searches segments of *polyline* for a circle of *radius* that
        passes through *point*, is tangent to a segment, and has its
        centre inside *containment*.  Returns the one whose tangent point
        is closest to the searched end.

        :param point: Point the circle must pass through (x, y).
        :param polyline: Polyline segments to search.
        :param radius: Circle radius.
        :param from_end: True to search from last vertex; False from first.
        :param containment: Centre must be inside this polygon.
        :returns: (centre, tangent_point, segment_index) or None.
        """
"#,
    module = "raygeo.geo.shape.circle"
)]
#[pyfunction(name = "nearest_tangent_circle_on_polyline")]
#[allow(clippy::type_complexity)]
fn nearest_tangent_circle_on_polyline_py(
    point: (f64, f64),
    polyline: Vec<(f64, f64)>,
    radius: f64,
    from_end: bool,
    containment: Vec<(f64, f64)>,
) -> Option<((f64, f64), (f64, f64), usize)> {
    let poly: Vec<Point> =
        polyline.into_iter().map(|p| Point::new(p.0, p.1)).collect();
    let cont: Polygon = containment
        .into_iter()
        .map(|p| Point::new(p.0, p.1))
        .collect();
    nearest_tangent_circle_on_polyline(
        Point::new(point.0, point.1),
        &poly,
        radius,
        from_end,
        &cont,
    )
    .map(|(c, t, i)| ((c.x, c.y), (t.x, t.y), i))
}
