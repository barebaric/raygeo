pyo3_stub_gen::module_doc!("raygeo.geo.shape.line", "{}", MODULE_DOC_LINE);

pub(crate) const MODULE_DOC_LINE: &str = "\
Line segment geometry queries.

Provides line-line intersection (infinite lines), line-segment intersection,
closest point on a line or segment to a given point, line-segment-vs-polygon
intersections, point-on-segment tests, point-in-rectangle tests, rectangle
containment checks, and angle-at-vertex computation.
";

use super::super::flex_point::{
    option_point_to_tuple, point_to_tuple, points3d_to_tuples,
    polygons_from_tuples,
};
use crate::geo::shape::line::{
    does_line_cross_polygon, does_line_segment_intersect_circle,
    does_line_segment_intersect_rect, get_angle_at_vertex, get_interior_angle,
    get_line_closest_point, get_line_line_intersection,
    get_line_segment_closest_point, get_line_segment_intersection,
    get_line_segment_length, get_line_segment_polygon_intersections,
    get_point_line_distance, get_segment_segment_distance,
    interpolated_segment_3d, is_point_on_segment, longest_line_through_point,
};
use crate::geo::types::{Point, Rect};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def interpolated_segment_3d(
        from_x: float,
        from_y: float,
        to_x: float,
        to_y: float,
        z: float,
        n: int,
    ) -> list[tuple[float, float, float]]:
        """Generate linearly interpolated 3D points along a 2D segment.

        Returns *n* points from *from* to *to* at height *z*.  The start is
        **not** included; the end *is* included.

        :param from_x: X coordinate of the start.
        :param from_y: Y coordinate of the start.
        :param to_x: X coordinate of the end.
        :param to_y: Y coordinate of the end.
        :param z: Z height for all points.
        :param n: Number of points to generate.
        :returns: List of ``(x, y, z)`` points.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "interpolated_segment_3d")]
fn interpolated_segment_3d_py(
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    z: f64,
    n: usize,
) -> Vec<(f64, f64, f64)> {
    points3d_to_tuples(interpolated_segment_3d(
        Point::new(from_x, from_y),
        Point::new(to_x, to_y),
        z,
        n,
    ))
}

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
        get_segment_segment_distance_py,
        is_point_on_line_segment_py,
        does_line_segment_intersect_rect_py,
        does_line_segment_intersect_circle_py,
        get_line_segment_polygon_intersections_py,
        does_line_cross_polygon_py,
        get_angle_at_vertex_py,
        get_interior_angle_py,
        get_line_segment_length_py,
        interpolated_segment_3d_py,
        longest_line_through_point_py,
    );

    shape_mod.add_submodule(&m)?;
    let sys_modules = shape_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape.line", &m)?;
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
        Rect::new(rect.0, rect.1, rect.2, rect.3),
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
    import collections.abc
    import raygeo.geo.types

    def get_line_segment_polygon_intersections(
        p1: types.Point,
        p2: types.Point,
        polygon: collections.abc.Sequence[types.Polygon],
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

    def does_line_cross_polygon(
        a: types.Point,
        b: types.Point,
        polygon: list[types.Point],
    ) -> bool:
        """Check if a line segment crosses the interior of a polygon.

        Returns ``True`` when the segment *strictly* crosses the polygon
        boundary — touching a vertex or grazing an edge at an endpoint is
        **not** considered a crossing.

        :param a: Segment start point (x, y).
        :param b: Segment end point (x, y).
        :param polygon: Polygon vertices [(x1, y1), (x2, y2), ...].
        :returns: ``True`` if the segment crosses the polygon interior.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "does_line_cross_polygon")]
fn does_line_cross_polygon_py(
    a: (f64, f64),
    b: (f64, f64),
    polygon: Vec<(f64, f64)>,
) -> bool {
    let pts: Vec<Point> =
        polygon.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    does_line_cross_polygon(Point::new(a.0, a.1), Point::new(b.0, b.1), &pts)
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

    def get_interior_angle(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
    ) -> float:
        """Interior angle at vertex ``p1`` formed by edges ``p0→p1`` and ``p1→p2``.

        Returns 0.0 when any two adjacent points coincide (degenerate input).

        :param p0: Previous point.
        :param p1: Vertex point.
        :param p2: Next point.
        :returns: Angle in radians in ``[0, π]``.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_interior_angle")]
fn get_interior_angle_py(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
) -> f64 {
    get_interior_angle(
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
#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_segment_segment_distance(
        a: tuple[float, float],
        b: tuple[float, float],
        c: tuple[float, float],
        d: tuple[float, float],
    ) -> float:
        """Minimum Euclidean distance between two line segments.

        :param a: Start of segment 1.
        :param b: End of segment 1.
        :param c: Start of segment 2.
        :param d: End of segment 2.
        :returns: Minimum distance between the two segments.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_segment_segment_distance")]
fn get_segment_segment_distance_py(
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
    d: (f64, f64),
) -> f64 {
    get_segment_segment_distance(
        Point::new(a.0, a.1),
        Point::new(b.0, b.1),
        Point::new(c.0, c.1),
        Point::new(d.0, d.1),
    )
}

#[pyfunction(name = "get_line_segment_length")]
fn get_line_segment_length_py(p1: (f64, f64), p2: (f64, f64)) -> f64 {
    get_line_segment_length(Point::new(p1.0, p1.1), Point::new(p2.0, p2.1))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def longest_line_through_point(
        pt: tuple[float, float],
        bbox: tuple[float, float, float, float],
    ) -> tuple[tuple[float, float], tuple[float, float]]:
        """Find the longest axis-aligned line through a point within a rectangle.

        Returns ``(start, end)`` — a horizontal line when the bounding box
        is wider than tall, otherwise a vertical line.

        :param pt: ``(x, y)`` point.
        :param bbox: ``(x_min, y_min, x_max, y_max)`` rectangle.
        :returns: ``((x1, y1), (x2, y2))`` start and end of the line.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "longest_line_through_point")]
fn longest_line_through_point_py(
    pt: (f64, f64),
    bbox: (f64, f64, f64, f64),
) -> ((f64, f64), (f64, f64)) {
    let (start, end) = longest_line_through_point(
        Point::new(pt.0, pt.1),
        Rect::new(bbox.0, bbox.1, bbox.2, bbox.3),
    );
    ((start.x, start.y), (end.x, end.y))
}
