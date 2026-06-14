//! Math: Affine transformations and linear algebra operations.
//!
//! This module provides functions for applying affine transformations to
//! geometry data, including uniform and non-uniform scaling, as well as
//! matrix operations for mapping geometry to arbitrary frames.

use glam::{DMat3, DMat4, DVec2, DVec3, DVec4};

use crate::geo::geometry::Geometry;
use crate::geo::shape::arc::linearize_arc;
use crate::geo::shape::point::transform_point;
use crate::types::{Command, Point, Point3D, Rect};

/// Transform a 2D point by a 3x3 affine matrix (homogeneous coordinates).
pub fn mat3_transform(m: DMat3, x: f64, y: f64) -> (f64, f64) {
    let r = m.transform_point2(DVec2::new(x, y));
    (r.x, r.y)
}

/// Determinant of the 2x2 sub-matrix (linear part) of a 3x3 matrix.
pub fn mat3_det2x2(m: DMat3) -> f64 {
    m.x_axis.x * m.y_axis.y - m.y_axis.x * m.x_axis.y
}

fn transform_vec(matrix: DMat4, x: f64, y: f64) -> (f64, f64) {
    let r = matrix.transform_vector3(DVec3::new(x, y, 0.0));
    (r.x, r.y)
}

fn transform_array_uniform(data: &[Command], matrix: DMat4) -> Vec<Command> {
    let mut result: Vec<Command> = Vec::with_capacity(data.len());
    for cmd in data {
        let end_pt = cmd.end_point();
        let p = transform_point(matrix, end_pt);
        let (nx, ny, nz) = (p.x(), p.y(), p.z());

        let transformed = match cmd {
            Command::Arc {
                center_offset,
                clockwise,
                ..
            } => {
                let (vi, vj) =
                    transform_vec(matrix, center_offset.x(), center_offset.y());
                let det = matrix.x_axis.x * matrix.y_axis.y
                    - matrix.y_axis.x * matrix.x_axis.y;
                let cw = if det < 0.0 { !*clockwise } else { *clockwise };
                Command::Arc {
                    end: Point3D(nx, ny, nz),
                    center_offset: Point(vi, vj),
                    clockwise: cw,
                }
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                let c1_t = transform_point(matrix, *control1);
                let (c1x, c1y, c1z) = (c1_t.x(), c1_t.y(), c1_t.z());
                let c2_t = transform_point(matrix, *control2);
                let (c2x, c2y, c2z) = (c2_t.x(), c2_t.y(), c2_t.z());
                Command::Bezier {
                    end: Point3D(nx, ny, nz),
                    control1: Point3D(c1x, c1y, c1z),
                    control2: Point3D(c2x, c2y, c2z),
                }
            }
            Command::Move { .. } => Command::Move {
                end: Point3D(nx, ny, nz),
            },
            Command::Line { .. } => Command::Line {
                end: Point3D(nx, ny, nz),
            },
        };

        result.push(transformed);
    }
    result
}

fn transform_array_non_uniform(
    data: &[Command],
    matrix: DMat4,
) -> Vec<Command> {
    let mut result: Vec<Command> = Vec::new();
    let mut last_pos: Point3D = Point3D(0.0, 0.0, 0.0);

    for cmd in data {
        let original_end = cmd.end_point();

        match cmd {
            Command::Arc {
                end,
                center_offset,
                clockwise,
                ..
            } => {
                let start_pt = last_pos;
                let segments = linearize_arc(
                    *end,
                    *center_offset,
                    *clockwise,
                    start_pt,
                    0.1,
                );
                for (_, p2) in segments {
                    let pt = transform_point(matrix, p2);
                    let (tx, ty, tz) = (pt.x(), pt.y(), pt.z());
                    result.push(Command::Line {
                        end: Point3D(tx, ty, tz),
                    });
                }
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                let p_t = transform_point(matrix, original_end);
                let (nx, ny, nz) = (p_t.x(), p_t.y(), p_t.z());
                let c1_t = transform_point(matrix, *control1);
                let (c1x, c1y, c1z) = (c1_t.x(), c1_t.y(), c1_t.z());
                let c2_t = transform_point(matrix, *control2);
                let (c2x, c2y, c2z) = (c2_t.x(), c2_t.y(), c2_t.z());
                result.push(Command::Bezier {
                    end: Point3D(nx, ny, nz),
                    control1: Point3D(c1x, c1y, c1z),
                    control2: Point3D(c2x, c2y, c2z),
                });
            }
            Command::Move { .. } => {
                let p_t = transform_point(matrix, original_end);
                let (nx, ny, nz) = (p_t.x(), p_t.y(), p_t.z());
                result.push(Command::Move {
                    end: Point3D(nx, ny, nz),
                });
            }
            Command::Line { .. } => {
                let p_t = transform_point(matrix, original_end);
                let (nx, ny, nz) = (p_t.x(), p_t.y(), p_t.z());
                result.push(Command::Line {
                    end: Point3D(nx, ny, nz),
                });
            }
        }

        last_pos = original_end;
    }
    result
}

/// Applies an affine transformation matrix to geometry commands.
/// Handles uniform and non-uniform scaling (linearizing arcs for the latter).
/// Returns the transformed commands. For non-uniform scaling, the result
/// may be longer than the input (arcs are linearized into lines).
pub fn apply_affine_transform_to_array(
    data: &[Command],
    matrix: DMat4,
) -> Vec<Command> {
    if data.is_empty() {
        return vec![];
    }

    let len_x_sq = matrix.x_axis.x.powi(2) + matrix.x_axis.y.powi(2);
    let len_y_sq = matrix.y_axis.x.powi(2) + matrix.y_axis.y.powi(2);
    let is_non_uniform = (len_x_sq - len_y_sq).abs() > 1e-9;

    if is_non_uniform {
        transform_array_non_uniform(data, matrix)
    } else {
        transform_array_uniform(data, matrix)
    }
}

/// Transforms a Geometry object to fit into an affine frame defined by
/// three points.
#[allow(clippy::too_many_arguments)]
pub fn map_geometry_to_frame(
    geometry: &Geometry,
    origin: Point,
    p_width: Point,
    p_height: Point,
    anchor_y: Option<f64>,
    stable_src_height: Option<f64>,
    anchor_x: Option<f64>,
    stable_src_width: Option<f64>,
) -> Geometry {
    if geometry.is_empty() {
        return Geometry::new();
    }

    let Rect(min_x, min_y, max_x, max_y) = geometry.rect();
    let src_width = stable_src_width.unwrap_or(max_x - min_x);
    let src_height = stable_src_height.unwrap_or(max_y - min_y);

    let anchor_x_value = anchor_x.unwrap_or(min_x);
    let anchor_y_value = anchor_y.unwrap_or(min_y);

    if src_width < 1e-9 || src_height < 1e-9 {
        return Geometry::new();
    }

    let u_vec = p_width - origin;
    let v_vec = p_height - origin;

    let t1 = DMat4::from_cols(
        DVec4::new(1.0, 0.0, 0.0, 0.0),
        DVec4::new(0.0, 1.0, 0.0, 0.0),
        DVec4::new(0.0, 0.0, 1.0, 0.0),
        DVec4::new(-anchor_x_value, -anchor_y_value, 0.0, 1.0),
    );

    let t2 = DMat4::from_cols(
        DVec4::new(1.0 / src_width, 0.0, 0.0, 0.0),
        DVec4::new(0.0, 1.0 / src_height, 0.0, 0.0),
        DVec4::new(0.0, 0.0, 1.0, 0.0),
        DVec4::new(0.0, 0.0, 0.0, 1.0),
    );

    let t3 = DMat4::from_cols(
        DVec4::new(u_vec.x(), u_vec.y(), 0.0, 0.0),
        DVec4::new(v_vec.x(), v_vec.y(), 0.0, 0.0),
        DVec4::new(0.0, 0.0, 1.0, 0.0),
        DVec4::new(origin.x(), origin.y(), 0.0, 1.0),
    );

    let final_matrix = t3 * t2 * t1;

    let mut transformed_geo = geometry.copy();
    transformed_geo.transform(&final_matrix);
    transformed_geo
}
