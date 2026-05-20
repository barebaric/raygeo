use super::axis::Axis;
use super::container::Ops;
use super::enums::{CommandCategory, CommandType};
use super::types::{OpCommand, OpMetadata};
use crate::types::Point3D;

fn linearize_scanline(
    commands: &[OpCommand],
    scanline_idx: usize,
    start_point: Point3D,
    end: Point3D,
    extra: Option<&[(Axis, f64)]>,
) -> Option<Ops> {
    let pv = match &commands[scanline_idx].metadata {
        OpMetadata::ScanLine(data) => data,
        _ => return None,
    };
    let num_steps = pv.len();
    if num_steps == 0 {
        return None;
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
    Some(result)
}

fn linearize_arc(
    commands: &[OpCommand],
    arc_idx: usize,
    start_point: Point3D,
    end: Point3D,
    extra: Option<&[(Axis, f64)]>,
) -> Option<Ops> {
    let (ci, cj, cw) = match &commands[arc_idx].metadata {
        OpMetadata::Arc(a) => *a,
        _ => return None,
    };
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
    let segments =
        crate::geo::shape::arc::linearize_arc(&arc_row, start_point, 0.1);
    if segments.is_empty() {
        return None;
    }

    let extra_owned = extra.map(|e| e.to_vec());
    let mut result = Ops::new();

    for (_, seg_end) in &segments {
        result.line_to(seg_end.0, seg_end.1, seg_end.2, extra_owned.clone());
    }

    Some(result)
}

fn linearize_bezier(
    commands: &[OpCommand],
    bezier_idx: usize,
    start_point: Point3D,
    end: Point3D,
    extra: Option<&[(Axis, f64)]>,
) -> Option<Ops> {
    let (c1, c2) = match &commands[bezier_idx].metadata {
        OpMetadata::Bezier(b) => *b,
        _ => return None,
    };
    let polyline = crate::geo::shape::bezier::linearize_bezier_segment(
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

    Some(result)
}

impl Ops {
    pub fn linearize(&self, idx: usize, start_point: Point3D) -> Ops {
        let ct = self.command_type(idx);
        let end = self.endpoint(idx);
        let extra = self.extra_axes(idx);

        match ct {
            CommandType::ScanLine => {
                linearize_scanline(&self.commands, idx, start_point, end, extra)
                    .unwrap_or_else(Ops::new)
            }
            CommandType::ArcTo => {
                linearize_arc(&self.commands, idx, start_point, end, extra)
                    .unwrap_or_else(Ops::new)
            }
            CommandType::BezierTo | CommandType::QuadraticBezierTo => {
                linearize_bezier(&self.commands, idx, start_point, end, extra)
                    .unwrap_or_else(Ops::new)
            }
            CommandType::MoveTo | CommandType::LineTo => {
                let extra_owned = extra.map(|e| e.to_vec());
                let mut result = Ops::new();
                if ct == CommandType::MoveTo {
                    result.move_to(end.0, end.1, end.2, extra_owned);
                } else {
                    result.line_to(end.0, end.1, end.2, extra_owned);
                }
                result
            }
            _ => Ops::new(),
        }
    }

    pub fn linearize_all(&mut self) {
        let mut new_cmds = Vec::new();
        let mut last_point: Point3D = (0.0, 0.0, 0.0);

        for i in 0..self.len() {
            if self.command_type(i) == CommandType::MoveTo {
                last_point = self.endpoint(i);
                break;
            }
        }

        for i in 0..self.len() {
            if self.command_type(i).category() == CommandCategory::Moving {
                let linearized = self.linearize(i, last_point);
                for cmd in &linearized.commands {
                    new_cmds.push(cmd.clone());
                    if cmd.ct.category() == CommandCategory::Moving {
                        last_point = cmd.end;
                    }
                }
            } else {
                new_cmds.push(self.commands[i].clone());
            }
        }

        self.commands = new_cmds;
        self.invalidate_time_cache();
    }

    pub fn linearize_curves(&mut self) {
        let mut new_cmds = Vec::new();
        let mut last_point: Point3D = (0.0, 0.0, 0.0);

        for i in 0..self.len() {
            let ct = self.command_type(i);
            if ct == CommandType::MoveTo {
                last_point = self.endpoint(i);
            }
            if ct == CommandType::BezierTo
                || ct == CommandType::QuadraticBezierTo
            {
                let linearized = self.linearize(i, last_point);
                for cmd in &linearized.commands {
                    new_cmds.push(cmd.clone());
                    if cmd.ct.category() == CommandCategory::Moving {
                        last_point = cmd.end;
                    }
                }
            } else {
                new_cmds.push(self.commands[i].clone());
                if self.command_type(i).category() == CommandCategory::Moving
                {
                    last_point = self.endpoint(i);
                }
            }
        }

        self.commands = new_cmds;
        self.invalidate_time_cache();
    }

    pub fn linearize_arcs(&mut self) {
        let mut new_cmds = Vec::new();
        let mut last_point: Point3D = (0.0, 0.0, 0.0);

        for i in 0..self.len() {
            let ct = self.command_type(i);
            if ct == CommandType::MoveTo {
                last_point = self.endpoint(i);
            }
            if ct == CommandType::ArcTo {
                let linearized = self.linearize(i, last_point);
                for cmd in &linearized.commands {
                    new_cmds.push(cmd.clone());
                    if cmd.ct.category() == CommandCategory::Moving {
                        last_point = cmd.end;
                    }
                }
            } else {
                new_cmds.push(self.commands[i].clone());
                if self.command_type(i).category() == CommandCategory::Moving
                {
                    last_point = self.endpoint(i);
                }
            }
        }

        self.commands = new_cmds;
        self.invalidate_time_cache();
    }
}
