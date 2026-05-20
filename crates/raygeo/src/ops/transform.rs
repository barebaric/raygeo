use crate::constants::EPSILON_COLLINEAR;

use super::container::Ops;
use super::types::{MoveCmd, OpCategory, OpNode};
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
        let is_non_uniform = (len_x - len_y).abs() > EPSILON_COLLINEAR;

        let det = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
        let flip_cw = det < 0.0;

        let mut new_cmds = Vec::new();
        let mut last_point_untransformed: Option<Point3D> = None;

        for node in &self.commands {
            let mut new_node = node.clone();
            let mut original_cmd_end = None;

            match &node.category {
                OpCategory::Moving { end, cmd } => {
                    original_cmd_end = Some(*end);
                    let new_end = transform_point(matrix, *end);

                    let new_cmd = match cmd {
                        MoveCmd::ArcTo { center, cw } if is_non_uniform => {
                            let start_point = last_point_untransformed
                                .unwrap_or((0.0, 0.0, 0.0));
                            let arc_row: [f64; 8] = [
                                crate::constants::CMD_TYPE_ARC,
                                end.0,
                                end.1,
                                end.2,
                                center.0,
                                center.1,
                                if *cw { 1.0 } else { 0.0 },
                                0.0,
                            ];
                            let segments =
                                crate::geo::shape::arc::linearize_arc(
                                    &arc_row,
                                    start_point,
                                    0.1,
                                );
                            let extra =
                                node.extra_axes.as_deref().map(|e| e.to_vec());
                            for (_, p2) in &segments {
                                let tv = transform_point(matrix, *p2);
                                let mut lcmd = OpNode::line_to(
                                    tv.0,
                                    tv.1,
                                    tv.2,
                                    extra.clone(),
                                );
                                if let Some(s) = &node.state {
                                    lcmd.set_state(s.clone());
                                }
                                new_cmds.push(lcmd);
                            }
                            last_point_untransformed = original_cmd_end;
                            continue; // We broke this into multiple lines, skip the single node push
                        }
                        MoveCmd::ArcTo { center, cw } => {
                            let new_ci = matrix[0][0] * center.0
                                + matrix[0][1] * center.1;
                            let new_cj = matrix[1][0] * center.0
                                + matrix[1][1] * center.1;
                            MoveCmd::ArcTo {
                                center: (new_ci, new_cj),
                                cw: if flip_cw { !cw } else { *cw },
                            }
                        }
                        MoveCmd::BezierTo { c1, c2 } => MoveCmd::BezierTo {
                            c1: transform_point(matrix, *c1),
                            c2: transform_point(matrix, *c2),
                        },
                        MoveCmd::QuadraticBezierTo { control } => {
                            MoveCmd::QuadraticBezierTo {
                                control: transform_point(matrix, *control),
                            }
                        }
                        MoveCmd::MoveTo => MoveCmd::MoveTo,
                        MoveCmd::LineTo => MoveCmd::LineTo,
                        MoveCmd::ScanLine { power_values } => {
                            MoveCmd::ScanLine {
                                power_values: power_values.clone(),
                            }
                        }
                    };

                    new_node.category = OpCategory::Moving {
                        end: new_end,
                        cmd: new_cmd,
                    };
                    new_cmds.push(new_node);
                }
                _ => {
                    new_cmds.push(new_node);
                }
            }

            if let Some(original_end) = original_cmd_end {
                last_point_untransformed = Some(original_end);
            }
        }

        self.commands = new_cmds;
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
