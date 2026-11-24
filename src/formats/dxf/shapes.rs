//! Whole-shape operations: transforming and measuring parsed shapes.
//!
//! These operate on a [`Shape`] as a unit, which is what an `INSERT`
//! needs: a block's contents are moved, rotated and scaled as a group
//! before they become part of the drawing.

use glam::DMat4;

use crate::formats::dxf::entity::{Shape, Vertex};

/// Applies an affine transform to a shape in place.
///
/// The transform may scale non-uniformly, so circular arcs and circles
/// become ellipses and are converted to bezier curves. Splines are
/// already beziers and are transformed point by point.
pub fn transform_shape(shape: &Shape, transform: DMat4) -> Shape {
    match shape {
        Shape::Point(point) => Shape::Point(transform.transform_point3(*point)),
        Shape::Line { start, end } => Shape::Line {
            start: transform.transform_point3(*start),
            end: transform.transform_point3(*end),
        },
        Shape::Polyline { vertices, closed } => Shape::Polyline {
            vertices: vertices
                .iter()
                .map(|vertex| transform_vertex(*vertex, transform))
                .collect(),
            closed: *closed,
        },
        Shape::Spline { segments } => Shape::Spline {
            segments: segments
                .iter()
                .map(|segment| {
                    [
                        transform.transform_point3(segment[0]),
                        transform.transform_point3(segment[1]),
                        transform.transform_point3(segment[2]),
                        transform.transform_point3(segment[3]),
                    ]
                })
                .collect(),
        },
        Shape::Arc { .. } | Shape::Circle { .. } | Shape::Ellipse { .. } => {
            Shape::Spline {
                segments:
                    crate::formats::dxf::geometry_builder::shape_to_beziers(
                        shape,
                    ),
            }
        }
    }
}

fn transform_vertex(vertex: Vertex, transform: DMat4) -> Vertex {
    Vertex {
        position: transform.transform_point3(vertex.position),
        bulge: vertex.bulge,
    }
}

/// The range of Z values a shape spans.
pub fn shape_z_range(shape: &Shape) -> (f64, f64) {
    match shape {
        Shape::Point(point) => (point.z, point.z),
        Shape::Line { start, end } => (start.z.min(end.z), start.z.max(end.z)),
        Shape::Arc { center, .. }
        | Shape::Circle { center, .. }
        | Shape::Ellipse { center, .. } => (center.z, center.z),
        Shape::Polyline { vertices, .. } => vertices.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(low, high), vertex| {
                (low.min(vertex.position.z), high.max(vertex.position.z))
            },
        ),
        Shape::Spline { segments } => segments.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(low, high), segment| {
                let zs =
                    [segment[0].z, segment[1].z, segment[2].z, segment[3].z];
                let lo = zs.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = zs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                (low.min(lo), high.max(hi))
            },
        ),
    }
}
