use glam::{DMat4, DVec2, DVec3, DVec4};

use crate::constants::EPSILON_COLLINEAR;
use crate::geo::shape::point::transform_point_3d;

use super::container::Ops;
use super::types::{MoveCmd, OpCategory, OpNode};
use crate::types::{Point, Point3D};

impl Ops {
    pub fn transform(&mut self, matrix: DMat4) -> &mut Self {
        let vx = DVec2::new(matrix.x_axis.x, matrix.x_axis.y);
        let vy = DVec2::new(matrix.y_axis.x, matrix.y_axis.y);
        let is_non_uniform =
            (vx.length() - vy.length()).abs() > EPSILON_COLLINEAR;

        let det = matrix.x_axis.x * matrix.y_axis.y
            - matrix.y_axis.x * matrix.x_axis.y;
        let flip_cw = det < 0.0;

        let mut new_cmds = Vec::new();
        let mut last_point_untransformed: Option<Point3D> = None;

        for node in &self.commands {
            let mut new_node = node.clone();
            let mut original_cmd_end = None;

            match &node.category {
                OpCategory::Moving { end, cmd } => {
                    original_cmd_end = Some(*end);
                    let new_end = transform_point_3d(matrix, *end);

                    let new_cmd = match cmd {
                        MoveCmd::ArcTo { center, cw } if is_non_uniform => {
                            let start_point = last_point_untransformed
                                .unwrap_or(Point3D::new(0.0, 0.0, 0.0));
                            let mut arc_buf = Vec::new();
                            let center_3d =
                                Point3D::new(center.x, center.y, 0.0);
                            let normal = if *cw {
                                Point3D::new(0.0, 0.0, -1.0)
                            } else {
                                Point3D::new(0.0, 0.0, 1.0)
                            };
                            crate::geo::shape::arc::linearize_arc(
                                *end,
                                center_3d,
                                normal,
                                start_point,
                                0.1,
                                &mut arc_buf,
                            );
                            let extra =
                                node.extra_axes.as_deref().map(|e| e.to_vec());
                            for (_, p2) in &arc_buf {
                                let tv = transform_point_3d(matrix, *p2);
                                let mut lcmd = OpNode::line_to(
                                    tv.x,
                                    tv.y,
                                    tv.z,
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
                            let new_vec = matrix.transform_vector3(DVec3::new(
                                center.x, center.y, 0.0,
                            ));
                            MoveCmd::ArcTo {
                                center: Point::new(new_vec.x, new_vec.y),
                                cw: if flip_cw { !cw } else { *cw },
                            }
                        }
                        MoveCmd::BezierTo { control1, control2 } => {
                            MoveCmd::BezierTo {
                                control1: transform_point_3d(matrix, *control1),
                                control2: transform_point_3d(matrix, *control2),
                            }
                        }
                        MoveCmd::QuadraticBezierTo { control } => {
                            MoveCmd::QuadraticBezierTo {
                                control: transform_point_3d(matrix, *control),
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
        self.last_move_to = transform_point_3d(matrix, self.last_move_to);
        self
    }

    pub fn translate(&mut self, dx: f64, dy: f64, dz: f64) -> &mut Self {
        let matrix = DMat4::from_cols(
            DVec4::new(1.0, 0.0, 0.0, 0.0),
            DVec4::new(0.0, 1.0, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.0, 0.0),
            DVec4::new(dx, dy, dz, 1.0),
        );
        self.transform(matrix)
    }

    pub fn scale(&mut self, sx: f64, sy: f64, sz: f64) -> &mut Self {
        let matrix = DMat4::from_cols(
            DVec4::new(sx, 0.0, 0.0, 0.0),
            DVec4::new(0.0, sy, 0.0, 0.0),
            DVec4::new(0.0, 0.0, sz, 0.0),
            DVec4::new(0.0, 0.0, 0.0, 1.0),
        );
        self.transform(matrix)
    }

    pub fn rotate(&mut self, angle_deg: f64, cx: f64, cy: f64) -> &mut Self {
        let angle_rad = angle_deg.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let translate_to_origin = DMat4::from_cols(
            DVec4::new(1.0, 0.0, 0.0, 0.0),
            DVec4::new(0.0, 1.0, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.0, 0.0),
            DVec4::new(-cx, -cy, 0.0, 1.0),
        );
        let rotation_mat = DMat4::from_cols(
            DVec4::new(cos_a, sin_a, 0.0, 0.0),
            DVec4::new(-sin_a, cos_a, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.0, 0.0),
            DVec4::new(0.0, 0.0, 0.0, 1.0),
        );
        let translate_back = DMat4::from_cols(
            DVec4::new(1.0, 0.0, 0.0, 0.0),
            DVec4::new(0.0, 1.0, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.0, 0.0),
            DVec4::new(cx, cy, 0.0, 1.0),
        );

        let matrix = translate_back * rotation_mat * translate_to_origin;
        self.transform(matrix)
    }
}
