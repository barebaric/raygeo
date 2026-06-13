pyo3_stub_gen::module_doc!("raygeo.geo.shape", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Primitive shape operations — arc, bezier, circle, line, point, polygon, rect.

Provides functions for geometric queries on primitive shapes including
bounding boxes, intersection tests, containment checks, linearization,
and affine transformations.
";

pyo3_stub_gen::module_doc!("raygeo.geo.shape.arc", "{}", MODULE_DOC_ARC);

pub(crate) const MODULE_DOC_ARC: &str = "\
Arc geometry queries and conversions.

Provides bounding rectangle computation, intersection tests (arc-rect,
arc-circle, arc-polygons), arc linearization into line segments for
rendering or further processing, angle utilities (normalize, direction,
containment), and arc midpoint / closest-point lookups.
";

pyo3_stub_gen::module_doc!("raygeo.geo.shape.bezier", "{}", MODULE_DOC_BEZIER);

pub(crate) const MODULE_DOC_BEZIER: &str = "\
Cubic bezier curve queries and conversions.

Provides point evaluation at a parameter t, splitting into two halves,
bounding rectangle computation, flattening to line segments (both
fixed-step and adaptive subdivision), rectangle clipping, flatness
testing, perpendicular distance measurement, and conversion from
cubic to quadratic form.
";

pyo3_stub_gen::module_doc!("raygeo.geo.shape.circle", "{}", MODULE_DOC_CIRCLE);

pub(crate) const MODULE_DOC_CIRCLE: &str = "\
Circle geometry queries.

Provides circle-circle and circle-rectangle intersection detection,
line-segment-vs-circle intersection points, circle-rectangle full-containment
checks, line-segment-vs-circle intersection, and point projection onto a
circle's circumference.
";

pyo3_stub_gen::module_doc!("raygeo.geo.shape.line", "{}", MODULE_DOC_LINE);

pub(crate) const MODULE_DOC_LINE: &str = "\
Line segment geometry queries.

Provides line-line intersection (infinite lines), line-segment intersection,
closest point on a line or segment to a given point, line-segment-vs-polygon
intersections, point-on-segment tests, point-in-rectangle tests, rectangle
containment checks, and angle-at-vertex computation.
";

pyo3_stub_gen::module_doc!("raygeo.geo.shape.point", "{}", MODULE_DOC_POINT);

pub(crate) const MODULE_DOC_POINT: &str = "\
Individual point operations.

Provides equality testing within a configurable tolerance, midpoint
computation between two points, and applying a 4x4 affine transformation
matrix to a single point.
";

pyo3_stub_gen::module_doc!(
    "raygeo.geo.shape.polygon",
    "{}",
    MODULE_DOC_POLYGON
);

pub(crate) const MODULE_DOC_POLYGON: &str = "\
Polygon manipulation functions.

Provides area (signed and unsigned), perimeter, bounding box, centroid,
convex hull, point-in-polygon containment test, edge extraction,
convexity testing, boolean operations (union, intersection, difference),
offset (inflate or deflate), cleaning (removing near-duplicate vertices),
normalisation (outer CCW / inner CW winding order), and transformations
(translate, rotate, scale, flip). All functions accept list-of-tuples
input; many also provide numpy array variants.
";

pyo3_stub_gen::module_doc!("raygeo.geo.shape.rect", "{}", MODULE_DOC_RECT);

pub(crate) const MODULE_DOC_RECT: &str = "\
Rectangle intersection and containment tests.

Provides functions to test whether two axis-aligned rectangles intersect
and whether one rectangle fully contains another.
";

use super::flex_point::{
    extract_polygons, poly_to_points, PyPoint2D, PyPoint3D,
};
use crate::geo::shape::arc::is_arc_clockwise;
use crate::geo::shape::arc::{
    does_arc_intersect_circle, does_arc_intersect_rect, get_arc_angles,
    get_arc_bounds, get_arc_closest_point, get_arc_direction, get_arc_length,
    get_arc_midpoint, is_angle_between, is_arc_inside_polygons, linearize_arc,
    normalize_angle,
};
use crate::geo::shape::bezier::{
    clip_bezier_with_rect, convert_cubic_bezier_to_quadratic, flatten_bezier,
    get_bezier_bounds, get_bezier_flatness_sq, get_bezier_length,
    get_bezier_point_at, get_bezier_rect_intersections,
    get_perpendicular_dist_sq, is_bezier_inside_polygons, linearize_bezier,
    linearize_bezier_adaptive, linearize_bezier_segment, split_bezier,
};
use crate::geo::shape::circle::{
    does_circle_intersect_rect, get_circle_circle_intersections,
    get_line_circle_intersections, is_circle_inside_rect,
    line_segment_intersects_circle, project_point_onto_circle,
};
use crate::geo::shape::line::get_angle_at_vertex;
use crate::geo::shape::line::{
    does_line_segment_intersect_circle, does_line_segment_intersect_rect,
    does_rect_contain_rect, get_line_closest_point, get_line_line_intersection,
    get_line_segment_closest_point, get_line_segment_intersection,
    get_line_segment_length, get_line_segment_polygon_intersections,
    get_point_line_distance, is_point_inside_rect, is_point_on_segment,
};
use crate::geo::shape::point::are_points_equal;
use crate::geo::shape::point::midpoint;
use crate::geo::shape::point::transform_point;
use crate::geo::shape::polygon::is_polygon_clockwise;
use crate::geo::shape::polygon::{
    clean_polygon, flip_polygon, flip_polygons, get_polygon_bounds,
    get_polygon_centroid, get_polygon_convex_hull, get_polygon_edges,
    get_polygon_group_bounds, get_polygon_perimeter, get_polygon_signed_area,
    get_polygons_difference, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_intersection,
    get_polygons_union, is_almost_equal, is_point_inside_polygon,
    is_polygon_convex, normalize_polygons, offset_polygon, point_line_distance,
    polygons_intersect, rotate_polygon, rotate_polygons, scale_polygon,
    to_clipper_from_points, translate_bounds, translate_polygon,
    translate_polygons,
};
use crate::geo::shape::rect::do_rects_intersect;
use crate::{BezierSplit, Point, Segment3D};
use numpy::{PyArray2, PyArrayMethods, PyUntypedArrayMethods};
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

#[allow(clippy::type_complexity)]
fn _arc_params_from_any(
    arc_cmd: &Bound<'_, PyAny>,
) -> PyResult<((f64, f64, f64), (f64, f64), bool)> {
    if let Ok(end) = arc_cmd.getattr("end") {
        let end: (f64, f64, f64) = end.extract()?;
        let center_offset: (f64, f64) =
            arc_cmd.getattr("center_offset")?.extract()?;
        let clockwise: bool = arc_cmd.getattr("clockwise")?.extract()?;
        return Ok((end, center_offset, clockwise));
    }
    if let Ok(row) = arc_cmd.extract::<Vec<f64>>() {
        if row.len() >= 7 {
            return Ok((
                (row[1], row[2], row[3]),
                (row[4], row[5]),
                row[6] > 0.5,
            ));
        }
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a command row or a MockArc-like namedtuple with end, center_offset, clockwise",
    ))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let shape_mod = PyModule::new(py, "shape")?;
    shape_mod.setattr("__doc__", MODULE_DOC)?;

    let arc_mod = PyModule::new(py, "arc")?;
    arc_mod.setattr("__doc__", MODULE_DOC_ARC)?;
    arc_mod
        .add_function(wrap_pyfunction!(get_arc_bounds_py, arc_mod.clone())?)?;
    arc_mod.add_function(wrap_pyfunction!(
        get_arc_direction_py,
        arc_mod.clone()
    )?)?;
    arc_mod.add_function(wrap_pyfunction!(
        get_arc_closest_point_py,
        arc_mod.clone()
    )?)?;
    arc_mod.add_function(wrap_pyfunction!(
        get_arc_midpoint_py,
        arc_mod.clone()
    )?)?;
    arc_mod
        .add_function(wrap_pyfunction!(get_arc_angles_py, arc_mod.clone())?)?;
    arc_mod.add_function(wrap_pyfunction!(
        does_arc_intersect_rect_py,
        arc_mod.clone()
    )?)?;
    arc_mod.add_function(wrap_pyfunction!(
        does_arc_intersect_circle_py,
        arc_mod.clone()
    )?)?;
    arc_mod.add_function(wrap_pyfunction!(
        is_arc_clockwise_py,
        arc_mod.clone()
    )?)?;
    arc_mod.add_function(wrap_pyfunction!(
        is_arc_inside_polygons_py,
        arc_mod.clone()
    )?)?;
    arc_mod.add_function(wrap_pyfunction!(
        is_angle_between_py,
        arc_mod.clone()
    )?)?;
    arc_mod
        .add_function(wrap_pyfunction!(normalize_angle_py, arc_mod.clone())?)?;
    arc_mod
        .add_function(wrap_pyfunction!(linearize_arc_py, arc_mod.clone())?)?;
    arc_mod
        .add_function(wrap_pyfunction!(get_arc_length_py, arc_mod.clone())?)?;
    shape_mod.add_submodule(&arc_mod)?;

    let bezier_mod = PyModule::new(py, "bezier")?;
    bezier_mod.setattr("__doc__", MODULE_DOC_BEZIER)?;
    bezier_mod.add_function(wrap_pyfunction!(
        get_bezier_point_at_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod
        .add_function(wrap_pyfunction!(split_bezier_py, bezier_mod.clone())?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        get_bezier_bounds_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        get_bezier_rect_intersections_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        clip_bezier_with_rect_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        convert_cubic_bezier_to_quadratic_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        is_bezier_inside_polygons_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        linearize_bezier_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        linearize_bezier_adaptive_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        linearize_bezier_segment_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        flatten_bezier_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        get_bezier_flatness_sq_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        get_perpendicular_dist_sq_py,
        bezier_mod.clone()
    )?)?;
    bezier_mod.add_function(wrap_pyfunction!(
        get_bezier_length_py,
        bezier_mod.clone()
    )?)?;
    shape_mod.add_submodule(&bezier_mod)?;

    let circle_mod = PyModule::new(py, "circle")?;
    circle_mod.setattr("__doc__", MODULE_DOC_CIRCLE)?;
    circle_mod.add_function(wrap_pyfunction!(
        get_circle_circle_intersections_py,
        circle_mod.clone()
    )?)?;
    circle_mod.add_function(wrap_pyfunction!(
        get_line_circle_intersections_py,
        circle_mod.clone()
    )?)?;
    circle_mod.add_function(wrap_pyfunction!(
        is_circle_inside_rect_py,
        circle_mod.clone()
    )?)?;
    circle_mod.add_function(wrap_pyfunction!(
        does_circle_intersect_rect_py,
        circle_mod.clone()
    )?)?;
    circle_mod.add_function(wrap_pyfunction!(
        line_segment_intersects_circle_py,
        circle_mod.clone()
    )?)?;
    circle_mod.add_function(wrap_pyfunction!(
        project_point_onto_circle_py,
        circle_mod.clone()
    )?)?;
    shape_mod.add_submodule(&circle_mod)?;

    let polygon_mod = PyModule::new(py, "polygon")?;
    polygon_mod.setattr("__doc__", MODULE_DOC_POLYGON)?;
    polygon_mod.add_function(wrap_pyfunction!(
        clean_polygon_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        is_almost_equal_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        normalize_polygons_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        translate_bounds_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        translate_polygons_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        point_line_distance_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygon_area_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygon_signed_area_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygon_perimeter_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygon_bounds_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygon_group_bounds_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygon_centroid_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        is_polygon_convex_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygon_convex_hull_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygon_edges_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        is_point_inside_polygon_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        offset_polygon_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygons_union_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygons_intersection_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygons_difference_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygons_group_intersection_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        get_polygons_group_difference_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        polygons_intersect_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        flip_polygon_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        flip_polygons_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        rotate_polygon_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        rotate_polygons_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        scale_polygon_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        translate_polygon_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        polygon_area_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        polygon_bounds_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        polygon_perimeter_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        polygon_group_bounds_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        flip_polygon_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        flip_polygons_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        normalize_polygons_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        point_in_polygon_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        polygons_intersect_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        rotate_polygon_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        rotate_polygons_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        translate_polygon_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        translate_polygons_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        to_clipper_numpy_py,
        polygon_mod.clone()
    )?)?;
    polygon_mod.add_function(wrap_pyfunction!(
        is_polygon_clockwise_py,
        polygon_mod.clone()
    )?)?;
    shape_mod.add_submodule(&polygon_mod)?;

    let line_mod = PyModule::new(py, "line")?;
    line_mod.setattr("__doc__", MODULE_DOC_LINE)?;
    line_mod.add_function(wrap_pyfunction!(
        get_line_line_intersection_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        get_line_segment_intersection_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        get_line_closest_point_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        get_line_segment_closest_point_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        get_point_line_distance_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        is_point_on_line_segment_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        does_line_segment_intersect_rect_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        does_line_segment_intersect_circle_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        get_line_segment_polygon_intersections_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        get_angle_at_vertex_py,
        line_mod.clone()
    )?)?;
    line_mod.add_function(wrap_pyfunction!(
        get_line_segment_length_py,
        line_mod.clone()
    )?)?;
    shape_mod.add_submodule(&line_mod)?;

    let rect_mod = PyModule::new(py, "rect")?;
    rect_mod.setattr("__doc__", MODULE_DOC_RECT)?;
    rect_mod.add_function(wrap_pyfunction!(
        is_point_inside_rect_py,
        rect_mod.clone()
    )?)?;
    rect_mod.add_function(wrap_pyfunction!(
        does_rect_contain_rect_py,
        rect_mod.clone()
    )?)?;
    rect_mod.add_function(wrap_pyfunction!(
        does_rect_intersect_rect_py,
        rect_mod.clone()
    )?)?;
    rect_mod.add_function(wrap_pyfunction!(
        do_rects_intersect_py,
        rect_mod.clone()
    )?)?;
    shape_mod.add_submodule(&rect_mod)?;

    let point_mod = PyModule::new(py, "point")?;
    point_mod.setattr("__doc__", MODULE_DOC_POINT)?;
    point_mod
        .add_function(wrap_pyfunction!(midpoint_py, point_mod.clone())?)?;
    point_mod.add_function(wrap_pyfunction!(
        are_points_equal_py,
        point_mod.clone()
    )?)?;
    point_mod.add_function(wrap_pyfunction!(
        transform_point_py,
        point_mod.clone()
    )?)?;
    shape_mod.add_submodule(&point_mod)?;

    m.add_submodule(&shape_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.shape", &shape_mod)?;
    sys_modules.set_item("raygeo.geo.shape.arc", &arc_mod)?;
    sys_modules.set_item("raygeo.geo.shape.bezier", &bezier_mod)?;
    sys_modules.set_item("raygeo.geo.shape.circle", &circle_mod)?;
    sys_modules.set_item("raygeo.geo.shape.polygon", &polygon_mod)?;
    sys_modules.set_item("raygeo.geo.shape.line", &line_mod)?;
    sys_modules.set_item("raygeo.geo.shape.rect", &rect_mod)?;
    sys_modules.set_item("raygeo.geo.shape.point", &point_mod)?;
    sys_modules.set_item("raygeo.shape", &shape_mod)?;
    sys_modules.set_item("raygeo.shape.arc", &arc_mod)?;
    sys_modules.set_item("raygeo.shape.bezier", &bezier_mod)?;
    sys_modules.set_item("raygeo.shape.circle", &circle_mod)?;
    sys_modules.set_item("raygeo.shape.polygon", &polygon_mod)?;
    sys_modules.set_item("raygeo.shape.line", &line_mod)?;
    sys_modules.set_item("raygeo.shape.rect", &rect_mod)?;
    sys_modules.set_item("raygeo.shape.point", &point_mod)?;

    Ok(())
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
    get_arc_bounds(start, end, center, clockwise)
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
    start_pos: (f64, f64, f64),
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
    does_arc_intersect_rect(arc_start, arc_end, arc_center, clockwise, rect)
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
    let points_2d: Vec<(f64, f64)> =
        points.iter().map(|p| (p.0, p.1)).collect();
    is_arc_clockwise(&points_2d, (center.0, center.1))
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
    start_point: (f64, f64, f64),
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

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_point_at(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        t: float,
    ) -> types.Point:
        """Get a point on a cubic bezier at parameter t.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param t: Parameter value (0..1).
        :returns: Point on the bezier curve (x, y).
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_point_at")]
fn get_bezier_point_at_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    t: f64,
) -> Point {
    get_bezier_point_at(
        (p0.0, p0.1),
        (p1.0, p1.1),
        (p2.0, p2.1),
        (p3.0, p3.1),
        t,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def split_bezier(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        t: float,
    ) -> tuple[
        tuple[types.Point, types.Point, types.Point, types.Point],
        tuple[types.Point, types.Point, types.Point, types.Point],
    ]:
        """Split a cubic bezier at parameter t.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param t: Split parameter (0..1).
        :returns: Two bezier curves (left, right).
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "split_bezier")]
fn split_bezier_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    t: f64,
) -> BezierSplit {
    split_bezier((p0.0, p0.1), (p1.0, p1.1), (p2.0, p2.1), (p3.0, p3.1), t)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_bounds(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
    ) -> types.Rect:
        """Get the bounding rectangle of a cubic bezier.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_bounds")]
fn get_bezier_bounds_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
) -> (f64, f64, f64, f64) {
    get_bezier_bounds((p0.0, p0.1), (p1.0, p1.1), (p2.0, p2.1), (p3.0, p3.1))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_rect_intersections(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        rect: types.Rect,
    ) -> list[float]:
        """Get intersection t-values of a bezier with a rectangle.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param rect: Rectangle (x_min, y_min, x_max, y_max).
        :returns: List of t-values where the bezier intersects.
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_rect_intersections")]
fn get_bezier_rect_intersections_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    rect: (f64, f64, f64, f64),
) -> Vec<f64> {
    get_bezier_rect_intersections(
        (p0.0, p0.1),
        (p1.0, p1.1),
        (p2.0, p2.1),
        (p3.0, p3.1),
        rect,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def clip_bezier_with_rect(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        rect: types.Rect,
    ) -> list[tuple[types.Point, types.Point, types.Point, types.Point]]:
        """Clip a cubic bezier with a rectangle.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param rect: Clipping rectangle (x_min, y_min, x_max, y_max).
        :returns: List of bezier segments inside the rectangle.
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "clip_bezier_with_rect")]
fn clip_bezier_with_rect_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    rect: (f64, f64, f64, f64),
) -> Vec<(Point, Point, Point, Point)> {
    clip_bezier_with_rect(
        (p0.0, p0.1),
        (p1.0, p1.1),
        (p2.0, p2.1),
        (p3.0, p3.1),
        rect,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def convert_cubic_bezier_to_quadratic(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
    ) -> tuple[types.Point, types.Point, types.Point]:
        """Convert a cubic bezier to a quadratic bezier.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :returns: Quadratic bezier (p0, p1, p2).
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "convert_cubic_bezier_to_quadratic")]
fn convert_cubic_bezier_to_quadratic_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
) -> (Point, Point, Point) {
    convert_cubic_bezier_to_quadratic(
        (p0.0, p0.1),
        (p1.0, p1.1),
        (p2.0, p2.1),
        (p3.0, p3.1),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def is_bezier_inside_polygons(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        polygons: typing.Any,
    ) -> bool:
        """Check if a bezier curve is inside a set of polygons.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param polygons: List of polygons to check against.
        :returns: True if the bezier is inside all polygons.
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "is_bezier_inside_polygons")]
fn is_bezier_inside_polygons_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    polygons: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    let polygons_2d = extract_polygons(polygons)?;
    Ok(is_bezier_inside_polygons(
        (p0.0, p0.1),
        (p1.0, p1.1),
        (p2.0, p2.1),
        (p3.0, p3.1),
        &polygons_2d,
    ))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def linearize_bezier(
        p0: types.Point3D,
        p1: types.Point3D,
        p2: types.Point3D,
        p3: types.Point3D,
        num_steps: int,
    ) -> list[tuple[types.Point3D, types.Point3D]]:
        """Linearize a bezier into line segments.

        :param p0: Start control point (x, y, z).
        :param p1: First control point (x, y, z).
        :param p2: Second control point (x, y, z).
        :param p3: End control point (x, y, z).
        :param num_steps: Number of linearization steps.
        :returns: List of (p1, p2) segment pairs.
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "linearize_bezier")]
#[allow(clippy::type_complexity)]
fn linearize_bezier_py(
    p0: PyPoint3D,
    p1: PyPoint3D,
    p2: PyPoint3D,
    p3: PyPoint3D,
    num_steps: usize,
) -> Vec<((f64, f64, f64), (f64, f64, f64))> {
    linearize_bezier(
        (p0.0, p0.1, p0.2),
        (p1.0, p1.1, p1.2),
        (p2.0, p2.1, p2.2),
        (p3.0, p3.1, p3.2),
        num_steps,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def linearize_bezier_adaptive(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
        p3: types.Point,
        tolerance_sq: float,
        max_subdivisions: int = 20,
    ) -> types.Polygon:
        """Adaptively linearize a bezier curve.

        :param p0: Start control point (x, y).
        :param p1: First control point (x, y).
        :param p2: Second control point (x, y).
        :param p3: End control point (x, y).
        :param tolerance_sq: Squared tolerance for subdivision.
        :param max_subdivisions: Maximum recursion depth.
        :returns: List of linearized points (x, y).
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "linearize_bezier_adaptive")]
#[pyo3(signature = (p0, p1, p2, p3, tolerance_sq, max_subdivisions=20))]
fn linearize_bezier_adaptive_py(
    p0: PyPoint2D,
    p1: PyPoint2D,
    p2: PyPoint2D,
    p3: PyPoint2D,
    tolerance_sq: f64,
    max_subdivisions: usize,
) -> Vec<(f64, f64)> {
    linearize_bezier_adaptive(
        (p0.0, p0.1),
        (p1.0, p1.1),
        (p2.0, p2.1),
        (p3.0, p3.1),
        tolerance_sq,
        max_subdivisions,
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def linearize_bezier_segment(
        p0: types.Point3D,
        p1: types.Point3D,
        p2: types.Point3D,
        p3: types.Point3D,
        tolerance: float = 0.1,
    ) -> list[types.Point3D]:
        """Linearize a single bezier segment.

        :param p0: Start control point (x, y, z).
        :param p1: First control point (x, y, z).
        :param p2: Second control point (x, y, z).
        :param p3: End control point (x, y, z).
        :param tolerance: Linearization tolerance.
        :returns: List of linearized points (x, y, z).
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "linearize_bezier_segment")]
#[pyo3(signature = (p0, p1, p2, p3, tolerance=0.1))]
fn linearize_bezier_segment_py(
    p0: PyPoint3D,
    p1: PyPoint3D,
    p2: PyPoint3D,
    p3: PyPoint3D,
    tolerance: f64,
) -> Vec<(f64, f64, f64)> {
    linearize_bezier_segment(
        (p0.0, p0.1, p0.2),
        (p1.0, p1.1, p1.2),
        (p2.0, p2.1, p2.2),
        (p3.0, p3.1, p3.2),
        Some(tolerance),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def flatten_bezier(
        p0: types.Point3D,
        p1: types.Point3D,
        p2: types.Point3D,
        p3: types.Point3D,
        tolerance: float,
        max_subdivisions: int,
        pts: list,
    ) -> None:
        """Flatten a bezier curve into points.

        :param p0: Start control point (x, y, z).
        :param p1: First control point (x, y, z).
        :param p2: Second control point (x, y, z).
        :param p3: End control point (x, y, z).
        :param tolerance: Flattening tolerance.
        :param max_subdivisions: Maximum recursion depth.
        :param pts: Output list to append points to.
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "flatten_bezier")]
fn flatten_bezier_py(
    p0: PyPoint3D,
    p1: PyPoint3D,
    p2: PyPoint3D,
    p3: PyPoint3D,
    tolerance: f64,
    max_subdivisions: usize,
    pts: &Bound<'_, PyList>,
) -> PyResult<()> {
    let mut result = Vec::new();
    flatten_bezier(
        (p0.0, p0.1, p0.2),
        (p1.0, p1.1, p1.2),
        (p2.0, p2.1, p2.2),
        (p3.0, p3.1, p3.2),
        tolerance,
        max_subdivisions,
        &mut result,
    );
    let py = pts.py();
    for p in result {
        let obj = (p.0, p.1, p.2).into_pyobject(py)?;
        pts.append(&obj)?;
    }
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_length(
        p0: types.Point,
        c1: types.Point,
        c2: types.Point,
        p1: types.Point,
    ) -> float:
        """Compute the arc length of a cubic Bezier curve.

        :param p0: Start point (x, y).
        :param c1: First control point (x, y).
        :param c2: Second control point (x, y).
        :param p1: End point (x, y).
        :returns: Arc length.
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_length")]
fn get_bezier_length_py(p0: Point, c1: Point, c2: Point, p1: Point) -> f64 {
    get_bezier_length(p0, c1, c2, p1)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_bezier_flatness_sq(
        a: types.Point3D,
        b: types.Point3D,
        c: types.Point3D,
        d: types.Point3D,
    ) -> float:
        """Compute the flatness squared of a cubic bezier.

        :param a: Start point (x, y, z).
        :param b: First control point (x, y, z).
        :param c: Second control point (x, y, z).
        :param d: End point (x, y, z).
        :returns: Flatness squared value.
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_bezier_flatness_sq")]
fn get_bezier_flatness_sq_py(
    a: PyPoint3D,
    b: PyPoint3D,
    c: PyPoint3D,
    d: PyPoint3D,
) -> f64 {
    get_bezier_flatness_sq(
        (a.0, a.1, a.2),
        (b.0, b.1, b.2),
        (c.0, c.1, c.2),
        (d.0, d.1, d.2),
    )
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_perpendicular_dist_sq(
        pt: types.Point3D,
        origin: types.Point3D,
        vx: float,
        vy: float,
        vz: float = 0.0,
        norm_sq: float = 0.0,
    ) -> float:
        """Compute the perpendicular distance squared.

        :param pt: Point to measure from.
        :param origin: Origin of the line.
        :param vx: X component of line direction.
        :param vy: Y component of line direction.
        :param vz: Z component of line direction.
        :param norm_sq: Precomputed squared norm (optional).
        :returns: Perpendicular distance squared.
        """
"#,
    module = "raygeo.geo.shape.bezier"
)]
#[pyfunction(name = "get_perpendicular_dist_sq")]
#[pyo3(signature = (pt, origin, vx, vy, vz=0.0, norm_sq=0.0))]
fn get_perpendicular_dist_sq_py(
    pt: PyPoint3D,
    origin: PyPoint3D,
    vx: f64,
    vy: f64,
    vz: f64,
    norm_sq: f64,
) -> f64 {
    get_perpendicular_dist_sq(
        (pt.0, pt.1, pt.2),
        (origin.0, origin.1, origin.2),
        vx,
        vy,
        vz,
        norm_sq,
    )
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

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def clean_polygon(
        polygon: collections.abc.Sequence[types.Point],
        tolerance: typing.Optional[float] = None,
    ) -> typing.Optional[types.Polygon]:
        """Clean a polygon by removing near-duplicate points.

        :param polygon: Input polygon as (x, y) points.
        :param tolerance: Distance tolerance for deduplication.
        :returns: Cleaned polygon or None.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "clean_polygon")]
#[pyo3(signature = (polygon, tolerance=None))]
fn clean_polygon_py(
    polygon: Vec<PyPoint2D>,
    tolerance: Option<f64>,
) -> Option<Vec<Point>> {
    clean_polygon(&poly_to_points(polygon), tolerance.unwrap_or(1e-6))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def is_almost_equal(
        a: float,
        b: float,
        tolerance: typing.Optional[float] = None,
    ) -> bool:
        """Check if two floats are almost equal.

        :param a: First float.
        :param b: Second float.
        :param tolerance: Comparison tolerance.
        :returns: True if |a - b| < tolerance.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "is_almost_equal")]
#[pyo3(signature = (a, b, tolerance=None))]
fn is_almost_equal_py(a: f64, b: f64, tolerance: Option<f64>) -> bool {
    is_almost_equal(a, b, tolerance.unwrap_or(1e-9))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def normalize_polygons(polygons: typing.Any) -> tuple[list[types.Polygon], float, float]:
        """Normalize polygons (outer CCW, inner CW).

        :param polygons: List of polygons to normalize.
        :returns: Tuple of (normalized_polygons, min_x, min_y).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "normalize_polygons")]
fn normalize_polygons_py(
    polygons: &Bound<'_, PyAny>,
) -> PyResult<(Vec<Vec<Point>>, f64, f64)> {
    let p = extract_polygons(polygons)?;
    Ok(normalize_polygons(&p))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def translate_bounds(
        bounds: types.Rect,
        dx: float,
        dy: float,
    ) -> types.Rect:
        """Translate a bounding rectangle.

        :param bounds: Bounding rectangle (x_min, y_min, x_max, y_max).
        :param dx: X translation.
        :param dy: Y translation.
        :returns: Translated bounding rectangle.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_bounds")]
fn translate_bounds_py(
    bounds: (f64, f64, f64, f64),
    dx: f64,
    dy: f64,
) -> (f64, f64, f64, f64) {
    translate_bounds(bounds, dx, dy)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def translate_polygons(polygons: typing.Any, dx: float, dy: float) -> list[types.Polygon]:
        """Translate a list of polygons.

        :param polygons: List of polygons to translate.
        :param dx: X translation.
        :param dy: Y translation.
        :returns: Translated polygons.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_polygons")]
fn translate_polygons_py(
    polygons: &Bound<'_, PyAny>,
    dx: f64,
    dy: f64,
) -> PyResult<Vec<Vec<Point>>> {
    let p = extract_polygons(polygons)?;
    Ok(translate_polygons(&p, dx, dy))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def point_line_distance(
        point: types.Point,
        line_start: types.Point,
        line_end: types.Point,
    ) -> float:
        """Compute the distance from a point to a line.

        :param point: Point (x, y).
        :param line_start: Line start point (x, y).
        :param line_end: Line end point (x, y).
        :returns: Perpendicular distance.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "point_line_distance")]
fn point_line_distance_py(
    point: Point,
    line_start: Point,
    line_end: Point,
) -> f64 {
    point_line_distance(point, line_start, line_end)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_area(polygon: collections.abc.Sequence[types.Point]) -> float:
        """Get the unsigned area of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Unsigned area.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_area")]
fn get_polygon_area_py(polygon: Vec<PyPoint2D>) -> f64 {
    get_polygon_signed_area(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_signed_area(
        polygon: collections.abc.Sequence[types.Point],
    ) -> float:
        """Get the signed area of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Signed area (positive for CCW, negative for CW).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_signed_area")]
fn get_polygon_signed_area_py(polygon: Vec<PyPoint2D>) -> f64 {
    get_polygon_signed_area(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_perimeter(
        polygon: collections.abc.Sequence[types.Point],
    ) -> float:
        """Get the perimeter of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Perimeter length.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_perimeter")]
fn get_polygon_perimeter_py(polygon: Vec<PyPoint2D>) -> f64 {
    get_polygon_perimeter(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_bounds(
        polygon: collections.abc.Sequence[types.Point],
    ) -> types.Rect:
        """Get the bounding rectangle of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_bounds")]
fn get_polygon_bounds_py(polygon: Vec<PyPoint2D>) -> (f64, f64, f64, f64) {
    get_polygon_bounds(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygon_group_bounds(
        polygons: typing.Any,
    ) -> types.Rect:
        """Get the bounding rectangle of a group of polygons.

        :param polygons: List of polygons.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_group_bounds")]
fn get_polygon_group_bounds_py(
    polygons: &Bound<'_, PyAny>,
) -> PyResult<(f64, f64, f64, f64)> {
    let p = extract_polygons(polygons)?;
    Ok(get_polygon_group_bounds(&p))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_centroid(
        polygon: collections.abc.Sequence[types.Point],
    ) -> types.Point:
        """Get the centroid of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Centroid point (x, y).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_centroid")]
fn get_polygon_centroid_py(polygon: Vec<PyPoint2D>) -> Point {
    get_polygon_centroid(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def is_polygon_convex(
        polygon: collections.abc.Sequence[types.Point],
    ) -> bool:
        """Check if a polygon is convex.

        :param polygon: Polygon as (x, y) points.
        :returns: True if the polygon is convex.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "is_polygon_convex")]
fn is_polygon_convex_py(polygon: Vec<PyPoint2D>) -> bool {
    is_polygon_convex(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_convex_hull(
        polygon: collections.abc.Sequence[types.Point],
    ) -> types.Polygon:
        """Get the convex hull of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: Convex hull as list of points.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_convex_hull")]
fn get_polygon_convex_hull_py(polygon: Vec<PyPoint2D>) -> Vec<Point> {
    get_polygon_convex_hull(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygon_edges(
        polygon: collections.abc.Sequence[types.Point],
    ) -> list[tuple[types.Point, types.Point]]:
        """Get the edges of a polygon.

        :param polygon: Polygon as (x, y) points.
        :returns: List of ((x1, y1), (x2, y2)) edges.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygon_edges")]
fn get_polygon_edges_py(polygon: Vec<PyPoint2D>) -> Vec<(Point, Point)> {
    get_polygon_edges(&poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def is_point_inside_polygon(
        point: types.Point,
        polygon: collections.abc.Sequence[types.Point],
    ) -> bool:
        """Check if a point is inside a polygon.

        :param point: Point (x, y) to test.
        :param polygon: Polygon as (x, y) points.
        :returns: True if point is inside the polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "is_point_inside_polygon")]
fn is_point_inside_polygon_py(point: Point, polygon: Vec<PyPoint2D>) -> bool {
    is_point_inside_polygon(point, &poly_to_points(polygon))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def offset_polygon(
        polygon: collections.abc.Sequence[types.Point],
        offset: float,
    ) -> list[types.Polygon]:
        """Offset (inflate/deflate) a polygon.

        :param polygon: Polygon as (x, y) points.
        :param offset: Offset distance (positive to inflate, negative to deflate).
        :returns: Offset polygon(s).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "offset_polygon")]
fn offset_polygon_py(polygon: Vec<PyPoint2D>, offset: f64) -> Vec<Vec<Point>> {
    offset_polygon(&poly_to_points(polygon), offset)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygons_union(polygons: typing.Any) -> list[types.Polygon]:
        """Get the union of multiple polygons.

        :param polygons: List of polygons to union.
        :returns: Union polygon(s).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_union")]
fn get_polygons_union_py(
    polygons: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<Point>>> {
    let p = extract_polygons(polygons)?;
    Ok(get_polygons_union(&p))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_intersection(
        poly1: collections.abc.Sequence[types.Point],
        poly2: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Get the intersection of two polygons.

        :param poly1: First polygon as (x, y) points.
        :param poly2: Second polygon as (x, y) points.
        :returns: Intersection polygon(s).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_intersection")]
fn get_polygons_intersection_py(
    poly1: Vec<PyPoint2D>,
    poly2: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    get_polygons_intersection(&poly_to_points(poly1), &poly_to_points(poly2))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def get_polygons_difference(
        poly1: collections.abc.Sequence[types.Point],
        poly2: collections.abc.Sequence[types.Point],
    ) -> list[types.Polygon]:
        """Get the difference of two polygons.

        :param poly1: First polygon as (x, y) points.
        :param poly2: Second polygon to subtract.
        :returns: Difference polygon(s).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_difference")]
fn get_polygons_difference_py(
    poly1: Vec<PyPoint2D>,
    poly2: Vec<PyPoint2D>,
) -> Vec<Vec<Point>> {
    get_polygons_difference(&poly_to_points(poly1), &poly_to_points(poly2))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygons_group_intersection(
        subject: typing.Sequence[types.Polygon],
        clip: typing.Sequence[types.Polygon],
    ) -> list[types.Polygon]:
        """Intersect two groups of polygons (subject & clip).

        :param subject: Subject polygons.
        :param clip: Clip polygons.
        :returns: Intersection polygon(s).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_group_intersection")]
fn get_polygons_group_intersection_py(
    subject: &Bound<'_, PyAny>,
    clip: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<Point>>> {
    let subject_polys = extract_polygons(subject)?;
    let clip_polys = extract_polygons(clip)?;
    Ok(get_polygons_group_intersection(&subject_polys, &clip_polys))
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def get_polygons_group_difference(
        subject: typing.Sequence[types.Polygon],
        clip: typing.Sequence[types.Polygon],
    ) -> list[types.Polygon]:
        """Subtract clip polygons from subject polygons.

        :param subject: Subject polygons.
        :param clip: Clip polygons to subtract.
        :returns: Difference polygon(s).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "get_polygons_group_difference")]
fn get_polygons_group_difference_py(
    subject: &Bound<'_, PyAny>,
    clip: &Bound<'_, PyAny>,
) -> PyResult<Vec<Vec<Point>>> {
    let subject_polys = extract_polygons(subject)?;
    let clip_polys = extract_polygons(clip)?;
    Ok(get_polygons_group_difference(&subject_polys, &clip_polys))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def polygons_intersect(
        p1: collections.abc.Sequence[types.Point],
        p2: collections.abc.Sequence[types.Point],
        min_area: float = 0.0,
    ) -> bool:
        """Check if two polygons intersect.

        :param p1: First polygon as (x, y) points.
        :param p2: Second polygon as (x, y) points.
        :param min_area: Minimum intersection area threshold.
        :returns: True if polygons intersect.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygons_intersect")]
#[pyo3(signature = (p1, p2, min_area=0.0))]
fn polygons_intersect_py(
    p1: Vec<PyPoint2D>,
    p2: Vec<PyPoint2D>,
    min_area: f64,
) -> bool {
    polygons_intersect(&poly_to_points(p1), &poly_to_points(p2), min_area)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def flip_polygon(
        polygon: collections.abc.Sequence[types.Point],
        flip_h: bool,
        flip_v: bool,
    ) -> types.Polygon:
        """Flip a polygon horizontally and/or vertically.

        :param polygon: Polygon as (x, y) points.
        :param flip_h: Whether to flip horizontally.
        :param flip_v: Whether to flip vertically.
        :returns: Flipped polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "flip_polygon")]
fn flip_polygon_py(
    polygon: Vec<PyPoint2D>,
    flip_h: bool,
    flip_v: bool,
) -> Vec<Point> {
    flip_polygon(&poly_to_points(polygon), flip_h, flip_v)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def flip_polygons(
        polygons: typing.Any,
        flip_h: bool,
        flip_v: bool,
    ) -> list[types.Polygon]:
        """Flip multiple polygons.

        :param polygons: List of polygons to flip.
        :param flip_h: Whether to flip horizontally.
        :param flip_v: Whether to flip vertically.
        :returns: Flipped polygons.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "flip_polygons")]
fn flip_polygons_py(
    polygons: &Bound<'_, PyAny>,
    flip_h: bool,
    flip_v: bool,
) -> PyResult<Vec<Vec<Point>>> {
    let p = extract_polygons(polygons)?;
    Ok(flip_polygons(&p, flip_h, flip_v))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def rotate_polygon(
        polygon: collections.abc.Sequence[types.Point],
        angle: float,
    ) -> types.Polygon:
        """Rotate a polygon by an angle.

        :param polygon: Polygon as (x, y) points.
        :param angle: Rotation angle in degrees.
        :returns: Rotated polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "rotate_polygon")]
fn rotate_polygon_py(polygon: Vec<PyPoint2D>, angle: f64) -> Vec<Point> {
    rotate_polygon(&poly_to_points(polygon), angle)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import raygeo.geo.types

    def rotate_polygons(polygons: typing.Any, angle: float) -> list[types.Polygon]:
        """Rotate multiple polygons by an angle.

        :param polygons: List of polygons to rotate.
        :param angle: Rotation angle in degrees.
        :returns: Rotated polygons.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "rotate_polygons")]
fn rotate_polygons_py(
    polygons: &Bound<'_, PyAny>,
    angle: f64,
) -> PyResult<Vec<Vec<Point>>> {
    let p = extract_polygons(polygons)?;
    Ok(rotate_polygons(&p, angle))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import raygeo.geo.types

    def scale_polygon(
        polygon: collections.abc.Sequence[types.Point],
        scale: float,
        scale_y: typing.Optional[float] = None,
    ) -> types.Polygon:
        """Scale a polygon.

        :param polygon: Polygon as (x, y) points.
        :param scale: X (and Y if scale_y is None) scale factor.
        :param scale_y: Y scale factor (optional).
        :returns: Scaled polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "scale_polygon")]
#[pyo3(signature = (polygon, scale, scale_y=None))]
fn scale_polygon_py(
    polygon: Vec<PyPoint2D>,
    scale: f64,
    scale_y: Option<f64>,
) -> Vec<Point> {
    scale_polygon(&poly_to_points(polygon), scale, scale_y)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def translate_polygon(
        polygon: collections.abc.Sequence[types.Point],
        dx: float,
        dy: float,
    ) -> types.Polygon:
        """Translate a polygon.

        :param polygon: Polygon as (x, y) points.
        :param dx: X translation.
        :param dy: Y translation.
        :returns: Translated polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_polygon")]
fn translate_polygon_py(
    polygon: Vec<PyPoint2D>,
    dx: f64,
    dy: f64,
) -> Vec<Point> {
    translate_polygon(&poly_to_points(polygon), dx, dy)
}

// -- numpy wrapper helpers --

fn _polygon_from_numpy(arr: &Bound<'_, PyArray2<f64>>) -> Vec<(f64, f64)> {
    let readonly = arr.readonly();
    let view = readonly.as_array();
    view.rows()
        .into_iter()
        .map(|row| (row[0], row[1]))
        .collect()
}

fn _polygon_to_numpy(py: Python<'_>, poly: Vec<(f64, f64)>) -> Py<PyAny> {
    let vecs: Vec<Vec<f64>> =
        poly.into_iter().map(|(x, y)| vec![x, y]).collect();
    let np_arr = PyArray2::<f64>::from_vec2(py, &vecs)
        .expect("failed to create numpy array");
    np_arr.into_any().unbind()
}

fn _polygons_from_numpy_list(
    polys: Vec<Bound<'_, PyArray2<f64>>>,
) -> Vec<Vec<(f64, f64)>> {
    polys.into_iter().map(|a| _polygon_from_numpy(&a)).collect()
}

fn _polygons_to_numpy_list(
    py: Python<'_>,
    polys: Vec<Vec<(f64, f64)>>,
) -> Vec<Py<PyAny>> {
    polys
        .into_iter()
        .map(|p| _polygon_to_numpy(py, p))
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing

    def polygon_area_numpy(polygon: numpy.typing.NDArray) -> float:
        """Get the area of a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :returns: Signed area.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygon_area_numpy")]
fn polygon_area_numpy_py(polygon: Bound<'_, PyArray2<f64>>) -> f64 {
    let p = _polygon_from_numpy(&polygon);
    get_polygon_signed_area(&p)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing
    import raygeo.geo.types

    def polygon_bounds_numpy(
        polygon: numpy.typing.NDArray,
    ) -> types.Rect:
        """Get bounds of a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygon_bounds_numpy")]
fn polygon_bounds_numpy_py(
    polygon: Bound<'_, PyArray2<f64>>,
) -> (f64, f64, f64, f64) {
    let p = _polygon_from_numpy(&polygon);
    get_polygon_bounds(&p)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing

    def polygon_perimeter_numpy(polygon: numpy.typing.NDArray) -> float:
        """Get the perimeter of a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :returns: Perimeter length.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygon_perimeter_numpy")]
fn polygon_perimeter_numpy_py(polygon: Bound<'_, PyArray2<f64>>) -> f64 {
    let p = _polygon_from_numpy(&polygon);
    get_polygon_perimeter(&p)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import numpy.typing
    import raygeo.geo.types

    def polygon_group_bounds_numpy(
        polygons: collections.abc.Sequence[numpy.typing.NDArray],
    ) -> types.Rect:
        """Get bounds of polygon group from numpy arrays.

        :param polygons: Sequence of 2D numpy arrays.
        :returns: Bounding rectangle as (x_min, y_min, x_max, y_max).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygon_group_bounds_numpy")]
fn polygon_group_bounds_numpy_py(
    polygons: Vec<Bound<'_, PyArray2<f64>>>,
) -> (f64, f64, f64, f64) {
    let p = _polygons_from_numpy_list(polygons);
    get_polygon_group_bounds(&p)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import numpy.typing

    def flip_polygon_numpy(
        polygon: numpy.typing.NDArray,
        flip_h: bool,
        flip_v: bool,
    ) -> typing.Any:
        """Flip a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :param flip_h: Whether to flip horizontally.
        :param flip_v: Whether to flip vertically.
        :returns: Flipped polygon as numpy array.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "flip_polygon_numpy")]
fn flip_polygon_numpy_py(
    py: Python<'_>,
    polygon: Bound<'_, PyArray2<f64>>,
    flip_h: bool,
    flip_v: bool,
) -> Py<PyAny> {
    let p = _polygon_from_numpy(&polygon);
    let result = flip_polygon(&p, flip_h, flip_v);
    _polygon_to_numpy(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def flip_polygons_numpy(polygons: list, flip_h: bool, flip_v: bool) -> typing.Any:
        """Flip polygons from numpy arrays.

        :param polygons: List of 2D numpy arrays.
        :param flip_h: Whether to flip horizontally.
        :param flip_v: Whether to flip vertically.
        :returns: List of flipped numpy arrays.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "flip_polygons_numpy")]
fn flip_polygons_numpy_py<'py>(
    py: Python<'py>,
    polygons: Bound<'py, PyList>,
    flip_h: bool,
    flip_v: bool,
) -> PyResult<Bound<'py, PyAny>> {
    if !flip_h && !flip_v {
        return Ok(polygons.as_any().clone());
    }
    let mut p = Vec::new();
    for item in polygons.iter() {
        let arr = item.cast::<PyArray2<f64>>()?;
        p.push(_polygon_from_numpy(arr));
    }
    let result = flip_polygons(&p, flip_h, flip_v);
    let np_list = _polygons_to_numpy_list(py, result);
    Ok(PyList::new(py, np_list)?.as_any().clone())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import numpy.typing

    def normalize_polygons_numpy(
        polygons: collections.abc.Sequence[numpy.typing.NDArray],
    ) -> tuple[list[numpy.typing.NDArray], float, float]:
        """Normalize polygons from numpy arrays.

        :param polygons: Sequence of 2D numpy arrays.
        :returns: Tuple of (normalized_arrays, min_x, min_y).
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "normalize_polygons_numpy")]
fn normalize_polygons_numpy_py(
    py: Python<'_>,
    polygons: Vec<Bound<'_, PyArray2<f64>>>,
) -> (Vec<Py<PyAny>>, f64, f64) {
    let p = _polygons_from_numpy_list(polygons);
    let (result, min_x, min_y) = normalize_polygons(&p);
    let result_np = _polygons_to_numpy_list(py, result);
    (result_np, min_x, min_y)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing
    import raygeo.geo.types

    def point_in_polygon_numpy(
        point: types.Point,
        polygon: numpy.typing.NDArray,
    ) -> bool:
        """Check if point is in polygon from numpy array.

        :param point: Point (x, y) to test.
        :param polygon: Polygon as a 2D numpy array.
        :returns: True if point is inside the polygon.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "point_in_polygon_numpy")]
fn point_in_polygon_numpy_py(
    point: (f64, f64),
    polygon: Bound<'_, PyArray2<f64>>,
) -> bool {
    let p = _polygon_from_numpy(&polygon);
    is_point_inside_polygon(point, &p)
}

#[gen_stub_pyfunction(
    python = r#"
    import numpy.typing

    def polygons_intersect_numpy(
        poly1: numpy.typing.NDArray,
        poly2: numpy.typing.NDArray,
        min_area: float = 0.0,
    ) -> bool:
        """Check if polygons intersect from numpy arrays.

        :param poly1: First polygon as a 2D numpy array.
        :param poly2: Second polygon as a 2D numpy array.
        :param min_area: Minimum intersection area threshold.
        :returns: True if polygons intersect.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "polygons_intersect_numpy")]
#[pyo3(signature = (poly1, poly2, min_area=0.0))]
fn polygons_intersect_numpy_py(
    poly1: Bound<'_, PyArray2<f64>>,
    poly2: Bound<'_, PyArray2<f64>>,
    min_area: f64,
) -> bool {
    let p1 = _polygon_from_numpy(&poly1);
    let p2 = _polygon_from_numpy(&poly2);
    polygons_intersect(&p1, &p2, min_area)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import numpy.typing

    def rotate_polygon_numpy(
        polygon: numpy.typing.NDArray,
        angle: float,
    ) -> typing.Any:
        """Rotate a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :param angle: Rotation angle in degrees.
        :returns: Rotated polygon as numpy array.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "rotate_polygon_numpy")]
fn rotate_polygon_numpy_py(
    py: Python<'_>,
    polygon: Bound<'_, PyArray2<f64>>,
    angle: f64,
) -> Py<PyAny> {
    let p = _polygon_from_numpy(&polygon);
    let result = rotate_polygon(&p, angle);
    _polygon_to_numpy(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import numpy.typing

    def rotate_polygons_numpy(
        polygons: collections.abc.Sequence[numpy.typing.NDArray],
        angle: float,
    ) -> list[typing.Any]:
        """Rotate polygons from numpy arrays.

        :param polygons: Sequence of 2D numpy arrays.
        :param angle: Rotation angle in degrees.
        :returns: List of rotated numpy arrays.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "rotate_polygons_numpy")]
fn rotate_polygons_numpy_py(
    py: Python<'_>,
    polygons: Vec<Bound<'_, PyArray2<f64>>>,
    angle: f64,
) -> Vec<Py<PyAny>> {
    let p = _polygons_from_numpy_list(polygons);
    let result = rotate_polygons(&p, angle);
    _polygons_to_numpy_list(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
    import numpy.typing

    def translate_polygon_numpy(
        polygon: numpy.typing.NDArray,
        dx: float,
        dy: float,
    ) -> typing.Any:
        """Translate a polygon from numpy array.

        :param polygon: Polygon as a 2D numpy array.
        :param dx: X translation.
        :param dy: Y translation.
        :returns: Translated polygon as numpy array.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_polygon_numpy")]
fn translate_polygon_numpy_py(
    py: Python<'_>,
    polygon: Bound<'_, PyArray2<f64>>,
    dx: f64,
    dy: f64,
) -> Py<PyAny> {
    let p = _polygon_from_numpy(&polygon);
    let result = translate_polygon(&p, dx, dy);
    _polygon_to_numpy(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import typing
    import numpy.typing

    def translate_polygons_numpy(
        polygons: collections.abc.Sequence[numpy.typing.NDArray],
        dx: float,
        dy: float,
    ) -> list[typing.Any]:
        """Translate polygons from numpy arrays.

        :param polygons: Sequence of 2D numpy arrays.
        :param dx: X translation.
        :param dy: Y translation.
        :returns: List of translated numpy arrays.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "translate_polygons_numpy")]
fn translate_polygons_numpy_py(
    py: Python<'_>,
    polygons: Vec<Bound<'_, PyArray2<f64>>>,
    dx: f64,
    dy: f64,
) -> Vec<Py<PyAny>> {
    let p = _polygons_from_numpy_list(polygons);
    let result = translate_polygons(&p, dx, dy);
    _polygons_to_numpy_list(py, result)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing

    def to_clipper_numpy(polygon: typing.Any) -> list[tuple[int, int]]:
        """Convert a polygon to Clipper coordinates.

        :param polygon: Polygon as numpy array or list of tuples.
        :returns: Polygon with integer coordinates for Clipper.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "to_clipper_numpy")]
fn to_clipper_numpy_py(
    polygon: &Bound<'_, PyAny>,
) -> PyResult<Vec<(i64, i64)>> {
    let points: Vec<(f64, f64)> =
        if let Ok(arr) = polygon.extract::<Bound<'_, PyArray2<f64>>>() {
            let readonly = arr.readonly();
            (0..readonly.shape()[0])
                .map(|i| {
                    (
                        *readonly.get([i, 0]).unwrap(),
                        *readonly.get([i, 1]).unwrap(),
                    )
                })
                .collect()
        } else if let Ok(pts) = polygon.extract::<Vec<(f64, f64)>>() {
            pts
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
            "polygon must be an (N, 2) numpy array or list of (x, y) tuples",
        ));
        };
    Ok(to_clipper_from_points(&points, 10_000_000.0))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def is_polygon_clockwise(points: collections.abc.Sequence[types.Point2DOr3D]) -> bool:
        """Check if a polygon has clockwise winding order.

        :param points: Sequence of (x, y) points defining a polygon.
        :returns: True if the winding is clockwise.
        """
"#,
    module = "raygeo.geo.shape.polygon"
)]
#[pyfunction(name = "is_polygon_clockwise")]
fn is_polygon_clockwise_py(points: Vec<PyPoint2D>) -> bool {
    let pts: Vec<(f64, f64)> = points.iter().map(|p| (p.0, p.1)).collect();
    is_polygon_clockwise(&pts)
}

#[gen_stub_pyfunction(
    python = r#"
    import typing
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
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_line_intersection")]
fn get_line_line_intersection_py(
    p1: Point,
    p2: Point,
    p3: Point,
    p4: Point,
) -> Option<Point> {
    get_line_line_intersection(p1, p2, p3, p4)
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
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_segment_intersection")]
fn get_line_segment_intersection_py(
    p1: Point,
    p2: Point,
    p3: Point,
    p4: Point,
) -> Option<Point> {
    get_line_segment_intersection(p1, p2, p3, p4)
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
        """Get the closest point on a line to a given point.

        :param line_p1: First point on the line.
        :param line_p2: Second point on the line.
        :param x: X coordinate of target point.
        :param y: Y coordinate of target point.
        :returns: Closest point (x, y) on the line.
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_closest_point")]
fn get_line_closest_point_py(
    line_p1: Point,
    line_p2: Point,
    x: f64,
    y: f64,
) -> Point {
    get_line_closest_point(line_p1, line_p2, x, y)
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
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_segment_closest_point")]
fn get_line_segment_closest_point_py(
    seg_p1: Point,
    seg_p2: Point,
    x: f64,
    y: f64,
) -> (f64, Point, f64) {
    get_line_segment_closest_point(seg_p1, seg_p2, x, y)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_point_line_distance(
        point: types.Point,
        line_p1: types.Point,
        line_p2: types.Point,
    ) -> float:
        """Get the distance from a point to a line.

        :param point: Point (x, y).
        :param line_p1: First point on the line.
        :param line_p2: Second point on the line.
        :returns: Perpendicular distance.
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_point_line_distance")]
fn get_point_line_distance_py(
    point: Point,
    line_p1: Point,
    line_p2: Point,
) -> f64 {
    get_point_line_distance(point, line_p1, line_p2)
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
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "is_point_on_line_segment")]
fn is_point_on_line_segment_py(
    point: Point,
    seg_p1: Point,
    seg_p2: Point,
) -> bool {
    is_point_on_segment(point, seg_p1, seg_p2)
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
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "does_line_segment_intersect_rect")]
fn does_line_segment_intersect_rect_py(
    p1: Point,
    p2: Point,
    rect: (f64, f64, f64, f64),
) -> bool {
    does_line_segment_intersect_rect(p1, p2, rect)
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
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "does_line_segment_intersect_circle")]
fn does_line_segment_intersect_circle_py(
    p1: Point,
    p2: Point,
    circle_center: Point,
    circle_radius: f64,
) -> bool {
    does_line_segment_intersect_circle(p1, p2, circle_center, circle_radius)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_line_segment_polygon_intersections(
        p1: types.Point,
        p2: types.Point,
        polygon: list[types.Polygon],
    ) -> list[float]:
        """Get t-values of line segment-polygon intersections.

        :param p1: Start of the line segment.
        :param p2: End of the line segment.
        :param polygon: Polygon(s) to test against.
        :returns: List of t-values where the segment intersects.
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_segment_polygon_intersections")]
fn get_line_segment_polygon_intersections_py(
    p1: Point,
    p2: Point,
    polygon: Vec<Vec<Point>>,
) -> Vec<f64> {
    get_line_segment_polygon_intersections(p1, p2, &polygon)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_angle_at_vertex(
        p0: types.Point,
        p1: types.Point,
        p2: types.Point,
    ) -> float:
        """Get the angle at a vertex between three points.

        :param p0: First point.
        :param p1: Vertex point.
        :param p2: Third point.
        :returns: Angle in radians.
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
    get_angle_at_vertex(p0, p1, p2)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def get_line_segment_length(
        p1: types.Point,
        p2: types.Point,
    ) -> float:
        """Compute the Euclidean length of a line segment.

        :param p1: Start point (x, y).
        :param p2: End point (x, y).
        :returns: Length of the segment.
        """
"#,
    module = "raygeo.geo.shape.line"
)]
#[pyfunction(name = "get_line_segment_length")]
fn get_line_segment_length_py(p1: (f64, f64), p2: (f64, f64)) -> f64 {
    get_line_segment_length(p1, p2)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def is_point_inside_rect(
        point: types.Point,
        rect: types.Rect,
    ) -> bool:
        """Check if a point is inside a rectangle.

        :param point: Point (x, y) to test.
        :param rect: Rectangle (x_min, y_min, x_max, y_max).
        :returns: True if the point is inside the rectangle.
        """
"#,
    module = "raygeo.geo.shape.rect"
)]
#[pyfunction(name = "is_point_inside_rect")]
fn is_point_inside_rect_py(point: Point, rect: (f64, f64, f64, f64)) -> bool {
    is_point_inside_rect(point, rect)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def does_rect_contain_rect(
        outer: types.Rect,
        inner: types.Rect,
    ) -> bool:
        """Check if one rectangle contains another.

        :param outer: Outer rectangle (x_min, y_min, x_max, y_max).
        :param inner: Inner rectangle (x_min, y_min, x_max, y_max).
        :returns: True if outer fully contains inner.
        """
"#,
    module = "raygeo.geo.shape.rect"
)]
#[pyfunction(name = "does_rect_contain_rect")]
fn does_rect_contain_rect_py(
    outer: (f64, f64, f64, f64),
    inner: (f64, f64, f64, f64),
) -> bool {
    does_rect_contain_rect(outer, inner)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def does_rect_intersect_rect(
        r1: types.Rect,
        r2: types.Rect,
    ) -> bool:
        """Check if two rectangles intersect.

        :param r1: First rectangle (x_min, y_min, x_max, y_max).
        :param r2: Second rectangle (x_min, y_min, x_max, y_max).
        :returns: True if the rectangles intersect.
        """
"#,
    module = "raygeo.geo.shape.rect"
)]
#[pyfunction(name = "does_rect_intersect_rect")]
fn does_rect_intersect_rect_py(
    r1: (f64, f64, f64, f64),
    r2: (f64, f64, f64, f64),
) -> bool {
    use crate::geo::shape::line::does_rect_intersect_rect;
    does_rect_intersect_rect(r1, r2)
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo.geo.types

    def do_rects_intersect(
        r1: types.Rect,
        r2: types.Rect,
    ) -> bool:
        """Check if two rectangles intersect.

        :param r1: First rectangle (x_min, y_min, x_max, y_max).
        :param r2: Second rectangle (x_min, y_min, x_max, y_max).
        :returns: True if the rectangles intersect.
        """
"#,
    module = "raygeo.geo.shape.rect"
)]
#[pyfunction(name = "do_rects_intersect")]
fn do_rects_intersect_py(
    r1: (f64, f64, f64, f64),
    r2: (f64, f64, f64, f64),
) -> bool {
    do_rects_intersect(r1, r2)
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
) -> (f64, f64, f64) {
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
fn midpoint_py(p1: PyPoint3D, p2: PyPoint3D) -> (f64, f64, f64) {
    let p1_3d = (p1.0, p1.1, p1.2);
    let p2_3d = (p2.0, p2.1, p2.2);
    let result = midpoint(p1_3d, p2_3d);
    (result.0, result.1, result.2)
}
