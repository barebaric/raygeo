use super::container::Ops;
use super::types::{MoveCmd, OpCategory, OpNode};
use crate::types::Point3D;

pub fn linearize_node(node: &OpNode, start_point: Point3D) -> Ops {
    let extra = node.extra_axes.as_deref();

    if let OpCategory::Moving { end, cmd } = &node.category {
        let end = *end;
        match cmd {
            MoveCmd::ScanLine { power_values } => {
                let pv = power_values;
                let num_steps = pv.len();
                if num_steps == 0 {
                    return Ops::new();
                }

                let extra_owned = extra.map(|e| e.to_vec());
                let mut result = Ops::new();

                let seg_start_power = pv[0];
                result.set_power(seg_start_power as f64 / 255.0);

                if num_steps > 1 {
                    let line_vec = (
                        end.0 - start_point.0,
                        end.1 - start_point.1,
                        end.2 - start_point.2,
                    );
                    let mut cur_start_power = seg_start_power;
                    for i in 1..num_steps {
                        let cur_power = pv[i];
                        if cur_power != cur_start_power {
                            let t = i as f64 / num_steps as f64;
                            let seg_end = (
                                start_point.0 + t * line_vec.0,
                                start_point.1 + t * line_vec.1,
                                start_point.2 + t * line_vec.2,
                            );
                            result.line_to(
                                seg_end.0,
                                seg_end.1,
                                seg_end.2,
                                extra_owned.clone(),
                            );
                            cur_start_power = cur_power;
                            result.set_power(cur_start_power as f64 / 255.0);
                        }
                    }
                }

                result.line_to(end.0, end.1, end.2, extra_owned);
                result
            }
            MoveCmd::ArcTo { center, cw } => {
                let segments = crate::geo::shape::arc::linearize_arc(
                    end,
                    *center,
                    *cw,
                    start_point,
                    0.1,
                );
                if segments.is_empty() {
                    return Ops::new();
                }

                let extra_owned = extra.map(|e| e.to_vec());
                let mut result = Ops::new();

                for (_, seg_end) in &segments {
                    result.line_to(
                        seg_end.0,
                        seg_end.1,
                        seg_end.2,
                        extra_owned.clone(),
                    );
                }

                result
            }
            MoveCmd::BezierTo { c1, c2 } => {
                let polyline =
                    crate::geo::shape::bezier::linearize_bezier_segment(
                        start_point,
                        *c1,
                        *c2,
                        end,
                        None,
                    );

                let extra_owned = extra.map(|e| e.to_vec());
                let mut result = Ops::new();

                for pt in polyline.iter().skip(1) {
                    result.line_to(pt.0, pt.1, pt.2, extra_owned.clone());
                }

                result
            }
            MoveCmd::QuadraticBezierTo { control } => {
                let c1 = (
                    start_point.0 + (2.0 / 3.0) * (control.0 - start_point.0),
                    start_point.1 + (2.0 / 3.0) * (control.1 - start_point.1),
                    start_point.2 + (2.0 / 3.0) * (control.2 - start_point.2),
                );
                let c2 = (
                    end.0 + (2.0 / 3.0) * (control.0 - end.0),
                    end.1 + (2.0 / 3.0) * (control.1 - end.1),
                    end.2 + (2.0 / 3.0) * (control.2 - end.2),
                );
                let polyline =
                    crate::geo::shape::bezier::linearize_bezier_segment(
                        start_point,
                        c1,
                        c2,
                        end,
                        None,
                    );

                let extra_owned = extra.map(|e| e.to_vec());
                let mut result = Ops::new();

                for pt in polyline.iter().skip(1) {
                    result.line_to(pt.0, pt.1, pt.2, extra_owned.clone());
                }

                result
            }
            MoveCmd::MoveTo | MoveCmd::LineTo => {
                let extra_owned = extra.map(|e| e.to_vec());
                let mut result = Ops::new();
                if matches!(cmd, MoveCmd::MoveTo) {
                    result.move_to(end.0, end.1, end.2, extra_owned);
                } else {
                    result.line_to(end.0, end.1, end.2, extra_owned);
                }
                result
            }
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
        let mut last_point: Point3D = (0.0, 0.0, 0.0);

        for node in &self.commands {
            if let OpCategory::Moving {
                cmd: MoveCmd::MoveTo,
                end,
            } = &node.category
            {
                last_point = *end;
                break;
            }
        }

        for node in &self.commands {
            if node.is_moving() {
                let linearized = linearize_node(node, last_point);
                for cmd in &linearized.commands {
                    new_cmds.push(cmd.clone());
                    if cmd.is_moving() {
                        last_point = cmd.end_point();
                    }
                }
            } else {
                new_cmds.push(node.clone());
            }
        }

        self.commands = new_cmds;
        self.invalidate_time_cache();
    }

    pub fn linearize_curves(&mut self) {
        let mut new_cmds = Vec::new();
        let mut last_point: Point3D = (0.0, 0.0, 0.0);

        for node in &self.commands {
            if let OpCategory::Moving { end, cmd } = &node.category {
                if matches!(cmd, MoveCmd::MoveTo) {
                    last_point = *end;
                }
                match cmd {
                    MoveCmd::BezierTo { .. }
                    | MoveCmd::QuadraticBezierTo { .. } => {
                        let linearized = linearize_node(node, last_point);
                        for cmd in &linearized.commands {
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

        self.commands = new_cmds;
        self.invalidate_time_cache();
    }

    pub fn linearize_arcs(&mut self) {
        let mut new_cmds = Vec::new();
        let mut last_point: Point3D = (0.0, 0.0, 0.0);

        for node in &self.commands {
            if let OpCategory::Moving { end, cmd } = &node.category {
                if matches!(cmd, MoveCmd::MoveTo) {
                    last_point = *end;
                }
                match cmd {
                    MoveCmd::ArcTo { .. } => {
                        let linearized = linearize_node(node, last_point);
                        for cmd in &linearized.commands {
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

        self.commands = new_cmds;
        self.invalidate_time_cache();
    }
}
