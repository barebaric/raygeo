pyo3_stub_gen::module_doc!("raygeo.geo.shape.arc", "{}", MODULE_DOC_ARC);

pub(crate) const MODULE_DOC_ARC: &str = "\
Arc geometry queries and conversions.

Provides bounding rectangle computation, intersection tests (arc-rect,
arc-circle, arc-polygons), arc linearization into line segments for
rendering or further processing, angle utilities (normalize, direction,
containment), and arc midpoint / closest-point lookups.
";

use super::super::flex_point::{
    edge_pairs3d_to_tuples, extract_polygons, point_to_tuple, tuple_to_point3d,
    PyPoint2D,
};
use super::super::types::{ArcClosestResult, Edge3D};
use crate::geo::shape::arc::{
    arc_through_point, does_arc_intersect_circle, does_arc_intersect_rect,
    get_arc_angles, get_arc_bounds, get_arc_closest_point, get_arc_direction,
    get_arc_length, get_arc_midpoint, get_arc_sweep, get_polyline_turn_sign,
    is_angle_between, is_arc_clockwise, is_arc_inside_polygons, linearize_arc,
    normalize_angle,
};
use crate::types::{Point, Point3D, Rect};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def arc_through_point(
        t_start: types.Point,
        t_end: types.Point,
        t_mid: types.Point,
        center: types.Point,
        radius: float,
    ) -> types.Polygon:
        """Build a circular arc through three points around a centre.

        Returns a polyline approximation of the arc from *t_start* to
        *t_end* that passes through *t_mid*, with the given centre and
        radius.

        :param t_start: Arc start point (x, y).
        :param t_end: Arc end point (x, y).
        :param t_mid: Point the arc must pass through (x, y).
        :param center: Arc centre (x, y).
        :param radius: Arc radius.
        :returns: Polyline approximation as list of (x, y) points.
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "arc_through_point")]
fn arc_through_point_py(
    t_start: (f64, f64),
    t_end: (f64, f64),
    t_mid: (f64, f64),
    center: (f64, f64),
    radius: f64,
) -> Vec<(f64, f64)> {
    arc_through_point(
        Point::new(t_start.0, t_start.1),
        Point::new(t_end.0, t_end.1),
        Point::new(t_mid.0, t_mid.1),
        Point::new(center.0, center.1),
        radius,
    )
    .into_iter()
    .map(|p| (p.x, p.y))
    .collect()
}

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "arc")?;
    m.setattr("__doc__", MODULE_DOC_ARC)?;

    register_functions!(
        m,
        arc_through_point_py,
        get_arc_bounds_py,
        get_arc_direction_py,
        get_arc_closest_point_py,
        get_arc_midpoint_py,
        get_arc_angles_py,
        does_arc_intersect_rect_py,
        does_arc_intersect_circle_py,
        is_arc_clockwise_py,
        is_arc_inside_polygons_py,
        is_angle_between_py,
        normalize_angle_py,
        linearize_arc_py,
        get_arc_length_py,
        get_arc_sweep_py,
        get_polyline_turn_sign_py,
    );

    shape_mod.add_submodule(&m)?;
    Ok(())
}

fn _arc_params_from_any(
    arc_cmd: &Bound<'_, PyAny>,
) -> PyResult<(Point3D, Point3D, Point3D)> {
    if let Ok(end) = arc_cmd.getattr("end") {
        let end: (f64, f64, f64) = end.extract()?;
        let center_offset: (f64, f64, f64) =
            arc_cmd.getattr("center_offset")?.extract()?;
        let normal: (f64, f64, f64) = arc_cmd.getattr("normal")?.extract()?;
        return Ok((
            Point3D::new(end.0, end.1, end.2),
            Point3D::new(center_offset.0, center_offset.1, center_offset.2),
            Point3D::new(normal.0, normal.1, normal.2),
        ));
    }
    if let Ok(row) = arc_cmd.extract::<Vec<f64>>() {
        if row.len() >= 10 {
            return Ok((
                Point3D::new(row[1], row[2], row[3]),
                Point3D::new(row[4], row[5], row[6]),
                Point3D::new(row[7], row[8], row[9]),
            ));
        }
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a command row or a MockArc-like namedtuple with end, center_offset, normal",
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_arc_bounds(
        start: types.Point,
        end: types.Point,
        center: types.Point,
        clockwise: bool,
    ) -> types.Rect:
        """Get the bounding rectangle of an arc.

        :param start: Arc start point (x, y).
        :param end: Arc end point (x, y).
        :param center: Arc center point (x, y).
        :param clockwise: Whether the arc is clockwise.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_bounds")]
#[pyo3(signature = (start, end, center, clockwise))]
fn get_arc_bounds_py(
    start: (f64, f64),
    end: (f64, f64),
    center: (f64, f64),
    clockwise: bool,
) -> (f64, f64, f64, f64) {
    let r = get_arc_bounds(
        Point::new(start.0, start.1),
        Point::new(end.0, end.1),
        Point::new(center.0, center.1),
        clockwise,
    );
    (r.min.x, r.min.y, r.max.x, r.max.y)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_arc_direction(
        center: types.Point,
        start: types.Point,
        mouse: types.Point,
    ) -> bool:
        """Get the direction (CW/CCW) of an arc at a mouse point.

        :param center: Arc center (x, y).
        :param start: Arc start point (x, y).
        :param mouse: Mouse point (x, y).
        :returns: True if clockwise, False if counter-clockwise.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_direction")]
fn get_arc_direction_py(
    center: (f64, f64),
    start: (f64, f64),
    mouse: (f64, f64),
) -> bool {
    get_arc_direction(
        Point::new(center.0, center.1),
        Point::new(start.0, start.1),
        Point::new(mouse.0, mouse.1),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_arc_length(
        start_pos: types.Point,
        end_pos: types.Point,
        center_offset: types.Point,
        clockwise: bool,
    ) -> float:
        """Compute the arc length of a circular arc.

        :param start_pos: Start point (x, y).
        :param end_pos: End point (x, y).
        :param center_offset: Center offset (i, j) from start.
        :param clockwise: True for clockwise, False for counter-clockwise.
        :returns: Arc length.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_length")]
fn get_arc_length_py(
    start_pos: (f64, f64),
    end_pos: (f64, f64),
    center_offset: (f64, f64),
    clockwise: bool,
) -> f64 {
    get_arc_length(
        Point::new(start_pos.0, start_pos.1),
        Point::new(end_pos.0, end_pos.1),
        Point::new(center_offset.0, center_offset.1),
        clockwise,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    def get_arc_sweep(start_angle: float, end_angle: float, clockwise: bool) -> float:
        """Compute the signed sweep angle for an arc.

        Handles direction (CW/CCW) and full-circle detection.

        :param start_angle: Start angle in radians.
        :param end_angle: End angle in radians.
        :param clockwise: Whether the arc is clockwise.
        :returns: Signed sweep angle in radians.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_sweep")]
fn get_arc_sweep_py(start_angle: f64, end_angle: f64, clockwise: bool) -> f64 {
    get_arc_sweep(start_angle, end_angle, clockwise)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_arc_closest_point(
        arc_cmd: typing.Any,
        start_pos: types.Point3D,
        x: float,
        y: float,
    ) -> typing.Optional[tuple[float, types.Point, float]]:
        """Get the closest point on an arc to a given point.

        :param arc_cmd: Arc command row or MockArc-like object.
        :param start_pos: Start position (x, y, z).
        :param x: X coordinate of target point.
        :param y: Y coordinate of target point.
        :returns: Tuple of (parameter, closest_point, distance) or None.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_closest_point")]
fn get_arc_closest_point_py(
    arc_cmd: &Bound<'_, PyAny>,
    start_pos: (f64, f64, f64),
    x: f64,
    y: f64,
) -> PyResult<Option<ArcClosestResult>> {
    let (end, center_offset, normal) = _arc_params_from_any(arc_cmd)?;
    Ok(get_arc_closest_point(
        end,
        center_offset,
        normal,
        tuple_to_point3d(start_pos),
        x,
        y,
    )
    .map(|(a, p, c)| (a, point_to_tuple(p), c)))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_arc_midpoint(
        start: types.Point,
        end: types.Point,
        center: types.Point,
        clockwise: bool,
    ) -> types.Point:
        """Get the midpoint of an arc.

        :param start: Arc start point (x, y).
        :param end: Arc end point (x, y).
        :param center: Arc center point (x, y).
        :param clockwise: Whether the arc is clockwise.
        :returns: Midpoint (x, y).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_midpoint")]
#[pyo3(signature = (start, end, center, clockwise))]
fn get_arc_midpoint_py(
    start: (f64, f64),
    end: (f64, f64),
    center: (f64, f64),
    clockwise: bool,
) -> (f64, f64) {
    point_to_tuple(get_arc_midpoint(
        Point::new(start.0, start.1),
        Point::new(end.0, end.1),
        Point::new(center.0, center.1),
        clockwise,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_arc_angles(
        start: types.Point,
        end: types.Point,
        center: types.Point,
        clockwise: bool,
    ) -> types.Point3D:
        """Get the start, end, and sweep angles of an arc.

        :param start: Arc start point (x, y).
        :param end: Arc end point (x, y).
        :param center: Arc center point (x, y).
        :param clockwise: Whether the arc is clockwise.
        :returns: Tuple of (start_angle, end_angle, sweep_angle) in radians.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_angles")]
#[pyo3(signature = (start, end, center, clockwise))]
fn get_arc_angles_py(
    start: (f64, f64),
    end: (f64, f64),
    center: (f64, f64),
    clockwise: bool,
) -> (f64, f64, f64) {
    get_arc_angles(
        Point::new(start.0, start.1),
        Point::new(end.0, end.1),
        Point::new(center.0, center.1),
        clockwise,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def does_arc_intersect_rect(
        arc_start: types.Point,
        arc_end: types.Point,
        arc_center: types.Point,
        clockwise: bool,
        rect: types.Rect,
    ) -> bool:
        """Check if an arc intersects a rectangle.

        :param arc_start: Arc start point (x, y).
        :param arc_end: Arc end point (x, y).
        :param arc_center: Arc center point (x, y).
        :param clockwise: Whether the arc is clockwise.
        :param rect: Rectangle (x_min, y_min, x_max, y_max).
        :returns: True if the arc intersects the rectangle.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "does_arc_intersect_rect")]
#[pyo3(signature = (arc_start, arc_end, arc_center, clockwise, rect))]
fn does_arc_intersect_rect_py(
    arc_start: (f64, f64),
    arc_end: (f64, f64),
    arc_center: (f64, f64),
    clockwise: bool,
    rect: (f64, f64, f64, f64),
) -> bool {
    does_arc_intersect_rect(
        Point::new(arc_start.0, arc_start.1),
        Point::new(arc_end.0, arc_end.1),
        Point::new(arc_center.0, arc_center.1),
        clockwise,
        Rect::new(rect.0, rect.1, rect.2, rect.3),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def does_arc_intersect_circle(
        arc_start: types.Point,
        arc_end: types.Point,
        arc_center: types.Point,
        clockwise: bool,
        circle_center: types.Point,
        circle_radius: float,
    ) -> bool:
        """Check if an arc intersects a circle.

        :param arc_start: Arc start point (x, y).
        :param arc_end: Arc end point (x, y).
        :param arc_center: Arc center point (x, y).
        :param clockwise: Whether the arc is clockwise.
        :param circle_center: Circle center (x, y).
        :param circle_radius: Circle radius.
        :returns: True if the arc intersects the circle.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "does_arc_intersect_circle")]
#[pyo3(signature = (arc_start, arc_end, arc_center, clockwise, circle_center, circle_radius))]
fn does_arc_intersect_circle_py(
    arc_start: (f64, f64),
    arc_end: (f64, f64),
    arc_center: (f64, f64),
    clockwise: bool,
    circle_center: (f64, f64),
    circle_radius: f64,
) -> bool {
    does_arc_intersect_circle(
        Point::new(arc_start.0, arc_start.1),
        Point::new(arc_end.0, arc_end.1),
        Point::new(arc_center.0, arc_center.1),
        clockwise,
        Point::new(circle_center.0, circle_center.1),
        circle_radius,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def is_arc_clockwise(
        points: collections.abc.Sequence[types.Point2DOr3D],
        center: types.Point2DOr3D,
    ) -> bool:
        """Check if an arc is clockwise.

        :param points: Sequence of (x, y) points on the arc.
        :param center: Arc center (x, y).
        :returns: True if the arc is clockwise.
        :complexity: O(n) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "is_arc_clockwise")]
fn is_arc_clockwise_py(points: Vec<PyPoint2D>, center: PyPoint2D) -> bool {
    let points_2d: Vec<Point> =
        points.iter().map(|p| Point::new(p.0, p.1)).collect();
    is_arc_clockwise(&points_2d, Point::new(center.0, center.1))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def is_arc_inside_polygons(
        arc_start: types.Point,
        arc_end: types.Point,
        arc_center: types.Point,
        clockwise: bool,
        polygons: typing.Any,
    ) -> bool:
        """Check if an arc is inside a set of polygons.

        :param arc_start: Arc start point (x, y).
        :param arc_end: Arc end point (x, y).
        :param arc_center: Arc center point (x, y).
        :param clockwise: Whether the arc is clockwise.
        :param polygons: List of polygons to check against.
        :returns: True if the arc is inside all polygons.
        :complexity: O(n * m) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "is_arc_inside_polygons")]
#[pyo3(signature = (arc_start, arc_end, arc_center, clockwise, polygons))]
fn is_arc_inside_polygons_py(
    arc_start: (f64, f64),
    arc_end: (f64, f64),
    arc_center: (f64, f64),
    clockwise: bool,
    polygons: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    let polygons_2d = extract_polygons(polygons)?;
    Ok(is_arc_inside_polygons(
        Point::new(arc_start.0, arc_start.1),
        Point::new(arc_end.0, arc_end.1),
        Point::new(arc_center.0, arc_center.1),
        clockwise,
        &polygons_2d,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    def is_angle_between(
        angle: float,
        start: float,
        end: float,
        clockwise: bool,
    ) -> bool:
        """Check if an angle is between two other angles.

        :param angle: Angle to test.
        :param start: Start angle.
        :param end: End angle.
        :param clockwise: Whether the arc is clockwise.
        :returns: True if angle is between start and end.
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "is_angle_between")]
#[pyo3(signature = (angle, start, end, clockwise))]
fn is_angle_between_py(
    angle: f64,
    start: f64,
    end: f64,
    clockwise: bool,
) -> bool {
    is_angle_between(angle, start, end, clockwise)
}

#[gen_stub_pyfunction(
    python = r#"
    def normalize_angle(angle: float) -> float:
        """Normalize an angle to the range [0, 2*pi).

        :param angle: Angle in radians.
        :returns: Normalized angle in [0, 2*pi).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "normalize_angle")]
fn normalize_angle_py(angle: f64) -> f64 {
    normalize_angle(angle)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def linearize_arc(
        arc_cmd: typing.Any,
        start_point: types.Point3D,
        resolution: float = 0.1,
    ) -> list[tuple[types.Point3D, types.Point3D]]:
        """Linearize an arc into line segments.

        :param arc_cmd: Arc command row or MockArc-like object.
        :param start_point: Start point (x, y, z).
        :param resolution: Maximum segment length.
        :returns: List of (p1, p2) segment pairs.
        :complexity: O(n) time, O(n) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "linearize_arc")]
#[pyo3(signature = (arc_cmd, start_point, resolution=0.1))]
fn linearize_arc_py(
    arc_cmd: &Bound<'_, PyAny>,
    start_point: (f64, f64, f64),
    resolution: f64,
) -> PyResult<Vec<Edge3D>> {
    let (end, center_offset, normal) = _arc_params_from_any(arc_cmd)?;
    let mut segments = Vec::new();
    linearize_arc(
        end,
        center_offset,
        normal,
        tuple_to_point3d(start_point),
        resolution,
        &mut segments,
    );
    Ok(edge_pairs3d_to_tuples(segments))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polyline_turn_sign(
        polyline: collections.abc.Sequence[types.Point],
    ) -> float:
        """Determine the turn direction of a polyline at its midpoint.

        Computes the cross product of the edge vectors just before and
        just after the midpoint vertex.  Returns ``+1.0`` for a
        counter-clockwise (left) turn and ``-1.0`` for a clockwise
        (right) turn.

        :param polyline: Open polyline as (x, y) points.
        :returns: ``+1.0`` (CCW) or ``-1.0`` (CW).
        :complexity: O(1) time, O(1) space
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_polyline_turn_sign")]
fn get_polyline_turn_sign_py(polyline: Vec<(f64, f64)>) -> f64 {
    let pts: Vec<Point> =
        polyline.iter().map(|&(x, y)| Point::new(x, y)).collect();
    get_polyline_turn_sign(&pts)
}
