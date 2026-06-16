pyo3_stub_gen::module_doc!("raygeo.geo.shape.line", "{}", MODULE_DOC_LINE);

pub(crate) const MODULE_DOC_LINE: &str = "\
Line segment geometry queries.

Provides line-line intersection (infinite lines), line-segment intersection,
closest point on a line or segment to a given point, line-segment-vs-polygon
intersections, point-on-segment tests, point-in-rectangle tests, rectangle
containment checks, and angle-at-vertex computation.
";

use super::super::flex_point::{
    option_point_to_tuple, point_to_tuple, polygons_from_tuples,
};
use crate::geo::shape::line::{
    does_line_segment_intersect_circle, does_line_segment_intersect_rect,
    get_angle_at_vertex, get_line_closest_point, get_line_line_intersection,
    get_line_segment_closest_point, get_line_segment_intersection,
    get_line_segment_length, get_line_segment_polygon_intersections,
    get_point_line_distance, is_point_on_segment,
};
use crate::types::{Point, Rect};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "line")?;
    m.setattr("__doc__", MODULE_DOC_LINE)?;

    register_functions!(
        m,
        get_line_line_intersection_py,
        get_line_segment_intersection_py,
        get_line_closest_point_py,
        get_line_segment_closest_point_py,
        get_point_line_distance_py,
        is_point_on_line_segment_py,
        does_line_segment_intersect_rect_py,
        does_line_segment_intersect_circle_py,
        get_line_segment_polygon_intersections_py,
        get_angle_at_vertex_py,
        get_line_segment_length_py,
    );

    shape_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_line_line_intersection(
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        p4: types.Point,
    ) -> typing.Optional[types.Point]:
        """Get the intersection of two infinite lines.

        :param p1: First point on line 1.
        :param p2: Second point on line 1.
        :param p3: First point on line 2.
        :param p4: Second point on line 2.
        :returns: Intersection point (x, y) or None.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_line_intersection")]
fn get_line_line_intersection_py(
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    p4: (f64, f64),
) -> Option<(f64, f64)> {
    option_point_to_tuple(get_line_line_intersection(
        Point::new(p1.0, p1.1),
        Point::new(p2.0, p2.1),
        Point::new(p3.0, p3.1),
        Point::new(p4.0, p4.1),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_line_segment_intersection(
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        p4: types.Point,
    ) -> typing.Optional[types.Point]:
        """Get the intersection of two line segments.

        :param p1: Start of segment 1.
        :param p2: End of segment 1.
        :param p3: Start of segment 2.
        :param p4: End of segment 2.
        :returns: Intersection point (x, y) or None.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_segment_intersection")]
fn get_line_segment_intersection_py(
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    p4: (f64, f64),
) -> Option<(f64, f64)> {
    option_point_to_tuple(get_line_segment_intersection(
        Point::new(p1.0, p1.1),
        Point::new(p2.0, p2.1),
        Point::new(p3.0, p3.1),
        Point::new(p4.0, p4.1),
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_line_closest_point(
        line_p1: types.Point,
        line_p2: types.Point,
        x: float,
        y: float,
    ) -> types.Point:
        """Get the closest point on an **infinite line** to a given point.
        The result may lie beyond the segment endpoints (unclamped projection).

        :param line_p1: First point on the line.
        :param line_p2: Second point on the line.
        :param x: X coordinate of target point.
        :param y: Y coordinate of target point.
        :returns: Closest point (x, y) on the infinite line.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_closest_point")]
fn get_line_closest_point_py(
    line_p1: (f64, f64),
    line_p2: (f64, f64),
    x: f64,
    y: f64,
) -> (f64, f64) {
    point_to_tuple(get_line_closest_point(
        Point::new(line_p1.0, line_p1.1),
        Point::new(line_p2.0, line_p2.1),
        x,
        y,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_line_segment_closest_point(
        seg_p1: types.Point,
        seg_p2: types.Point,
        x: float,
        y: float,
    ) -> tuple[float, types.Point, float]:
        """Get closest point on a line segment to a point.

        :param seg_p1: Start of the line segment.
        :param seg_p2: End of the line segment.
        :param x: X coordinate of target point.
        :param y: Y coordinate of target point.
        :returns: Tuple of (parameter, closest_point, distance).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_segment_closest_point")]
fn get_line_segment_closest_point_py(
    seg_p1: (f64, f64),
    seg_p2: (f64, f64),
    x: f64,
    y: f64,
) -> (f64, (f64, f64), f64) {
    let (t, p, d) = get_line_segment_closest_point(
        Point::new(seg_p1.0, seg_p1.1),
        Point::new(seg_p2.0, seg_p2.1),
        x,
        y,
    );
    (t, point_to_tuple(p), d)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_point_line_distance(
        point: types.Point,
        line_p1: types.Point,
        line_p2: types.Point,
    ) -> float:
        """Get the distance from a point to a **line segment**.
        The projection is clamped to the segment, so distance is measured to
        the nearest endpoint when the perpendicular falls outside.

        :param point: Point (x, y).
        :param line_p1: First point on the segment.
        :param line_p2: Second point on the segment.
        :returns: Distance (clamped to segment).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_point_line_distance")]
fn get_point_line_distance_py(
    point: (f64, f64),
    line_p1: (f64, f64),
    line_p2: (f64, f64),
) -> f64 {
    get_point_line_distance(
        Point::new(point.0, point.1),
        Point::new(line_p1.0, line_p1.1),
        Point::new(line_p2.0, line_p2.1),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def is_point_on_line_segment(
        point: types.Point,
        seg_p1: types.Point,
        seg_p2: types.Point,
    ) -> bool:
        """Check if a point is on a line segment.

        :param point: Point (x, y) to test.
        :param seg_p1: Start of the line segment.
        :param seg_p2: End of the line segment.
        :returns: True if the point lies on the segment.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "is_point_on_line_segment")]
fn is_point_on_line_segment_py(
    point: (f64, f64),
    seg_p1: (f64, f64),
    seg_p2: (f64, f64),
) -> bool {
    is_point_on_segment(
        Point::new(point.0, point.1),
        Point::new(seg_p1.0, seg_p1.1),
        Point::new(seg_p2.0, seg_p2.1),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def does_line_segment_intersect_rect(
        p1: types.Point,
        p2: types.Point,
        rect: types.Rect,
    ) -> bool:
        """Check if a line segment intersects a rectangle.

        :param p1: Start of the line segment.
        :param p2: End of the line segment.
        :param rect: Rectangle (x_min, y_min, x_max, y_max).
        :returns: True if the segment intersects the rectangle.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "does_line_segment_intersect_rect")]
fn does_line_segment_intersect_rect_py(
    p1: (f64, f64),
    p2: (f64, f64),
    rect: (f64, f64, f64, f64),
) -> bool {
    does_line_segment_intersect_rect(
        Point::new(p1.0, p1.1),
        Point::new(p2.0, p2.1),
        Rect(rect.0, rect.1, rect.2, rect.3),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def does_line_segment_intersect_circle(
        p1: types.Point,
        p2: types.Point,
        circle_center: types.Point,
        circle_radius: float,
    ) -> bool:
        """Check if a line segment intersects a circle.

        :param p1: Start of the line segment.
        :param p2: End of the line segment.
        :param circle_center: Circle center (x, y).
        :param circle_radius: Circle radius.
        :returns: True if the segment intersects the circle.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "does_line_segment_intersect_circle")]
fn does_line_segment_intersect_circle_py(
    p1: (f64, f64),
    p2: (f64, f64),
    circle_center: (f64, f64),
    circle_radius: f64,
) -> bool {
    does_line_segment_intersect_circle(
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

    def get_line_segment_polygon_intersections(
        p1: types.Point,
        p2: types.Point,
        polygon: typing.Sequence[types.Polygon],
    ) -> list[float]:
        """Get t-values where a line segment intersects a polygon.

        :param p1: Start of the line segment.
        :param p2: End of the line segment.
        :param polygon: Polygon to check against.
        :returns: List of t-values of intersection points.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_segment_polygon_intersections")]
fn get_line_segment_polygon_intersections_py(
    p1: (f64, f64),
    p2: (f64, f64),
    polygon: Vec<Vec<(f64, f64)>>,
) -> Vec<f64> {
    let poly = polygons_from_tuples(polygon);
    get_line_segment_polygon_intersections(
        Point::new(p1.0, p1.1),
        Point::new(p2.0, p2.1),
        &poly,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_angle_at_vertex(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
    ) -> float:
        """Compute the angle at vertex p1.

        :param p0: Previous point.
        :param p1: Vertex point.
        :param p2: Next point.
        :returns: Angle in radians.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_angle_at_vertex")]
fn get_angle_at_vertex_py(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
) -> f64 {
    get_angle_at_vertex(
        Point::new(p0.0, p0.1),
        Point::new(p1.0, p1.1),
        Point::new(p2.0, p2.1),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_line_segment_length(
        p1: types.Point,
        p2: types.Point,
    ) -> float:
        """Compute the length of a line segment.

        :param p1: Start point (x, y).
        :param p2: End point (x, y).
        :returns: Distance between the two points.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_segment_length")]
fn get_line_segment_length_py(p1: (f64, f64), p2: (f64, f64)) -> f64 {
    get_line_segment_length(Point::new(p1.0, p1.1), Point::new(p2.0, p2.1))
}
