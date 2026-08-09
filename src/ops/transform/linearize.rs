use crate::geo::shape::arc::linearize_arc;
use crate::geo::types::Point3D;
use crate::ops::axis::Axis;
use crate::ops::container::Ops;
use crate::ops::types::{MoveCmd, OpCategory, OpNode};

use std::sync::Arc;

fn linearize_scanline(
    end: Point3D,
    start_point: Point3D,
    power_values: &[u8],
    extra: Option<Vec<(Axis, f64)>>,
) -> Ops {
    let num_steps = power_values.len();
    if num_steps == 0 {
        return Ops::new();
    }

    let mut result = Ops::new();
    let seg_start_power = power_values[0];
    result.set_power(seg_start_power as f64 / 255.0);

    if num_steps > 1 {
        let line_vec = (
            end.x - start_point.x,
            end.y - start_point.y,
            end.z - start_point.z,
        );
        let mut cur_start_power = seg_start_power;
        for (i, &cur_power) in
            power_values.iter().enumerate().take(num_steps).skip(1)
        {
            if cur_power != cur_start_power {
                let t = i as f64 / num_steps as f64;
                let seg_end = (
                    start_point.x + t * line_vec.0,
                    start_point.y + t * line_vec.1,
                    start_point.z + t * line_vec.2,
                );
                result.line_to(seg_end.0, seg_end.1, seg_end.2, extra.clone());
                cur_start_power = cur_power;
                result.set_power(cur_start_power as f64 / 255.0);
            }
        }
    }

    result.line_to(end.x, end.y, end.z, extra);
    result
}

fn linearize_arc_to(
    end: Point3D,
    start_point: Point3D,
    center: Point3D,
    cw: bool,
    extra: Option<Vec<(Axis, f64)>>,
) -> Ops {
    let mut arc_buf = Vec::new();
    let normal = if cw {
        Point3D::new(0.0, 0.0, -1.0)
    } else {
        Point3D::new(0.0, 0.0, 1.0)
    };
    linearize_arc(end, center, normal, start_point, 0.1, &mut arc_buf);

    let mut result = Ops::new();
    for (_, seg_end) in arc_buf {
        result.line_to(seg_end.x, seg_end.y, seg_end.z, extra.clone());
    }
    result
}

fn linearize_bezier_to(
    start_point: Point3D,
    control1: Point3D,
    control2: Point3D,
    end: Point3D,
    extra: Option<Vec<(Axis, f64)>>,
) -> Ops {
    let polyline = crate::geo::shape::bezier::linearize_bezier_segment(
        start_point,
        control1,
        control2,
        end,
        None,
    );

    let mut result = Ops::new();
    for pt in polyline.iter().skip(1) {
        result.line_to(pt.x, pt.y, pt.z, extra.clone());
    }
    result
}

fn linearize_quadratic_to(
    start_point: Point3D,
    control: Point3D,
    end: Point3D,
    extra: Option<Vec<(Axis, f64)>>,
) -> Ops {
    let control1 = Point3D::new(
        start_point.x + (2.0 / 3.0) * (control.x - start_point.x),
        start_point.y + (2.0 / 3.0) * (control.y - start_point.y),
        start_point.z + (2.0 / 3.0) * (control.z - start_point.z),
    );
    let control2 = Point3D::new(
        end.x + (2.0 / 3.0) * (control.x - end.x),
        end.y + (2.0 / 3.0) * (control.y - end.y),
        end.z + (2.0 / 3.0) * (control.z - end.z),
    );
    linearize_bezier_to(start_point, control1, control2, end, extra)
}

fn linearize_move_or_line(
    end: Point3D,
    is_move: bool,
    extra: Option<Vec<(Axis, f64)>>,
) -> Ops {
    let mut result = Ops::new();
    if is_move {
        result.move_to(end.x, end.y, end.z, extra);
    } else {
        result.line_to(end.x, end.y, end.z, extra);
    }
    result
}

fn own_extra_axes(node: &OpNode) -> Option<Vec<(Axis, f64)>> {
    node.extra_axes.as_ref().map(|e| e.to_vec())
}

pub fn linearize_node(node: &OpNode, start_point: Point3D) -> Ops {
    let extra = own_extra_axes(node);

    if let OpCategory::Moving { end, cmd } = &node.category {
        let end = *end;
        match cmd {
            MoveCmd::ScanLine { power_values } => {
                linearize_scanline(end, start_point, power_values, extra)
            }
            MoveCmd::ArcTo { center, cw } => linearize_arc_to(
                end,
                start_point,
                Point3D::new(center.x, center.y, 0.0),
                *cw,
                extra,
            ),
            MoveCmd::BezierTo { control1, control2 } => linearize_bezier_to(
                start_point,
                *control1,
                *control2,
                end,
                extra,
            ),
            MoveCmd::QuadraticBezierTo { control } => {
                linearize_quadratic_to(start_point, *control, end, extra)
            }
            MoveCmd::MoveTo | MoveCmd::LineTo => linearize_move_or_line(
                end,
                matches!(cmd, MoveCmd::MoveTo),
                extra,
            ),
        }
    } else {
        Ops::new()
    }
}

impl Ops {
    pub fn linearize(&self, idx: usize, start_point: Point3D) -> Ops {
        linearize_node(&self.commands[idx], start_point)
    }

    pub fn linearize_all(&mut self) {
        let mut new_cmds = Vec::new();
        let mut last_point: Point3D = Point3D::new(0.0, 0.0, 0.0);

        for node in self.commands.iter() {
            if let OpCategory::Moving {
                cmd: MoveCmd::MoveTo,
                end,
            } = &node.category
            {
                last_point = *end;
                break;
            }
        }

        for node in self.commands.iter() {
            if node.is_moving() {
                let linearized = linearize_node(node, last_point);
                for cmd in linearized.commands.iter() {
                    new_cmds.push(cmd.clone());
                    if cmd.is_moving() {
                        last_point = cmd.end_point();
                    }
                }
            } else {
                new_cmds.push(node.clone());
            }
        }

        self.commands = Arc::new(new_cmds);
        self.invalidate_time_cache();
    }

    pub fn linearize_curves(&mut self) {
        let mut new_cmds = Vec::new();
        let mut last_point: Point3D = Point3D::new(0.0, 0.0, 0.0);

        for node in self.commands.iter() {
            if let OpCategory::Moving { end, cmd } = &node.category {
                if matches!(cmd, MoveCmd::MoveTo) {
                    last_point = *end;
                }
                match cmd {
                    MoveCmd::BezierTo { .. }
                    | MoveCmd::QuadraticBezierTo { .. } => {
                        let linearized = linearize_node(node, last_point);
                        for cmd in linearized.commands.iter() {
                            new_cmds.push(cmd.clone());
                            if cmd.is_moving() {
                                last_point = cmd.end_point();
                            }
                        }
                    }
                    _ => {
                        new_cmds.push(node.clone());
                        last_point = *end;
                    }
                }
            } else {
                new_cmds.push(node.clone());
            }
        }

        self.commands = Arc::new(new_cmds);
        self.invalidate_time_cache();
    }

    pub fn linearize_arcs(&mut self) {
        let mut new_cmds = Vec::new();
        let mut last_point: Point3D = Point3D::new(0.0, 0.0, 0.0);

        for node in self.commands.iter() {
            if let OpCategory::Moving { end, cmd } = &node.category {
                if matches!(cmd, MoveCmd::MoveTo) {
                    last_point = *end;
                }
                match cmd {
                    MoveCmd::ArcTo { .. } => {
                        let linearized = linearize_node(node, last_point);
                        for cmd in linearized.commands.iter() {
                            new_cmds.push(cmd.clone());
                            if cmd.is_moving() {
                                last_point = cmd.end_point();
                            }
                        }
                    }
                    _ => {
                        new_cmds.push(node.clone());
                        last_point = *end;
                    }
                }
            } else {
                new_cmds.push(node.clone());
            }
        }

        self.commands = Arc::new(new_cmds);
        self.invalidate_time_cache();
    }
}
