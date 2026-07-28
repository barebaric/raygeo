pub(crate) mod structure;
pub(crate) mod time;

use crate::constants::EPSILON_COLLINEAR;
use crate::error::RaygeoError;

use super::axis::Axis;
use super::enums::{CommandCategory, CommandType, RasterMode, SectionType};
use super::state::{AirAssistMode, CoolantMode, HeadCoolantMode, State};
use super::types::{MarkerCmd, MoveCmd, OpCategory, OpNode, StateCmd};
use crate::types::{Point, Point3D, Rect};

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
            last_move_to: Point3D::new(0.0, 0.0, 0.0),
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
                    let dx = end.x - lp.x;
                    let dy = end.y - lp.y;
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
                    let dx = end.x - lp.x;
                    let dy = end.y - lp.y;
                    total += (dx * dx + dy * dy).sqrt();
                }
                last = Some(*end);
            }
        }
        total
    }

    /// Estimated heap-allocated bytes for this Ops instance
    /// (commands Vec buffer + scanline power data).
    pub fn heap_size(&self) -> usize {
        let commands_buf = self.commands.len() * std::mem::size_of::<OpNode>();
        let scanline_data: usize = self
            .commands
            .iter()
            .filter_map(|node| {
                if let OpCategory::Moving {
                    cmd: MoveCmd::ScanLine { power_values },
                    ..
                } = &node.category
                {
                    Some(power_values.len())
                } else {
                    None
                }
            })
            .sum();
        commands_buf + scanline_data
    }

    pub fn cut_distance(&self) -> f64 {
        let mut total = 0.0;
        let mut last: Option<Point3D> = None;
        for node in &self.commands {
            if let OpCategory::Moving { end, cmd } = &node.category {
                if let Some(lp) = last {
                    if !matches!(cmd, MoveCmd::MoveTo) {
                        let dx = end.x - lp.x;
                        let dy = end.y - lp.y;
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
        self.last_move_to = Point3D::new(x, y, z);
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
            self.last_move_to.x,
            self.last_move_to.y,
            self.last_move_to.z,
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

    pub fn set_feed_rate(&mut self, feed_rate: i32) {
        self.commands.push(OpNode::set_feed_rate(feed_rate));
        self.invalidate_time_cache();
    }

    pub fn set_rapid_rate(&mut self, rapid_rate: i32) {
        self.commands.push(OpNode::set_rapid_rate(rapid_rate));
        self.invalidate_time_cache();
    }

    pub fn dwell(&mut self, duration_ms: f64) {
        self.commands.push(OpNode::dwell(duration_ms));
        self.invalidate_time_cache();
    }

    pub fn set_head(&mut self, head_uid: &str) {
        self.commands.push(OpNode::set_head(head_uid));
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

    pub fn set_spindle_rpm(&mut self, rpm: u32) {
        self.commands.push(OpNode::set_spindle_rpm(rpm));
        self.invalidate_time_cache();
    }

    pub fn set_coolant(&mut self, mode: CoolantMode) {
        self.commands.push(OpNode::set_coolant(mode));
        self.invalidate_time_cache();
    }

    pub fn set_air_assist(&mut self, mode: AirAssistMode) {
        self.commands.push(OpNode::set_air_assist(mode));
        self.invalidate_time_cache();
    }

    pub fn set_head_coolant(&mut self, mode: HeadCoolantMode) {
        self.commands.push(OpNode::set_head_coolant(mode));
        self.invalidate_time_cache();
    }

    /// Emit the state commands needed to reach *state*.
    ///
    /// Power is always emitted (default 0.0). All other fields are
    /// emitted only when ``Some``. Domain-neutral: does not decide
    /// what values to use, just emits them. The caller (cnc layer)
    /// computes the ``State``.
    pub fn apply_state(&mut self, state: &State) {
        self.set_power(state.power);
        if let Some(fr) = state.feed_rate {
            self.set_feed_rate(fr);
        }
        if let Some(rr) = state.rapid_rate {
            self.set_rapid_rate(rr);
        }
        if let Some(rpm) = state.spindle_rpm {
            self.set_spindle_rpm(rpm);
        }
        if let Some(c) = state.coolant {
            self.set_coolant(c);
        }
        if let Some(a) = state.air_assist {
            self.set_air_assist(a);
        }
        if let Some(h) = state.head_coolant {
            self.set_head_coolant(h);
        }
        if let Some(f) = state.frequency {
            self.set_frequency(f);
        }
        if let Some(pw) = state.pulse_width {
            self.set_pulse_width(pw);
        }
        if let Some(ref h) = state.active_head_uid {
            self.set_head(h);
        }
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
        raster_mode: Option<RasterMode>,
    ) -> Result<(), RaygeoError> {
        self.commands.push(OpNode::ops_section_start(
            section_type,
            workpiece_uid,
            raster_mode,
        )?);
        self.invalidate_time_cache();
        Ok(())
    }

    pub fn ops_section_end(
        &mut self,
        section_type: SectionType,
        raster_mode: Option<RasterMode>,
    ) -> Result<(), RaygeoError> {
        self.commands
            .push(OpNode::ops_section_end(section_type, raster_mode)?);
        self.invalidate_time_cache();
        Ok(())
    }

    pub fn state_block_start(&mut self, name: Option<&str>) {
        self.commands.push(OpNode::state_block_start(name));
        self.invalidate_time_cache();
    }

    pub fn state_block_end(&mut self) {
        self.commands.push(OpNode::state_block_end());
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

    pub fn flip_ops(&self) -> Self {
        super::transform::flip::flip_ops(self)
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
                StateCmd::SetFeedRate(s) => state.feed_rate = Some(*s),
                StateCmd::SetRapidRate(s) => state.rapid_rate = Some(*s),
                StateCmd::SetHead(uid) => {
                    state.active_head_uid = Some(uid.to_string())
                }
                StateCmd::SetFrequency(f) => state.frequency = Some(*f),
                StateCmd::SetPulseWidth(pw) => state.pulse_width = Some(*pw),
                StateCmd::SetSpindleRpm(s) => state.spindle_rpm = Some(*s),
                StateCmd::SetCoolant(mode) => state.coolant = Some(*mode),
                StateCmd::SetAirAssist(mode) => state.air_assist = Some(*mode),
                StateCmd::SetHeadCoolant(mode) => {
                    state.head_coolant = Some(*mode)
                }
                StateCmd::Dwell(d) => state.dwell_ms = Some(*d),
            }
        }
    }

    /// Count cutting commands (LineTo, ArcTo, BezierTo, QuadraticBezierTo, ScanLine).
    pub fn count_cutting(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| {
                matches!(
                    c.category,
                    OpCategory::Moving {
                        cmd: MoveCmd::LineTo
                            | MoveCmd::ArcTo { .. }
                            | MoveCmd::BezierTo { .. }
                            | MoveCmd::QuadraticBezierTo { .. }
                            | MoveCmd::ScanLine { .. },
                        ..
                    }
                )
            })
            .count()
    }

    /// Count travel (MoveTo) commands.
    pub fn count_travel(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| {
                matches!(
                    c.category,
                    OpCategory::Moving {
                        cmd: MoveCmd::MoveTo,
                        ..
                    }
                )
            })
            .count()
    }

    // --- Arithmetic ---
}

impl std::ops::Add<&Ops> for &Ops {
    type Output = Ops;

    fn add(self, other: &Ops) -> Ops {
        let mut result = Ops::new();
        for cmd in &self.commands {
            result.commands.push(cmd.clone());
        }
        for cmd in &other.commands {
            result.commands.push(cmd.clone());
        }
        result
    }
}

impl std::ops::Mul<usize> for &Ops {
    type Output = Ops;

    fn mul(self, other: usize) -> Ops {
        let mut result = Ops::new();
        for _ in 0..other {
            for cmd in &self.commands {
                result.commands.push(cmd.clone());
            }
        }
        result
    }
}

impl Ops {
    // --- Utility ---

    pub fn rect(&self, include_travel: bool) -> Option<Rect> {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut has_content = false;

        let mut curr_x = 0.0;
        let mut curr_y = 0.0;
        if include_travel {
            curr_x = self.last_move_to.x;
            curr_y = self.last_move_to.y;
        }

        for node in &self.commands {
            if let OpCategory::Moving { end, cmd } = &node.category {
                let (end_x, end_y) = (end.x, end.y);

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
                        (center.x * center.x + center.y * center.y).sqrt();
                    if (curr_x - end_x).abs() < EPSILON_COLLINEAR
                        && (curr_y - end_y).abs() < EPSILON_COLLINEAR
                        && radius > EPSILON_COLLINEAR
                    {
                        let cx = curr_x + center.x;
                        let cy = curr_y + center.y;
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
                            Point::new(curr_x, curr_y),
                            Point::new(end_x, end_y),
                            Point::new(center.x, center.y),
                            *cw,
                        );
                        Self::update_bounds(
                            &mut min_x, &mut min_y, &mut max_x, &mut max_y,
                            abox.min.x, abox.min.y,
                        );
                        Self::update_bounds(
                            &mut min_x, &mut min_y, &mut max_x, &mut max_y,
                            abox.max.x, abox.max.y,
                        );
                    }
                }

                curr_x = end_x;
                curr_y = end_y;
            }
        }

        if has_content {
            Some(Rect::new(min_x, min_y, max_x, max_y))
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
        super::transform::group::without_state(self)
    }

    pub fn group_by_auxiliary_state(&self) -> Vec<Ops> {
        super::transform::group::group_by_auxiliary_state(self)
    }
}

impl Default for Ops {
    fn default() -> Self {
        Self::new()
    }
}
