//! Math: Affine transformations and linear algebra operations.
//!
//! This module provides functions for applying affine transformations to
//! geometry data, including uniform and non-uniform scaling, as well as
//! matrix operations for mapping geometry to arbitrary frames.

use crate::geo::geometry::Geometry;
use crate::geo::shape::arc::linearize_arc;
use crate::geo::shape::point::transform_point;
use crate::types::{Command, Point, Point3D, Rect};

fn transform_vec(matrix: &[[f64; 4]; 4], x: f64, y: f64) -> (f64, f64) {
    let vx = matrix[0][0] * x + matrix[0][1] * y;
    let vy = matrix[1][0] * x + matrix[1][1] * y;
    (vx, vy)
}

pub fn mat4_mul(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

fn transform_array_uniform(
    data: &[Command],
    matrix: &[[f64; 4]; 4],
) -> Vec<Command> {
    let mut result: Vec<Command> = Vec::with_capacity(data.len());
    for cmd in data {
        let end_pt = cmd.end_point();
        let p = transform_point(matrix, end_pt.0, end_pt.1, end_pt.2);
        let (nx, ny, nz) = (p.0, p.1, p.2);

        let transformed = match cmd {
            Command::Arc {
                center_offset,
                clockwise,
                ..
            } => {
                let (vi, vj) =
                    transform_vec(matrix, center_offset.0, center_offset.1);
                let det =
                    matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
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
                let c1_t =
                    transform_point(matrix, control1.0, control1.1, control1.2);
                let (c1x, c1y, c1z) = (c1_t.0, c1_t.1, c1_t.2);
                let c2_t =
                    transform_point(matrix, control2.0, control2.1, control2.2);
                let (c2x, c2y, c2z) = (c2_t.0, c2_t.1, c2_t.2);
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
    matrix: &[[f64; 4]; 4],
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
                    let pt = transform_point(matrix, p2.0, p2.1, p2.2);
                    let (tx, ty, tz) = (pt.0, pt.1, pt.2);
                    result.push(Command::Line {
                        end: Point3D(tx, ty, tz),
                    });
                }
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                let p_t = transform_point(
                    matrix,
                    original_end.0,
                    original_end.1,
                    original_end.2,
                );
                let (nx, ny, nz) = (p_t.0, p_t.1, p_t.2);
                let c1_t =
                    transform_point(matrix, control1.0, control1.1, control1.2);
                let (c1x, c1y, c1z) = (c1_t.0, c1_t.1, c1_t.2);
                let c2_t =
                    transform_point(matrix, control2.0, control2.1, control2.2);
                let (c2x, c2y, c2z) = (c2_t.0, c2_t.1, c2_t.2);
                result.push(Command::Bezier {
                    end: Point3D(nx, ny, nz),
                    control1: Point3D(c1x, c1y, c1z),
                    control2: Point3D(c2x, c2y, c2z),
                });
            }
            Command::Move { .. } => {
                let p_t = transform_point(
                    matrix,
                    original_end.0,
                    original_end.1,
                    original_end.2,
                );
                let (nx, ny, nz) = (p_t.0, p_t.1, p_t.2);
                result.push(Command::Move {
                    end: Point3D(nx, ny, nz),
                });
            }
            Command::Line { .. } => {
                let p_t = transform_point(
                    matrix,
                    original_end.0,
                    original_end.1,
                    original_end.2,
                );
                let (nx, ny, nz) = (p_t.0, p_t.1, p_t.2);
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
    matrix: &[[f64; 4]; 4],
) -> Vec<Command> {
    if data.is_empty() {
        return vec![];
    }

    let v_x = [matrix[0][0], matrix[1][0], matrix[2][0], matrix[3][0]];
    let v_y = [matrix[0][1], matrix[1][1], matrix[2][1], matrix[3][1]];
    let len_x_sq = v_x[0] * v_x[0] + v_x[1] * v_x[1];
    let len_y_sq = v_y[0] * v_y[0] + v_y[1] * v_y[1];
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

    let u_vec = (p_width.0 - origin.0, p_width.1 - origin.1);
    let v_vec = (p_height.0 - origin.0, p_height.1 - origin.1);

    let t1: [[f64; 4]; 4] = [
        [1.0, 0.0, 0.0, -anchor_x_value],
        [0.0, 1.0, 0.0, -anchor_y_value],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let t2: [[f64; 4]; 4] = [
        [1.0 / src_width, 0.0, 0.0, 0.0],
        [0.0, 1.0 / src_height, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let t3: [[f64; 4]; 4] = [
        [u_vec.0, v_vec.0, 0.0, origin.0],
        [u_vec.1, v_vec.1, 0.0, origin.1],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let final_matrix = mat4_mul(&t3, &mat4_mul(&t2, &t1));

    let mut transformed_geo = geometry.copy();
    transformed_geo.transform(&final_matrix);
    transformed_geo
}
