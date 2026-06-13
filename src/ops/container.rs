use std::fmt::Write;

use crate::constants::EPSILON_COLLINEAR;
use crate::geo::shape::arc::get_arc_length;
use crate::geo::shape::bezier::get_bezier_length;
use crate::geo::shape::line::get_line_segment_length;

use super::axis::Axis;
use super::enums::{CommandCategory, CommandType, SectionType};
use super::state::State;
use super::types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};
use crate::types::{Point3D, Rect};

#[derive(Clone, Debug)]
pub struct Ops {
    pub commands: Vec<OpNode>,
    pub last_move_to: Point3D,
    pub time_dirty: bool,
    pub cached_time: f64,
    pub time_params: Option<(f64, f64, f64)>,
}

impl Ops {
    pub fn new() -> Self {
        Ops {
            commands: Vec::new(),
            last_move_to: (0.0, 0.0, 0.0),
            time_dirty: true,
            cached_time: 0.0,
            time_params: None,
        }
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn command_type(&self, idx: usize) -> CommandType {
        self.commands[idx].command_type()
    }

    pub fn category(&self, idx: usize) -> CommandCategory {
        self.commands[idx].command_type().category()
    }

    pub fn is_travel(&self, idx: usize) -> bool {
        self.commands[idx].command_type() == CommandType::MoveTo
    }

    pub fn is_cutting(&self, idx: usize) -> bool {
        let ct = self.commands[idx].command_type();
        ct.category() == CommandCategory::Moving && ct != CommandType::MoveTo
    }

    pub fn is_state(&self, idx: usize) -> bool {
        self.commands[idx].is_state_cmd()
    }

    pub fn is_marker(&self, idx: usize) -> bool {
        self.commands[idx].is_marker()
    }

    pub fn is_scanline(&self, idx: usize) -> bool {
        self.commands[idx].command_type() == CommandType::ScanLine
    }

    pub fn endpoint(&self, idx: usize) -> Point3D {
        self.commands[idx].end_point()
    }

    pub fn scanline_data(&self, idx: usize) -> Vec<u8> {
        if let OpCategory::Moving {
            cmd: MoveCmd::ScanLine { power_values },
            ..
        } = &self.commands[idx].category
        {
            power_values.to_vec()
        } else {
            Vec::new()
        }
    }

    pub fn workpiece_uid(&self, idx: usize) -> &str {
        if let OpCategory::Marker(MarkerCmd::WorkpieceStart(uid)) =
            &self.commands[idx].category
        {
            uid.as_ref()
        } else if let OpCategory::Marker(MarkerCmd::WorkpieceEnd(uid)) =
            &self.commands[idx].category
        {
            uid.as_ref()
        } else {
            ""
        }
    }

    pub fn set_endpoint(
        &mut self,
        idx: usize,
        end: Point3D,
    ) -> Option<Point3D> {
        self.commands[idx].set_endpoint(end)
    }

    pub fn extra_axes(&self, idx: usize) -> Option<&[(Axis, f64)]> {
        self.commands[idx].extra_axes()
    }

    pub fn set_extra_axes(&mut self, idx: usize, ea: Vec<(Axis, f64)>) {
        self.commands[idx].set_extra_axes(std::sync::Arc::from(ea));
    }

    pub fn state(&self, idx: usize) -> Option<&State> {
        self.commands[idx].state()
    }

    pub fn preloaded_state(&self, idx: usize) -> Option<&State> {
        self.commands[idx].state()
    }

    pub fn set_state_on_moving(&mut self, state: &State) {
        for node in &mut self.commands {
            if node.is_moving() {
                node.set_state(state.clone());
            }
        }
    }

    pub fn set_state_at(&mut self, idx: usize, state: &State) {
        self.commands[idx].set_state(state.clone());
    }

    pub fn distance_at(&self, idx: usize, last_point: Option<Point3D>) -> f64 {
        if let OpCategory::Moving { end, .. } = &self.commands[idx].category {
            match last_point {
                None => 0.0,
                Some(lp) => {
                    let dx = end.0 - lp.0;
                    let dy = end.1 - lp.1;
                    (dx * dx + dy * dy).sqrt()
                }
            }
        } else {
            0.0
        }
    }

    pub fn distance(&self) -> f64 {
        let mut total = 0.0;
        let mut last: Option<Point3D> = None;
        for node in &self.commands {
            if let OpCategory::Moving { end, .. } = &node.category {
                if let Some(lp) = last {
                    let dx = end.0 - lp.0;
                    let dy = end.1 - lp.1;
                    total += (dx * dx + dy * dy).sqrt();
                }
                last = Some(*end);
            }
        }
        total
    }

    pub fn cut_distance(&self) -> f64 {
        let mut total = 0.0;
        let mut last: Option<Point3D> = None;
        for node in &self.commands {
            if let OpCategory::Moving { end, cmd } = &node.category {
                if let Some(lp) = last {
                    if !matches!(cmd, MoveCmd::MoveTo) {
                        let dx = end.0 - lp.0;
                        let dy = end.1 - lp.1;
                        total += (dx * dx + dy * dy).sqrt();
                    }
                }
                last = Some(*end);
            }
        }
        total
    }

    // --- Builder methods ---

    pub fn move_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.last_move_to = (x, y, z);
        self.commands.push(OpNode::move_to(x, y, z, extra));
        self.invalidate_time_cache();
    }

    pub fn line_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.commands.push(OpNode::line_to(x, y, z, extra));
        self.invalidate_time_cache();
    }

    pub fn close_path(&mut self) {
        self.line_to(
            self.last_move_to.0,
            self.last_move_to.1,
            self.last_move_to.2,
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn arc_to(
        &mut self,
        x: f64,
        y: f64,
        i: f64,
        j: f64,
        clockwise: bool,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.commands
            .push(OpNode::arc_to(x, y, i, j, clockwise, z, extra));
        self.invalidate_time_cache();
    }

    pub fn bezier_to(
        &mut self,
        control1: Point3D,
        control2: Point3D,
        end: Point3D,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.commands
            .push(OpNode::bezier_to(control1, control2, end, extra));
        self.invalidate_time_cache();
    }

    pub fn quadratic_bezier_to(
        &mut self,
        control: Point3D,
        end: Point3D,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.commands
            .push(OpNode::quadratic_bezier_to(control, end, extra));
        self.invalidate_time_cache();
    }

    pub fn scan_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        power_values: Vec<u8>,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.commands
            .push(OpNode::scan_to(x, y, z, power_values, extra));
        self.invalidate_time_cache();
    }

    pub fn set_power(&mut self, power: f64) {
        self.commands.push(OpNode::set_power(power));
    }

    pub fn set_cut_speed(&mut self, speed: i32) {
        self.commands.push(OpNode::set_cut_speed(speed));
        self.invalidate_time_cache();
    }

    pub fn set_travel_speed(&mut self, speed: i32) {
        self.commands.push(OpNode::set_travel_speed(speed));
        self.invalidate_time_cache();
    }

    pub fn dwell(&mut self, duration_ms: f64) {
        self.commands.push(OpNode::dwell(duration_ms));
        self.invalidate_time_cache();
    }

    pub fn enable_air_assist(&mut self, enabled: bool) {
        self.commands.push(OpNode::enable_air_assist(enabled));
        self.invalidate_time_cache();
    }

    pub fn set_laser(&mut self, laser_uid: &str) {
        self.commands.push(OpNode::set_laser(laser_uid));
        self.invalidate_time_cache();
    }

    pub fn set_frequency(&mut self, frequency: i32) {
        self.commands.push(OpNode::set_frequency(frequency));
        self.invalidate_time_cache();
    }

    pub fn set_pulse_width(&mut self, pulse_width: f64) {
        self.commands.push(OpNode::set_pulse_width(pulse_width));
        self.invalidate_time_cache();
    }

    pub fn job_start(&mut self) {
        self.commands.push(OpNode::job_start());
        self.invalidate_time_cache();
    }

    pub fn job_end(&mut self) {
        self.commands.push(OpNode::job_end());
        self.invalidate_time_cache();
    }

    pub fn layer_start(&mut self, layer_uid: &str) {
        self.commands.push(OpNode::layer_start(layer_uid));
        self.invalidate_time_cache();
    }

    pub fn layer_end(&mut self, layer_uid: &str) {
        self.commands.push(OpNode::layer_end(layer_uid));
        self.invalidate_time_cache();
    }

    pub fn workpiece_start(&mut self, workpiece_uid: &str) {
        self.commands.push(OpNode::workpiece_start(workpiece_uid));
        self.invalidate_time_cache();
    }

    pub fn workpiece_end(&mut self, workpiece_uid: &str) {
        self.commands.push(OpNode::workpiece_end(workpiece_uid));
        self.invalidate_time_cache();
    }

    pub fn ops_section_start(
        &mut self,
        section_type: SectionType,
        workpiece_uid: &str,
    ) {
        self.commands
            .push(OpNode::ops_section_start(section_type, workpiece_uid));
        self.invalidate_time_cache();
    }

    pub fn ops_section_end(&mut self, section_type: SectionType) {
        self.commands.push(OpNode::ops_section_end(section_type));
        self.invalidate_time_cache();
    }

    // --- Copy / Transfer ---

    pub fn copy(&self) -> Self {
        let mut new_ops = Ops::new();
        for cmd in &self.commands {
            new_ops.commands.push(cmd.clone());
        }
        new_ops.last_move_to = self.last_move_to;
        new_ops.time_dirty = self.time_dirty;
        new_ops.cached_time = self.cached_time;
        new_ops.time_params = self.time_params;
        new_ops
    }

    pub fn copy_command_from(&mut self, source: &Ops, idx: usize) {
        let cmd = source.commands[idx].clone();
        self.commands.push(cmd);
        self.invalidate_time_cache();
    }

    pub fn transfer_command_from(&mut self, source: &Ops, idx: usize) {
        let cmd = source.commands[idx].clone();
        self.commands.push(cmd);
        self.invalidate_time_cache();
    }

    pub fn extend(&mut self, other: &Ops) {
        if !other.is_empty() {
            for cmd in &other.commands {
                self.commands.push(cmd.clone());
            }
            self.invalidate_time_cache();
        }
    }

    pub fn sub_ops(&self, indices: &[usize]) -> Self {
        let mut result = Ops::new();
        for &i in indices {
            let cmd = self.commands[i].clone();
            result.commands.push(cmd);
        }
        result.invalidate_time_cache();
        result
    }

    pub fn replace_all(&mut self, source: &Ops) {
        self.commands.clear();
        for cmd in &source.commands {
            self.commands.push(cmd.clone());
        }
        self.invalidate_time_cache();
    }

    pub fn replace_with(&mut self, source: &Ops) {
        self.commands.clear();
        for cmd in &source.commands {
            self.commands.push(cmd.clone());
        }
        self.last_move_to = source.last_move_to;
        self.invalidate_time_cache();
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.invalidate_time_cache();
    }

    pub fn subpath_indices(&self) -> Vec<Vec<usize>> {
        let mut subpaths: Vec<Vec<usize>> = Vec::new();
        let mut current: Vec<usize> = Vec::new();
        let mut has_move_to = false;
        for (i, node) in self.commands.iter().enumerate() {
            let is_move = matches!(
                node.category,
                OpCategory::Moving {
                    cmd: MoveCmd::MoveTo,
                    ..
                }
            );
            if is_move && has_move_to {
                subpaths.push(current);
                current = Vec::new();
            }
            if is_move {
                has_move_to = true;
            }
            current.push(i);
        }
        if !current.is_empty() {
            subpaths.push(current);
        }
        subpaths
    }

    pub fn split_into_subpaths(&self) -> Vec<Ops> {
        super::group::split_into_subpaths(self)
    }

    pub fn iter_sections(&self) -> Vec<super::group::OpsSection> {
        super::group::iter_sections(self)
    }

    pub fn iter_section_ranges(&self) -> Vec<super::group::OpsSectionRange> {
        super::group::iter_section_ranges(self)
    }

    pub fn flip_ops(&self) -> Self {
        super::flip::flip_ops(self)
    }

    pub fn state_at(&self, idx: usize) -> State {
        let mut state = State::default();
        for node in &self.commands[..=idx] {
            if let OpCategory::State(_) = node.category {
                Self::apply_state_at(&mut state, node);
            }
        }
        state
    }

    // --- State ---

    pub fn preload_state(&mut self) {
        let mut state = State::default();
        for node in &mut self.commands {
            if let OpCategory::State(_) = node.category {
                Self::apply_state_at(&mut state, node);
            } else if node.is_moving() {
                node.set_state(state.clone());
            }
        }
    }

    fn apply_state_at(state: &mut State, node: &OpNode) {
        if let OpCategory::State(cmd) = &node.category {
            match cmd {
                StateCmd::SetPower(p) => state.power = *p,
                StateCmd::SetCutSpeed(s) => state.cut_speed = Some(*s),
                StateCmd::SetTravelSpeed(s) => state.travel_speed = Some(*s),
                StateCmd::EnableAirAssist => state.air_assist = true,
                StateCmd::DisableAirAssist => state.air_assist = false,
                StateCmd::SetLaser(uid) => {
                    state.active_laser_uid = Some(uid.to_string())
                }
                StateCmd::SetFrequency(f) => state.frequency = Some(*f),
                StateCmd::SetPulseWidth(pw) => state.pulse_width = Some(*pw),
                StateCmd::Dwell(d) => state.dwell_ms = Some(*d),
            }
        }
    }

    // --- Arithmetic ---

    pub fn ops_add(&self, other: &Ops) -> Self {
        let mut result = Ops::new();
        for cmd in &self.commands {
            result.commands.push(cmd.clone());
        }
        for cmd in &other.commands {
            result.commands.push(cmd.clone());
        }
        result
    }

    pub fn ops_mul(&self, count: usize) -> Self {
        let mut result = Ops::new();
        for _ in 0..count {
            for cmd in &self.commands {
                result.commands.push(cmd.clone());
            }
        }
        result
    }

    // --- Utility ---

    pub fn dump(&self) {
        print!("{}", self.format_dump());
    }

    pub fn format_dump(&self) -> String {
        let mut out = format!("Ops {{ len: {} }}\n", self.commands.len());
        for (i, node) in self.commands.iter().enumerate() {
            let ct = node.command_type();
            write!(out, "  [{}] {}", i, ct).unwrap();
            if let OpCategory::Moving { end, cmd } = &node.category {
                write!(out, " end=({:.3},{:.3},{:.3})", end.0, end.1, end.2)
                    .unwrap();
                match cmd {
                    MoveCmd::ArcTo { center, cw } => {
                        write!(
                            out,
                            " arc=(i={:.3},j={:.3},cw={})",
                            center.0, center.1, cw
                        )
                        .unwrap();
                    }
                    MoveCmd::BezierTo { control1, control2 } => {
                        write!(
                            out,
                            " bezier=(control1=({:.3},{:.3}),control2=({:.3},{:.3}))",
                            control1.0, control1.1, control2.0, control2.1
                        )
                        .unwrap();
                    }
                    _ => {}
                }
            }
            writeln!(out).unwrap();
        }
        out
    }

    pub fn invalidate_time_cache(&mut self) {
        self.time_dirty = true;
    }

    pub fn estimate_time(
        &mut self,
        default_cut_speed: f64,
        default_travel_speed: f64,
        acceleration: f64,
    ) -> f64 {
        if self.commands.is_empty() {
            return 0.0;
        }
        let params = (default_cut_speed, default_travel_speed, acceleration);
        if !self.time_dirty && self.time_params == Some(params) {
            return self.cached_time;
        }
        let total = estimate_time_core(
            self,
            default_cut_speed,
            default_travel_speed,
            acceleration,
        );
        self.cached_time = total;
        self.time_dirty = false;
        self.time_params = Some(params);
        total
    }

    pub fn estimate_command_times(
        &self,
        default_cut_speed: f64,
        default_travel_speed: f64,
        acceleration: f64,
    ) -> Vec<f64> {
        let mut times = Vec::with_capacity(self.commands.len());
        let mut last_point = (0.0, 0.0, 0.0);
        let mut cut_speed = default_cut_speed;
        let mut travel_speed = default_travel_speed;

        for node in &self.commands {
            let cmd_time = match &node.category {
                OpCategory::State(StateCmd::SetCutSpeed(s)) => {
                    cut_speed = *s as f64;
                    0.0
                }
                OpCategory::State(StateCmd::SetTravelSpeed(s)) => {
                    travel_speed = *s as f64;
                    0.0
                }
                OpCategory::Moving { end, cmd } => {
                    let distance = move_distance(cmd, last_point, *end);

                    if distance < EPSILON_COLLINEAR {
                        last_point = *end;
                        0.0
                    } else {
                        let speed = if matches!(cmd, MoveCmd::MoveTo) {
                            travel_speed
                        } else {
                            cut_speed
                        };

                        let speed_mm_per_sec = speed / 60.0;
                        let move_time = if acceleration > 0.0 {
                            let accel_time = speed_mm_per_sec / acceleration;
                            let accel_distance =
                                0.5 * acceleration * accel_time * accel_time;
                            if distance < 2.0 * accel_distance {
                                2.0 * (distance / acceleration).sqrt()
                            } else {
                                let cruise_distance =
                                    distance - 2.0 * accel_distance;
                                let cruise_time =
                                    cruise_distance / speed_mm_per_sec;
                                2.0 * accel_time + cruise_time
                            }
                        } else {
                            distance / speed_mm_per_sec
                        };

                        last_point = *end;
                        move_time
                    }
                }
                _ => 0.0,
            };
            times.push(cmd_time);
        }
        times
    }

    pub fn segment_indices(&self) -> Vec<Vec<usize>> {
        super::group::segment_indices(self)
    }

    pub fn get_frame(&self, power: Option<f64>, speed: Option<f64>) -> Self {
        let Some(Rect(min_x, min_y, max_x, max_y)) = self.rect(false) else {
            return Ops::new();
        };
        let mut frame_ops = Ops::new();
        if let Some(p) = power {
            frame_ops.set_power(p);
        }
        if let Some(s) = speed {
            frame_ops.set_cut_speed(s as i32);
        }
        frame_ops.move_to(min_x, min_y, 0.0, None);
        frame_ops.line_to(min_x, max_y, 0.0, None);
        frame_ops.line_to(max_x, max_y, 0.0, None);
        frame_ops.line_to(max_x, min_y, 0.0, None);
        frame_ops.line_to(min_x, min_y, 0.0, None);
        frame_ops
    }

    pub fn rect(&self, include_travel: bool) -> Option<Rect> {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_content = false;

        let mut curr_x = 0.0;
        let mut curr_y = 0.0;
        if include_travel {
            curr_x = self.last_move_to.0;
            curr_y = self.last_move_to.1;
        }

        for node in &self.commands {
            if let OpCategory::Moving { end, cmd } = &node.category {
                let (end_x, end_y) = (end.0, end.1);

                if matches!(cmd, MoveCmd::MoveTo) {
                    if include_travel {
                        Self::update_bounds(
                            &mut min_x, &mut min_y, &mut max_x, &mut max_y,
                            curr_x, curr_y,
                        );
                        Self::update_bounds(
                            &mut min_x, &mut min_y, &mut max_x, &mut max_y,
                            end_x, end_y,
                        );
                        has_content = true;
                    }
                    curr_x = end_x;
                    curr_y = end_y;
                    continue;
                }

                Self::update_bounds(
                    &mut min_x, &mut min_y, &mut max_x, &mut max_y, curr_x,
                    curr_y,
                );
                Self::update_bounds(
                    &mut min_x, &mut min_y, &mut max_x, &mut max_y, end_x,
                    end_y,
                );
                has_content = true;

                if let MoveCmd::ArcTo { center, cw } = cmd {
                    let radius =
                        (center.0 * center.0 + center.1 * center.1).sqrt();
                    if (curr_x - end_x).abs() < EPSILON_COLLINEAR
                        && (curr_y - end_y).abs() < EPSILON_COLLINEAR
                        && radius > EPSILON_COLLINEAR
                    {
                        let cx = curr_x + center.0;
                        let cy = curr_y + center.1;
                        Self::update_bounds(
                            &mut min_x,
                            &mut min_y,
                            &mut max_x,
                            &mut max_y,
                            cx - radius,
                            cy - radius,
                        );
                        Self::update_bounds(
                            &mut min_x,
                            &mut min_y,
                            &mut max_x,
                            &mut max_y,
                            cx + radius,
                            cy + radius,
                        );
                    } else {
                        let abox = crate::geo::shape::arc::get_arc_bounds(
                            (curr_x, curr_y),
                            (end_x, end_y),
                            (center.0, center.1),
                            *cw,
                        );
                        Self::update_bounds(
                            &mut min_x, &mut min_y, &mut max_x, &mut max_y,
                            abox.0, abox.1,
                        );
                        Self::update_bounds(
                            &mut min_x, &mut min_y, &mut max_x, &mut max_y,
                            abox.2, abox.3,
                        );
                    }
                }

                curr_x = end_x;
                curr_y = end_y;
            }
        }

        if has_content {
            Some(Rect(min_x, min_y, max_x, max_y))
        } else {
            None
        }
    }

    fn update_bounds(
        min_x: &mut f64,
        min_y: &mut f64,
        max_x: &mut f64,
        max_y: &mut f64,
        x: f64,
        y: f64,
    ) {
        if x < *min_x {
            *min_x = x;
        }
        if x > *max_x {
            *max_x = x;
        }
        if y < *min_y {
            *min_y = y;
        }
        if y > *max_y {
            *max_y = y;
        }
    }

    pub fn without_state(&self) -> Self {
        super::group::without_state(self)
    }

    pub fn group_by_state_continuity(&self) -> Vec<Ops> {
        super::group::group_by_state_continuity(self)
    }

    pub fn from_geometry(
        geometry: &crate::geo::geometry::Geometry,
    ) -> Result<Self, crate::RaygeoError> {
        let mut ops = Ops::new();
        if geometry.data.is_empty() {
            ops.last_move_to = geometry.last_move_to;
            return Ok(ops);
        }

        for cmd in &geometry.data {
            match cmd {
                crate::Command::Move { end } => {
                    ops.move_to(end.0, end.1, end.2, None);
                }
                crate::Command::Line { end } => {
                    ops.line_to(end.0, end.1, end.2, None);
                }
                crate::Command::Arc {
                    end,
                    center_offset,
                    clockwise,
                } => {
                    ops.arc_to(
                        end.0,
                        end.1,
                        center_offset.0,
                        center_offset.1,
                        *clockwise,
                        end.2,
                        None,
                    );
                }
                crate::Command::Bezier {
                    end,
                    control1,
                    control2,
                } => {
                    ops.bezier_to(*control1, *control2, *end, None);
                }
            }
        }
        ops.last_move_to = geometry.last_move_to;
        Ok(ops)
    }

    pub fn to_geometry(&self) -> crate::geo::geometry::Geometry {
        let mut geo = crate::geo::geometry::Geometry::new();
        for node in &self.commands {
            if let OpCategory::Moving { end, cmd } = &node.category {
                match cmd {
                    MoveCmd::MoveTo => {
                        geo.move_to(end.0, end.1, end.2);
                    }
                    MoveCmd::LineTo => {
                        geo.line_to(end.0, end.1, end.2);
                    }
                    MoveCmd::ArcTo { center, cw } => {
                        geo.arc_to(
                            end.0, end.1, center.0, center.1, *cw, end.2,
                        );
                    }
                    MoveCmd::BezierTo { control1, control2 } => {
                        geo.bezier_to(*control1, *control2, *end);
                    }
                    _ => {}
                }
            }
        }
        geo
    }
}

fn move_distance(cmd: &MoveCmd, last_point: Point3D, end: Point3D) -> f64 {
    match cmd {
        MoveCmd::ArcTo { center, cw } => get_arc_length(
            (last_point.0, last_point.1),
            (end.0, end.1),
            *center,
            *cw,
        ),
        MoveCmd::BezierTo { control1, control2 } => get_bezier_length(
            (last_point.0, last_point.1),
            (control1.0, control1.1),
            (control2.0, control2.1),
            (end.0, end.1),
        ),
        MoveCmd::QuadraticBezierTo { control } => {
            let c = *control;
            get_bezier_length(
                (last_point.0, last_point.1),
                (
                    (last_point.0 + 2.0 * c.0) / 3.0,
                    (last_point.1 + 2.0 * c.1) / 3.0,
                ),
                ((end.0 + 2.0 * c.0) / 3.0, (end.1 + 2.0 * c.1) / 3.0),
                (end.0, end.1),
            )
        }
        _ => get_line_segment_length(
            (last_point.0, last_point.1),
            (end.0, end.1),
        ),
    }
}

fn estimate_time_core(
    ops: &Ops,
    default_cut_speed: f64,
    default_travel_speed: f64,
    acceleration: f64,
) -> f64 {
    ops.estimate_command_times(
        default_cut_speed,
        default_travel_speed,
        acceleration,
    )
    .into_iter()
    .sum()
}

impl Default for Ops {
    fn default() -> Self {
        Self::new()
    }
}
