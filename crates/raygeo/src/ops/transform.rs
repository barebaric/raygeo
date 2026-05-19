use std::sync::Arc;

use super::container::Ops;
use super::enums::{CommandCategory, CommandType};
use super::soa::{OpCommand, OpMetadata, SoA};
use crate::types::Point3D;

fn transform_point(matrix: &[[f64; 4]; 4], p: Point3D) -> Point3D {
    (
        matrix[0][0] * p.0
            + matrix[0][1] * p.1
            + matrix[0][2] * p.2
            + matrix[0][3],
        matrix[1][0] * p.0
            + matrix[1][1] * p.1
            + matrix[1][2] * p.2
            + matrix[1][3],
        matrix[2][0] * p.0
            + matrix[2][1] * p.1
            + matrix[2][2] * p.2
            + matrix[2][3],
    )
}

fn mat4_mul(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            result[i][j] = a[i][0] * b[0][j]
                + a[i][1] * b[1][j]
                + a[i][2] * b[2][j]
                + a[i][3] * b[3][j];
        }
    }
    result
}

impl Ops {
    pub fn transform(&mut self, matrix: &[[f64; 4]; 4]) -> &mut Self {
        let vx = [matrix[0][0], matrix[1][0]];
        let vy = [matrix[0][1], matrix[1][1]];
        let len_x = (vx[0] * vx[0] + vx[1] * vx[1]).sqrt();
        let len_y = (vy[0] * vy[0] + vy[1] * vy[1]).sqrt();
        let is_non_uniform = (len_x - len_y).abs() > 1e-9;

        let det = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
        let flip_cw = det < 0.0;

        let mut new_soa = SoA::new();
        let mut last_point_untransformed: Option<Point3D> = None;

        for i in 0..self.soa.len() {
            let ct = self.soa.command_type(i);
            let cat = self.soa.category(i);
            let original_cmd_end = if cat == CommandCategory::Moving {
                Some(self.soa.endpoint(i))
            } else {
                None
            };

            if ct == CommandType::ArcTo && is_non_uniform {
                let start_point =
                    last_point_untransformed.unwrap_or((0.0, 0.0, 0.0));
                let end = self.soa.endpoint(i);
                let &(ci, cj, cw) = self.soa.arc_params(i);
                let arc_row: [f64; 8] = [
                    crate::constants::CMD_TYPE_ARC,
                    end.0,
                    end.1,
                    end.2,
                    ci,
                    cj,
                    if cw { 1.0 } else { 0.0 },
                    0.0,
                ];
                let segments = crate::geo::shape::arc::linearize_arc(
                    &arc_row,
                    start_point,
                    0.1,
                );
                let st = self.soa.state(i);
                let ea = self.soa.extra_axes(i);
                for (_, p2) in &segments {
                    let tv = transform_point(matrix, *p2);
                    let mut cmd = OpCommand::new(CommandType::LineTo);
                    cmd.end = tv;
                    if let Some(ea) = ea {
                        cmd.extra_axes = Some(Arc::from(ea));
                    }
                    if let Some(st) = st {
                        cmd.state = Some(st.clone());
                    }
                    new_soa.push(cmd);
                }
            } else if cat == CommandCategory::Moving {
                let end = self.soa.endpoint(i);
                let new_end = transform_point(matrix, end);
                let st = self.soa.state(i);
                let ea = self.soa.extra_axes(i);

                let mut cmd = OpCommand::new(ct);
                cmd.end = new_end;

                if ct == CommandType::ArcTo {
                    let &(ci, cj, cw) = self.soa.arc_params(i);
                    let new_ci = matrix[0][0] * ci + matrix[0][1] * cj;
                    let new_cj = matrix[1][0] * ci + matrix[1][1] * cj;
                    let new_cw = if flip_cw { !cw } else { cw };
                    cmd.metadata = OpMetadata::Arc((new_ci, new_cj, new_cw));
                } else if ct == CommandType::BezierTo {
                    let &(c1, c2) = self.soa.bezier_params(i);
                    let t_c1 = transform_point(matrix, c1);
                    let t_c2 = transform_point(matrix, c2);
                    cmd.metadata = OpMetadata::Bezier((t_c1, t_c2));
                } else if ct == CommandType::QuadraticBezierTo {
                    let c = self.soa.quad_params(i);
                    let t_c = transform_point(matrix, *c);
                    cmd.metadata = OpMetadata::QuadraticBezier(t_c);
                }

                if let Some(ea) = ea {
                    cmd.extra_axes = Some(Arc::from(ea));
                }
                if let Some(st) = st {
                    cmd.state = Some(st.clone());
                }
                new_soa.push(cmd);
            } else {
                new_soa.push(self.soa.commands[i].clone());
            }

            if let Some(original_end) = original_cmd_end {
                last_point_untransformed = Some(original_end);
            }
        }

        self.soa = new_soa;
        self.invalidate_time_cache();
        self.last_move_to = transform_point(matrix, self.last_move_to);
        self
    }

    pub fn translate(&mut self, dx: f64, dy: f64, dz: f64) -> &mut Self {
        let matrix = [
            [1.0, 0.0, 0.0, dx],
            [0.0, 1.0, 0.0, dy],
            [0.0, 0.0, 1.0, dz],
            [0.0, 0.0, 0.0, 1.0],
        ];
        self.transform(&matrix)
    }

    pub fn scale(&mut self, sx: f64, sy: f64, sz: f64) -> &mut Self {
        let matrix = [
            [sx, 0.0, 0.0, 0.0],
            [0.0, sy, 0.0, 0.0],
            [0.0, 0.0, sz, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        self.transform(&matrix)
    }

    pub fn rotate(&mut self, angle_deg: f64, cx: f64, cy: f64) -> &mut Self {
        let angle_rad = angle_deg.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let translate_to_origin = [
            [1.0, 0.0, 0.0, -cx],
            [0.0, 1.0, 0.0, -cy],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let rotation_matrix = [
            [cos_a, -sin_a, 0.0, 0.0],
            [sin_a, cos_a, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let translate_back = [
            [1.0, 0.0, 0.0, cx],
            [0.0, 1.0, 0.0, cy],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let m = mat4_mul(&rotation_matrix, &translate_to_origin);
        let matrix = mat4_mul(&translate_back, &m);
        self.transform(&matrix)
    }
}
