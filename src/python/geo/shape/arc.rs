pyo3_stub_gen::module_doc!("raygeo.geo.shape.arc", "{}", MODULE_DOC_ARC);

pub(crate) const MODULE_DOC_ARC: &str = "\
Arc geometry queries and conversions.

Provides bounding rectangle computation, intersection tests (arc-rect,
arc-circle, arc-polygons), arc linearization into line segments for
rendering or further processing, angle utilities (normalize, direction,
containment), and arc midpoint / closest-point lookups.
";

use super::super::flex_point::{extract_polygons, PyPoint2D};
use crate::geo::shape::arc::{
    does_arc_intersect_circle, does_arc_intersect_rect, get_arc_angles,
    get_arc_bounds, get_arc_closest_point, get_arc_direction, get_arc_length,
    get_arc_midpoint, get_arc_sweep, is_angle_between, is_arc_clockwise,
    is_arc_inside_polygons, linearize_arc, normalize_angle,
};
use crate::types::{Point, Point3D, Rect, Segment3D};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(shape_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = shape_mod.py();
    let m = PyModule::new(py, "arc")?;
    m.setattr("__doc__", MODULE_DOC_ARC)?;

    register_functions!(
        m,
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
    );

    shape_mod.add_submodule(&m)?;
    Ok(())
}

#[allow(clippy::type_complexity)]
fn _arc_params_from_any(
    arc_cmd: &Bound<'_, PyAny>,
) -> PyResult<(Point3D, Point, bool)> {
    if let Ok(end) = arc_cmd.getattr("end") {
        let end: Point3D = end.extract()?;
        let center_offset: Point =
            arc_cmd.getattr("center_offset")?.extract()?;
        let clockwise: bool = arc_cmd.getattr("clockwise")?.extract()?;
        return Ok((end, center_offset, clockwise));
    }
    if let Ok(row) = arc_cmd.extract::<Vec<f64>>() {
        if row.len() >= 7 {
            return Ok((
                Point3D(row[1], row[2], row[3]),
                Point(row[4], row[5]),
                row[6] > 0.5,
            ));
        }
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a command row or a MockArc-like namedtuple with end, center_offset, clockwise",
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_bounds")]
#[pyo3(signature = (start, end, center, clockwise))]
fn get_arc_bounds_py(
    start: Point,
    end: Point,
    center: Point,
    clockwise: bool,
) -> (f64, f64, f64, f64) {
    let r = get_arc_bounds(start, end, center, clockwise);
    (r.0, r.1, r.2, r.3)
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_direction")]
fn get_arc_direction_py(center: Point, start: Point, mouse: Point) -> bool {
    get_arc_direction(center, start, mouse)
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_length")]
fn get_arc_length_py(
    start_pos: Point,
    end_pos: Point,
    center_offset: Point,
    clockwise: bool,
) -> f64 {
    get_arc_length(start_pos, end_pos, center_offset, clockwise)
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_closest_point")]
fn get_arc_closest_point_py(
    arc_cmd: &Bound<'_, PyAny>,
    start_pos: Point3D,
    x: f64,
    y: f64,
) -> PyResult<Option<(f64, Point, f64)>> {
    let (end, center_offset, clockwise) = _arc_params_from_any(arc_cmd)?;
    Ok(get_arc_closest_point(
        end,
        center_offset,
        clockwise,
        start_pos,
        x,
        y,
    ))
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_midpoint")]
#[pyo3(signature = (start, end, center, clockwise))]
fn get_arc_midpoint_py(
    start: Point,
    end: Point,
    center: Point,
    clockwise: bool,
) -> Point {
    get_arc_midpoint(start, end, center, clockwise)
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "get_arc_angles")]
#[pyo3(signature = (start, end, center, clockwise))]
fn get_arc_angles_py(
    start: Point,
    end: Point,
    center: Point,
    clockwise: bool,
) -> (f64, f64, f64) {
    get_arc_angles(start, end, center, clockwise)
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "does_arc_intersect_rect")]
#[pyo3(signature = (arc_start, arc_end, arc_center, clockwise, rect))]
fn does_arc_intersect_rect_py(
    arc_start: Point,
    arc_end: Point,
    arc_center: Point,
    clockwise: bool,
    rect: (f64, f64, f64, f64),
) -> bool {
    does_arc_intersect_rect(
        arc_start,
        arc_end,
        arc_center,
        clockwise,
        Rect(rect.0, rect.1, rect.2, rect.3),
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "does_arc_intersect_circle")]
#[pyo3(signature = (arc_start, arc_end, arc_center, clockwise, circle_center, circle_radius))]
fn does_arc_intersect_circle_py(
    arc_start: Point,
    arc_end: Point,
    arc_center: Point,
    clockwise: bool,
    circle_center: Point,
    circle_radius: f64,
) -> bool {
    does_arc_intersect_circle(
        arc_start,
        arc_end,
        arc_center,
        clockwise,
        circle_center,
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "is_arc_clockwise")]
fn is_arc_clockwise_py(points: Vec<PyPoint2D>, center: PyPoint2D) -> bool {
    let points_2d: Vec<Point> =
        points.iter().map(|p| Point(p.0, p.1)).collect();
    is_arc_clockwise(&points_2d, Point(center.0, center.1))
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "is_arc_inside_polygons")]
#[pyo3(signature = (arc_start, arc_end, arc_center, clockwise, polygons))]
fn is_arc_inside_polygons_py(
    arc_start: Point,
    arc_end: Point,
    arc_center: Point,
    clockwise: bool,
    polygons: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    let polygons_2d = extract_polygons(polygons)?;
    Ok(is_arc_inside_polygons(
        arc_start,
        arc_end,
        arc_center,
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
        """
"#,
    module = "raygeo.geo.shape.arc"
)]
#[pyfunction(name = "linearize_arc")]
#[pyo3(signature = (arc_cmd, start_point, resolution=0.1))]
fn linearize_arc_py(
    arc_cmd: &Bound<'_, PyAny>,
    start_point: Point3D,
    resolution: f64,
) -> PyResult<Vec<Segment3D>> {
    let (end, center_offset, clockwise) = _arc_params_from_any(arc_cmd)?;
    Ok(linearize_arc(
        end,
        center_offset,
        clockwise,
        start_point,
        resolution,
    ))
}
