use std::fmt::Write;

use crate::constants::EPSILON_COLLINEAR;

use super::axis::Axis;
use super::enums::{CommandCategory, CommandType, SectionType};
use super::types::{ArcParams, BezierParams, OpCommand, OpMetadata};
use super::state::State;
use crate::types::Point3D;

#[derive(Clone, Debug)]
pub struct Ops {
    pub commands: Vec<OpCommand>,
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
        self.commands[idx].ct
    }

    pub fn category(&self, idx: usize) -> CommandCategory {
        self.commands[idx].ct.category()
    }

    pub fn is_travel(&self, idx: usize) -> bool {
        self.commands[idx].ct == CommandType::MoveTo
    }

    pub fn is_cutting(&self, idx: usize) -> bool {
        let ct = self.commands[idx].ct;
        ct.category() == CommandCategory::Moving && ct != CommandType::MoveTo
    }

    pub fn is_state(&self, idx: usize) -> bool {
        self.commands[idx].ct.category() == CommandCategory::State
    }

    pub fn is_marker(&self, idx: usize) -> bool {
        self.commands[idx].ct.category() == CommandCategory::Marker
    }

    pub fn is_scanline(&self, idx: usize) -> bool {
        self.commands[idx].ct == CommandType::ScanLine
    }

    pub fn indices_of(&self, ct: CommandType) -> Vec<usize> {
        self.commands
            .iter()
            .enumerate()
            .filter(|(_, cmd)| cmd.ct == ct)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn endpoint(&self, idx: usize) -> Point3D {
        self.commands[idx].end
    }

    pub fn set_endpoint(&mut self, idx: usize, end: Point3D) {
        self.commands[idx].end = end;
    }

    pub fn set_arc_params(
        &mut self,
        idx: usize,
        center_offset: Option<(f64, f64)>,
        clockwise: Option<bool>,
    ) {
        if let OpMetadata::Arc(ref old) = self.commands[idx].metadata {
            let co = center_offset.unwrap_or((old.0, old.1));
            let cw = clockwise.unwrap_or(old.2);
            self.commands[idx].metadata = OpMetadata::Arc((co.0, co.1, cw));
        }
    }

    pub fn arc_params(&self, idx: usize) -> &ArcParams {
        match &self.commands[idx].metadata {
            OpMetadata::Arc(params) => params,
            _ => panic!("arc_params called on non-arc command"),
        }
    }

    pub fn set_bezier_params(&mut self, idx: usize, c1: Point3D, c2: Point3D) {
        self.commands[idx].metadata = OpMetadata::Bezier((c1, c2));
    }

    pub fn bezier_params(&self, idx: usize) -> &BezierParams {
        match &self.commands[idx].metadata {
            OpMetadata::Bezier(params) => params,
            _ => panic!("bezier_params called on non-bezier command"),
        }
    }

    pub fn quad_params(&self, idx: usize) -> &Point3D {
        match &self.commands[idx].metadata {
            OpMetadata::QuadraticBezier(control) => control,
            _ => panic!("quad_params called on non-quadratic-bezier command"),
        }
    }

    pub fn set_quad_params(&mut self, idx: usize, control: Point3D) {
        self.commands[idx].metadata = OpMetadata::QuadraticBezier(control);
    }

    pub fn scanline_data(&self, idx: usize) -> &[u8] {
        match &self.commands[idx].metadata {
            OpMetadata::ScanLine(data) => data,
            _ => panic!("scanline_data called on non-scanline command"),
        }
    }

    pub fn dwell_duration(&self, idx: usize) -> f64 {
        match self.commands[idx].metadata {
            OpMetadata::Dwell(d) => d,
            _ => panic!("dwell_duration called on non-dwell command"),
        }
    }

    pub fn power(&self, idx: usize) -> f64 {
        match self.commands[idx].metadata {
            OpMetadata::SetPower(p) => p,
            _ => panic!("power called on non-set-power command"),
        }
    }

    pub fn speed(&self, idx: usize) -> i32 {
        match self.commands[idx].metadata {
            OpMetadata::SetSpeed(s) => s,
            _ => panic!("speed called on non-set-speed command"),
        }
    }

    pub fn frequency(&self, idx: usize) -> i32 {
        match self.commands[idx].metadata {
            OpMetadata::SetFrequency(f) => f,
            _ => panic!("frequency called on non-set-frequency command"),
        }
    }

    pub fn pulse_width(&self, idx: usize) -> f64 {
        match self.commands[idx].metadata {
            OpMetadata::SetPulseWidth(pw) => pw,
            _ => panic!("pulse_width called on non-set-pulse-width command"),
        }
    }

    pub fn laser_uid(&self, idx: usize) -> &str {
        match &self.commands[idx].metadata {
            OpMetadata::SetLaser(uid) => uid,
            _ => panic!("laser_uid called on non-set-laser command"),
        }
    }

    pub fn layer_uid(&self, idx: usize) -> &str {
        match &self.commands[idx].metadata {
            OpMetadata::LayerMarker(uid) => uid,
            _ => panic!("layer_uid called on non-layer command"),
        }
    }

    pub fn workpiece_uid(&self, idx: usize) -> &str {
        match &self.commands[idx].metadata {
            OpMetadata::WorkpieceMarker(uid) => uid,
            _ => panic!("workpiece_uid called on non-workpiece command"),
        }
    }

    pub fn section_type(&self, idx: usize) -> SectionType {
        match &self.commands[idx].metadata {
            OpMetadata::SectionMarker { section_type, .. } => *section_type,
            _ => panic!("section_type called on non-section command"),
        }
    }

    pub fn section_workpiece_uid(&self, idx: usize) -> Option<&str> {
        match &self.commands[idx].metadata {
            OpMetadata::SectionMarker { workpiece_uid, .. } => {
                workpiece_uid.as_deref()
            }
            _ => panic!("section_workpiece_uid called on non-section command"),
        }
    }

    pub fn extra_axes(&self, idx: usize) -> Option<&[(Axis, f64)]> {
        self.commands[idx].extra_axes.as_deref()
    }

    pub fn set_extra_axes(&mut self, idx: usize, ea: Vec<(Axis, f64)>) {
        self.commands[idx].extra_axes = Some(std::sync::Arc::from(ea));
    }

    pub fn state(&self, idx: usize) -> Option<&State> {
        self.commands[idx].state.as_ref()
    }

    pub fn preloaded_state(&self, idx: usize) -> Option<&State> {
        self.commands[idx].state.as_ref()
    }

    pub fn set_state_on_moving(&mut self, state: &State) {
        for i in 0..self.commands.len() {
            if self.commands[i].ct.category() == CommandCategory::Moving {
                self.commands[i].state = Some(state.clone());
            }
        }
    }

    pub fn set_state_at(&mut self, idx: usize, state: &State) {
        self.commands[idx].state = Some(state.clone());
    }

    pub fn distance_at(&self, idx: usize, last_point: Option<Point3D>) -> f64 {
        if self.commands[idx].ct.category() != CommandCategory::Moving {
            return 0.0;
        }
        match last_point {
            None => 0.0,
            Some(lp) => {
                let end = self.commands[idx].end;
                let dx = end.0 - lp.0;
                let dy = end.1 - lp.1;
                (dx * dx + dy * dy).sqrt()
            }
        }
    }

    pub fn distance(&self) -> f64 {
        let mut total = 0.0;
        let mut last: Option<Point3D> = None;
        for i in 0..self.commands.len() {
            if self.commands[i].ct.category() == CommandCategory::Moving {
                let end = self.commands[i].end;
                if let Some(lp) = last {
                    let dx = end.0 - lp.0;
                    let dy = end.1 - lp.1;
                    total += (dx * dx + dy * dy).sqrt();
                }
                last = Some(end);
            }
        }
        total
    }

    pub fn cut_distance(&self) -> f64 {
        let mut total = 0.0;
        let mut last: Option<Point3D> = None;
        for i in 0..self.commands.len() {
            let ct = self.commands[i].ct;
            if ct.category() == CommandCategory::Moving {
                let end = self.commands[i].end;
                if let Some(lp) = last {
                    if ct != CommandType::MoveTo {
                        let dx = end.0 - lp.0;
                        let dy = end.1 - lp.1;
                        total += (dx * dx + dy * dy).sqrt();
                    }
                }
                last = Some(end);
            }
        }
        total
    }

    pub fn scanline_count(&self) -> usize {
        self.indices_of(CommandType::ScanLine).len()
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
        self.commands.push(OpCommand::move_to(x, y, z, extra));
        self.invalidate_time_cache();
    }

    pub fn line_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.commands.push(OpCommand::line_to(x, y, z, extra));
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
        self.commands.push(OpCommand::arc_to(x, y, i, j, clockwise, z, extra));
        self.invalidate_time_cache();
    }

    pub fn bezier_to(
        &mut self,
        c1: Point3D,
        c2: Point3D,
        end: Point3D,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        if self.commands.len() == 0 {
            return;
        }
        self.commands.push(OpCommand::bezier_to(c1, c2, end, extra));
        self.invalidate_time_cache();
    }

    pub fn quadratic_bezier_to(
        &mut self,
        control: Point3D,
        end: Point3D,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.commands.push(OpCommand::quadratic_bezier_to(control, end, extra));
        self.invalidate_time_cache();
    }

    pub fn scan_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        power_values: Option<Vec<u8>>,
        extra: Option<Vec<(Axis, f64)>>,
    ) {
        self.commands.push(OpCommand::scan_to(x, y, z, power_values, extra));
        self.invalidate_time_cache();
    }

    pub fn set_power(&mut self, power: f64) {
        self.commands.push(OpCommand::set_power(power));
    }

    pub fn set_cut_speed(&mut self, speed: i32) {
        self.commands.push(OpCommand::set_cut_speed(speed));
        self.invalidate_time_cache();
    }

    pub fn set_travel_speed(&mut self, speed: i32) {
        self.commands.push(OpCommand::set_travel_speed(speed));
        self.invalidate_time_cache();
    }

    pub fn dwell(&mut self, duration_ms: f64) {
        self.commands.push(OpCommand::dwell(duration_ms));
        self.invalidate_time_cache();
    }

    pub fn enable_air_assist(&mut self) {
        self.commands.push(OpCommand::enable_air_assist());
        self.invalidate_time_cache();
    }

    pub fn disable_air_assist(&mut self) {
        self.commands.push(OpCommand::disable_air_assist());
        self.invalidate_time_cache();
    }

    pub fn set_laser(&mut self, laser_uid: &str) {
        self.commands.push(OpCommand::set_laser(laser_uid));
        self.invalidate_time_cache();
    }

    pub fn set_frequency(&mut self, frequency: i32) {
        self.commands.push(OpCommand::set_frequency(frequency));
        self.invalidate_time_cache();
    }

    pub fn set_pulse_width(&mut self, pulse_width: f64) {
        self.commands.push(OpCommand::set_pulse_width(pulse_width));
        self.invalidate_time_cache();
    }

    pub fn job_start(&mut self) {
        self.commands.push(OpCommand::job_start());
        self.invalidate_time_cache();
    }

    pub fn job_end(&mut self) {
        self.commands.push(OpCommand::job_end());
        self.invalidate_time_cache();
    }

    pub fn layer_start(&mut self, layer_uid: &str) {
        self.commands.push(OpCommand::layer_start(layer_uid));
        self.invalidate_time_cache();
    }

    pub fn layer_end(&mut self, layer_uid: &str) {
        self.commands.push(OpCommand::layer_end(layer_uid));
        self.invalidate_time_cache();
    }

    pub fn workpiece_start(&mut self, workpiece_uid: &str) {
        self.commands.push(OpCommand::workpiece_start(workpiece_uid));
        self.invalidate_time_cache();
    }

    pub fn workpiece_end(&mut self, workpiece_uid: &str) {
        self.commands.push(OpCommand::workpiece_end(workpiece_uid));
        self.invalidate_time_cache();
    }

    pub fn ops_section_start(
        &mut self,
        section_type: SectionType,
        workpiece_uid: &str,
    ) {
        self.commands.push(OpCommand::ops_section_start(section_type, workpiece_uid));
        self.invalidate_time_cache();
    }

    pub fn ops_section_end(&mut self, section_type: SectionType) {
        self.commands.push(OpCommand::ops_section_end(section_type));
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
        for i in 0..self.commands.len() {
            let is_move = self.commands[i].ct == CommandType::MoveTo;
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
        for i in 0..=idx {
            if self.category(i) == CommandCategory::State {
                Self::apply_state_at(
                    &mut state,
                    self.commands[i].ct,
                    self,
                    i,
                );
            }
        }
        state
    }

    // --- State ---

    pub fn preload_state(&mut self) {
        let mut state = State::default();
        for i in 0..self.commands.len() {
            if self.category(i) == CommandCategory::State {
                Self::apply_state_at(
                    &mut state,
                    self.commands[i].ct,
                    self,
                    i,
                );
            } else if self.category(i) == CommandCategory::Moving {
                self.commands[i].state = Some(state.clone());
            }
        }
    }

    fn apply_state_at(
        state: &mut State,
        ct: CommandType,
        ops: &Ops,
        idx: usize,
    ) {
        match ct {
            CommandType::SetPower => state.power = ops.power(idx),
            CommandType::SetCutSpeed => state.cut_speed = Some(ops.speed(idx)),
            CommandType::SetTravelSpeed => {
                state.travel_speed = Some(ops.speed(idx))
            }
            CommandType::EnableAirAssist => state.air_assist = true,
            CommandType::DisableAirAssist => state.air_assist = false,
            CommandType::SetLaser => {
                state.active_laser_uid = Some(ops.laser_uid(idx).to_string())
            }
            CommandType::SetFrequency => {
                state.frequency = Some(ops.frequency(idx))
            }
            CommandType::SetPulseWidth => {
                state.pulse_width = Some(ops.pulse_width(idx))
            }
            _ => {}
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
        for i in 0..self.commands.len() {
            let cmd = &self.commands[i];
            let ct = cmd.ct;
            let _ = write!(out, "  [{}] {}", i, ct.name());
            if self.category(i) == CommandCategory::Moving {
                let _ = write!(
                    out,
                    " end=({:.3},{:.3},{:.3})",
                    cmd.end.0, cmd.end.1, cmd.end.2
                );
                if ct == CommandType::ArcTo {
                    let ap = self.arc_params(i);
                    let _ = write!(
                        out,
                        " arc=(i={:.3},j={:.3},cw={})",
                        ap.0, ap.1, ap.2
                    );
                }
                if ct == CommandType::BezierTo {
                    let bp = self.bezier_params(i);
                    let _ = write!(
                        out,
                        " bezier=(c1=({:.3},{:.3}),c2=({:.3},{:.3}))",
                        bp.0 .0, bp.0 .1, bp.1 .0, bp.1 .1
                    );
                }
            }
            let _ = writeln!(out);
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
        if self.commands.len() == 0 {
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

    pub fn segment_indices(&self) -> Vec<Vec<usize>> {
        super::group::segment_indices(self)
    }

    pub fn segments(&self) -> Vec<Vec<usize>> {
        super::group::segments(self)
    }

    pub fn get_frame(&self, power: Option<f64>, speed: Option<f64>) -> Self {
        let (min_x, min_y, max_x, max_y) = self.rect(false);
        if (min_x, min_y, max_x, max_y) == (0.0, 0.0, 0.0, 0.0) {
            return Ops::new();
        }
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

    pub fn rect(&self, include_travel: bool) -> crate::types::Rect {
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

        let mut xs: Vec<f64> = Vec::new();
        let mut ys: Vec<f64> = Vec::new();
        let mut arcs: Vec<(f64, f64, f64, f64, f64, f64, bool)> = Vec::new();

        for i in 0..self.commands.len() {
            if self.category(i) != CommandCategory::Moving {
                continue;
            }
            let ct = self.command_type(i);
            let end = self.endpoint(i);
            let (end_x, end_y) = (end.0, end.1);

            if ct == CommandType::MoveTo {
                if include_travel {
                    xs.push(curr_x);
                    ys.push(curr_y);
                    xs.push(end_x);
                    ys.push(end_y);
                    has_content = true;
                }
                curr_x = end_x;
                curr_y = end_y;
                continue;
            }

            xs.push(curr_x);
            ys.push(curr_y);
            xs.push(end_x);
            ys.push(end_y);
            has_content = true;

            if ct == CommandType::ArcTo {
                let &(ci, cj, cw) = self.arc_params(i);
                arcs.push((curr_x, curr_y, end_x, end_y, ci, cj, cw));
            }

            curr_x = end_x;
            curr_y = end_y;
        }

        if !has_content {
            return (0.0, 0.0, 0.0, 0.0);
        }

        if !xs.is_empty() {
            for &x in &xs {
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
            }
            for &y in &ys {
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }

        for (ax, ay, bx, by, i, j, cw) in arcs {
            let radius = (i * i + j * j).sqrt();
            if (ax - bx).abs() < EPSILON_COLLINEAR
                && (ay - by).abs() < EPSILON_COLLINEAR
                && radius > EPSILON_COLLINEAR
            {
                let cx = ax + i;
                let cy = ay + j;
                if cx - radius < min_x {
                    min_x = cx - radius;
                }
                if cx + radius > max_x {
                    max_x = cx + radius;
                }
                if cy - radius < min_y {
                    min_y = cy - radius;
                }
                if cy + radius > max_y {
                    max_y = cy + radius;
                }
            } else {
                let abox = crate::geo::shape::arc::get_arc_bounds(
                    (ax, ay),
                    (bx, by),
                    (i, j),
                    cw,
                );
                if abox.0 < min_x {
                    min_x = abox.0;
                }
                if abox.1 < min_y {
                    min_y = abox.1;
                }
                if abox.2 > max_x {
                    max_x = abox.2;
                }
                if abox.3 > max_y {
                    max_y = abox.3;
                }
            }
        }

        (min_x, min_y, max_x, max_y)
    }

    pub fn without_state(&self) -> Self {
        super::group::without_state(self)
    }

    pub fn group_by_state_continuity(&self) -> Vec<Ops> {
        super::group::group_by_state_continuity(self)
    }

    pub fn from_geometry(geometry: &crate::Geometry) -> Self {
        let mut ops = Ops::new();
        if geometry.data.is_empty() {
            ops.last_move_to = geometry.last_move_to;
            return ops;
        }

        let mut last_pos = (0.0, 0.0, 0.0);
        for row in &geometry.data {
            let cmd = crate::Command::from_row(row).expect("invalid command");
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
                        clockwise,
                        end.2,
                        None,
                    );
                }
                crate::Command::Bezier {
                    end,
                    control1,
                    control2,
                } => {
                    let z0 = last_pos.2;
                    let z1 = end.2;
                    let c1_3d = (
                        control1.0,
                        control1.1,
                        z0 * 2.0 / 3.0 + z1 * 1.0 / 3.0,
                    );
                    let c2_3d = (
                        control2.0,
                        control2.1,
                        z0 * 1.0 / 3.0 + z1 * 2.0 / 3.0,
                    );
                    ops.bezier_to(c1_3d, c2_3d, (end.0, end.1, end.2), None);
                }
            }
            last_pos = cmd.end_point();
        }
        ops.last_move_to = geometry.last_move_to;
        ops
    }

    pub fn to_geometry(&self) -> crate::Geometry {
        let mut geo = crate::Geometry::new();
        for i in 0..self.commands.len() {
            let ct = self.commands[i].ct;
            if ct.category() != CommandCategory::Moving {
                continue;
            }
            let end = self.commands[i].end;
            match ct {
                CommandType::MoveTo => {
                    geo.move_to(end.0, end.1, end.2);
                }
                CommandType::LineTo => {
                    geo.line_to(end.0, end.1, end.2);
                }
                CommandType::ArcTo => {
                    let (ci, cj, cw) = match &self.commands[i].metadata {
                        OpMetadata::Arc(a) => *a,
                        _ => panic!("expected arc"),
                    };
                    geo.arc_to(end.0, end.1, ci, cj, cw, end.2);
                }
                CommandType::BezierTo => {
                    let (c1, c2) = match &self.commands[i].metadata {
                        OpMetadata::Bezier(b) => *b,
                        _ => panic!("expected bezier"),
                    };
                    geo.bezier_to(
                        ((c1.0, c1.1), (c2.0, c2.1), (end.0, end.1)),
                        end.2,
                    );
                }
                _ => {}
            }
        }
        geo
    }
}

fn estimate_time_core(
    ops: &Ops,
    default_cut_speed: f64,
    default_travel_speed: f64,
    acceleration: f64,
) -> f64 {
    let mut total_time = 0.0;
    let mut last_point = (0.0, 0.0, 0.0);
    let mut cut_speed = default_cut_speed;
    let mut travel_speed = default_travel_speed;

    for i in 0..ops.len() {
        if ops.is_state(i) {
            match ops.command_type(i) {
                CommandType::SetCutSpeed => {
                    cut_speed = ops.speed(i) as f64;
                }
                CommandType::SetTravelSpeed => {
                    travel_speed = ops.speed(i) as f64;
                }
                _ => {}
            }
            continue;
        }
        if ops.category(i) != CommandCategory::Moving {
            continue;
        }

        let end = ops.endpoint(i);
        let dx = end.0 - last_point.0;
        let dy = end.1 - last_point.1;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance < EPSILON_COLLINEAR {
            last_point = end;
            continue;
        }

        let speed = if ops.is_cutting(i) {
            cut_speed
        } else {
            travel_speed
        };

        let speed_mm_per_sec = speed / 60.0;

        let move_time = if acceleration > 0.0 {
            let accel_time = speed_mm_per_sec / acceleration;
            let accel_distance = 0.5 * acceleration * accel_time * accel_time;
            if distance < 2.0 * accel_distance {
                2.0 * (distance / acceleration).sqrt()
            } else {
                let cruise_distance = distance - 2.0 * accel_distance;
                let cruise_time = cruise_distance / speed_mm_per_sec;
                2.0 * accel_time + cruise_time
            }
        } else {
            distance / speed_mm_per_sec
        };

        total_time += move_time;
        last_point = end;
    }

    total_time
}

impl Default for Ops {
    fn default() -> Self {
        Self::new()
    }
}
