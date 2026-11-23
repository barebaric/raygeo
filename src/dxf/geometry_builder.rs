//! Geometry building: turning parsed shapes into `Geometry` paths.

use std::f64::consts::TAU;

use glam::DVec3;

use crate::dxf::entity::{bulge_to_arc, ellipse_point, Shape, Vertex, Winding};
use crate::geo::geometry::Geometry;

/// Tolerance under which an arc's start and end coincide, in radians.
const FULL_CIRCLE_EPS: f64 = 1e-9;

/// Appends one shape to a geometry.
///
/// The shape's start is emitted as a `move_to`, so every entity becomes a
/// separate subpath. Joining those subpaths into contours is a separate
/// step, performed on the finished geometry.
pub fn append_shape(geo: &mut Geometry, shape: &Shape) {
    match shape {
        Shape::Point(_) => {}
        Shape::Line { start, end } => {
            geo.move_to(start.x, start.y, start.z);
            geo.line_to(end.x, end.y, end.z);
        }
        Shape::Circle {
            center,
            radius,
            normal,
        } => append_full_circle(geo, *center, *radius, *normal),
        Shape::Arc {
            center,
            radius,
            start_angle,
            end_angle,
            winding,
        } => append_arc(
            geo,
            *center,
            *radius,
            *start_angle,
            *end_angle,
            *winding,
        ),
        Shape::Ellipse {
            center,
            major_axis,
            minor_ratio,
            start_param,
            end_param,
        } => append_ellipse(
            geo,
            *center,
            *major_axis,
            *minor_ratio,
            *start_param,
            *end_param,
        ),
        Shape::Polyline { vertices, closed } => {
            append_polyline(geo, vertices, *closed)
        }
        Shape::Spline { segments } => append_beziers(geo, segments),
    }
}

fn append_arc(
    geo: &mut Geometry,
    center: DVec3,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    winding: Winding,
) {
    if radius <= 0.0 {
        return;
    }
    let sweep = sweep_between(start_angle, end_angle, winding);
    if sweep.abs() >= TAU - FULL_CIRCLE_EPS {
        let normal = match winding {
            Winding::CounterClockwise => DVec3::Z,
            Winding::Clockwise => -DVec3::Z,
        };
        append_full_circle(geo, center, radius, normal);
        return;
    }
    // The arc is drawn from its start angle towards its end angle; for a
    // clockwise arc the start angle is the larger one, so swap so the
    // path always travels in the stated direction.
    let (from_angle, to_angle) = match winding {
        Winding::CounterClockwise => (start_angle, end_angle),
        Winding::Clockwise => (end_angle, start_angle),
    };
    let from = angle_point(center, radius, from_angle);
    let to = angle_point(center, radius, to_angle);
    geo.move_to(from.x, from.y, from.z);
    geo.arc_to(
        to.x,
        to.y,
        center.x - from.x,
        center.y - from.y,
        winding == Winding::Clockwise,
        to.z,
    );
}

fn append_full_circle(
    geo: &mut Geometry,
    center: DVec3,
    radius: f64,
    normal: DVec3,
) {
    if radius <= 0.0 {
        return;
    }
    let (u, v) = if normal.z < 0.0 {
        (DVec3::X, -DVec3::Y)
    } else {
        (DVec3::X, DVec3::Y)
    };
    let start = center + radius * u;
    let top = center + radius * v;
    let bottom = center - radius * v;
    let clockwise = normal.z < 0.0;
    geo.move_to(start.x, start.y, start.z);
    geo.arc_to(
        top.x,
        top.y,
        center.x - start.x,
        center.y - start.y,
        clockwise,
        top.z,
    );
    geo.arc_to(
        bottom.x,
        bottom.y,
        center.x - top.x,
        center.y - top.y,
        clockwise,
        bottom.z,
    );
    geo.arc_to(
        start.x,
        start.y,
        center.x - bottom.x,
        center.y - bottom.y,
        clockwise,
        start.z,
    );
}

fn append_ellipse(
    geo: &mut Geometry,
    center: DVec3,
    major_axis: DVec3,
    minor_ratio: f64,
    start_param: f64,
    end_param: f64,
) {
    if major_axis.length_squared() < 1e-18 {
        return;
    }
    let (start_param, sweep) = ellipse_sweep(start_param, end_param);
    let steps = ((sweep.abs() / (TAU / 16.0)).ceil() as usize).clamp(4, 128);
    let first = ellipse_point(center, major_axis, minor_ratio, start_param);
    geo.move_to(first.x, first.y, first.z);
    for step in 1..=steps {
        let parameter = start_param + sweep * step as f64 / steps as f64;
        let point = ellipse_point(center, major_axis, minor_ratio, parameter);
        geo.line_to(point.x, point.y, point.z);
    }
}

fn append_polyline(geo: &mut Geometry, vertices: &[Vertex], closed: bool) {
    if vertices.len() < 2 {
        if let Some(vertex) = vertices.first() {
            geo.move_to(
                vertex.position.x,
                vertex.position.y,
                vertex.position.z,
            );
        }
        return;
    }
    let first = vertices[0].position;
    geo.move_to(first.x, first.y, first.z);
    let count = if closed {
        vertices.len()
    } else {
        vertices.len() - 1
    };
    for index in 0..count {
        let from = vertices[index];
        let to = vertices[(index + 1) % vertices.len()];
        append_segment(geo, from, to);
    }
}

fn append_segment(geo: &mut Geometry, from: Vertex, to: Vertex) {
    let end = to.position;
    if !from.has_bulge() {
        geo.line_to(end.x, end.y, end.z);
        return;
    }
    match bulge_to_arc(from.position, end, from.bulge) {
        Some(arc) => {
            let start = from.position;
            let center = arc.center_3d((start.z + end.z) * 0.5);
            geo.arc_to(
                end.x,
                end.y,
                center.x - start.x,
                center.y - start.y,
                arc.winding == Winding::Clockwise,
                end.z,
            );
        }
        None => {
            geo.line_to(end.x, end.y, end.z);
        }
    }
}

fn append_beziers(geo: &mut Geometry, segments: &[[DVec3; 4]]) {
    let Some(first) = segments.first() else {
        return;
    };
    geo.move_to(first[0].x, first[0].y, first[0].z);
    for segment in segments {
        geo.bezier_to(segment[1], segment[2], segment[3]);
    }
}

/// The point at the given angle on a circle in the XY plane.
fn angle_point(center: DVec3, radius: f64, angle: f64) -> DVec3 {
    center + DVec3::new(radius * angle.cos(), radius * angle.sin(), 0.0)
}

/// The signed sweep from `start_angle` to `end_angle` in the given
/// direction, in `(-2π, 2π)`.
fn sweep_between(start_angle: f64, end_angle: f64, winding: Winding) -> f64 {
    let mut sweep = end_angle - start_angle;
    match winding {
        Winding::CounterClockwise => {
            while sweep <= 0.0 {
                sweep += TAU;
            }
        }
        Winding::Clockwise => {
            while sweep >= 0.0 {
                sweep -= TAU;
            }
        }
    }
    sweep
}

/// Normalises an ellipse's parameter range into a start and a positive
/// sweep.
fn ellipse_sweep(start_param: f64, end_param: f64) -> (f64, f64) {
    let mut start = start_param.rem_euclid(TAU);
    let mut sweep = end_param - start_param;
    if sweep.abs() < FULL_CIRCLE_EPS {
        sweep = TAU;
    } else if sweep < 0.0 {
        sweep += TAU;
    }
    if start < 0.0 {
        start += TAU;
    }
    (start, sweep)
}

/// Converts a curved shape into cubic beziers.
///
/// Used when a shape must survive a non-uniform transform, which bends a
/// circle into an ellipse that a `Geometry` arc cannot represent.
pub fn shape_to_beziers(shape: &Shape) -> Vec<[DVec3; 4]> {
    match shape {
        Shape::Circle {
            center,
            radius,
            normal,
        } => circle_beziers(*center, *radius, *normal),
        Shape::Arc {
            center,
            radius,
            start_angle,
            end_angle,
            winding,
        } => arc_beziers(*center, *radius, *start_angle, *end_angle, *winding),
        Shape::Ellipse {
            center,
            major_axis,
            minor_ratio,
            start_param,
            end_param,
        } => ellipse_beziers(
            *center,
            *major_axis,
            *minor_ratio,
            *start_param,
            *end_param,
        ),
        Shape::Spline { segments } => segments.clone(),
        Shape::Line { start, end } => vec![[*start, *start, *end, *end]],
        Shape::Point(_) => Vec::new(),
        Shape::Polyline { .. } => Vec::new(),
    }
}

/// A full circle as four cubic beziers.
fn circle_beziers(
    center: DVec3,
    radius: f64,
    normal: DVec3,
) -> Vec<[DVec3; 4]> {
    let (u, v) = if normal.z < 0.0 {
        (DVec3::X, -DVec3::Y)
    } else {
        (DVec3::X, DVec3::Y)
    };
    let k = crate::geo::shape::bezier::get_bezier_arc_constant(
        std::f64::consts::FRAC_PI_2,
    ) * radius;
    let points = [u, v, -u, -v, u];
    let mut segments = Vec::new();
    for index in 0..4 {
        let from = center + points[index] * radius;
        let to = center + points[index + 1] * radius;
        let tangent_from = points[index].cross(normal.normalize()) * k;
        let tangent_to = points[index + 1].cross(normal.normalize()) * k;
        segments.push([from, from + tangent_from, to - tangent_to, to]);
    }
    segments
}

/// An arc as cubic beziers, one per quarter circle at most.
fn arc_beziers(
    center: DVec3,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    winding: Winding,
) -> Vec<[DVec3; 4]> {
    let sweep = sweep_between(start_angle, end_angle, winding);
    let steps =
        ((sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let step = sweep / steps as f64;
    let k = crate::geo::shape::bezier::get_bezier_arc_constant(step) * radius;
    let mut segments = Vec::new();
    for index in 0..steps {
        let angle_from = start_angle + step * index as f64;
        let angle_to = angle_from + step;
        let from = center
            + DVec3::new(
                radius * angle_from.cos(),
                radius * angle_from.sin(),
                0.0,
            );
        let to = center
            + DVec3::new(radius * angle_to.cos(), radius * angle_to.sin(), 0.0);
        let tangent_from = DVec3::new(-angle_from.sin(), angle_from.cos(), 0.0)
            * k
            * sweep.signum();
        let tangent_to = DVec3::new(-angle_to.sin(), angle_to.cos(), 0.0)
            * k
            * sweep.signum();
        segments.push([from, from + tangent_from, to - tangent_to, to]);
    }
    segments
}

/// An elliptical arc as cubic beziers.
fn ellipse_beziers(
    center: DVec3,
    major_axis: DVec3,
    minor_ratio: f64,
    start_param: f64,
    end_param: f64,
) -> Vec<[DVec3; 4]> {
    let (start, sweep) = ellipse_sweep(start_param, end_param);
    let steps = ((sweep / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let step = sweep / steps as f64;
    let mut segments = Vec::new();
    for index in 0..steps {
        let from_param = start + step * index as f64;
        let to_param = from_param + step;
        let from = ellipse_point(center, major_axis, minor_ratio, from_param);
        let to = ellipse_point(center, major_axis, minor_ratio, to_param);
        let from_tangent = ellipse_tangent(major_axis, minor_ratio, from_param);
        let to_tangent = ellipse_tangent(major_axis, minor_ratio, to_param);
        let k = crate::geo::shape::bezier::get_bezier_arc_constant(step);
        segments.push([from, from + from_tangent * k, to - to_tangent * k, to]);
    }
    segments
}

/// The derivative of an ellipse at the given parameter.
fn ellipse_tangent(
    major_axis: DVec3,
    minor_ratio: f64,
    parameter: f64,
) -> DVec3 {
    let major = major_axis;
    let minor = minor_ratio
        * major_axis.length()
        * DVec3::Z.cross(major.normalize()).normalize();
    -major * parameter.sin() + minor * parameter.cos()
}
