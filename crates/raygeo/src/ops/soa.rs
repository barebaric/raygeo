use super::axis::Axis;
use super::enums::{CommandCategory, CommandType, SectionType};
use super::state::State;
use crate::types::Point3D;

pub type ArcParams = (f64, f64, bool);
pub type BezierParams = (Point3D, Point3D);

#[derive(Clone, Debug)]
pub struct SoA {
    pub types: Vec<CommandType>,
    pub endpoints: Vec<Point3D>,

    pub arc_data: Vec<ArcParams>,
    pub arc_map: Vec<i32>,

    pub bezier_data: Vec<BezierParams>,
    pub bezier_map: Vec<i32>,

    pub quad_data: Vec<Point3D>,
    pub quad_map: Vec<i32>,

    pub scanline_data: Vec<Vec<u8>>,
    pub scanline_map: Vec<i32>,

    pub dwell_durations: Vec<f64>,
    pub dwell_map: Vec<i32>,

    pub powers: Vec<f64>,
    pub power_map: Vec<i32>,

    pub speeds: Vec<i32>,
    pub speed_map: Vec<i32>,

    pub frequencies: Vec<i32>,
    pub frequency_map: Vec<i32>,

    pub pulse_widths: Vec<f64>,
    pub pulse_width_map: Vec<i32>,

    pub laser_uids: Vec<String>,
    pub laser_uid_map: Vec<i32>,

    pub layer_uids: Vec<String>,
    pub layer_uid_map: Vec<i32>,

    pub workpiece_uids: Vec<String>,
    pub workpiece_uid_map: Vec<i32>,

    pub section_types: Vec<SectionType>,
    pub section_workpiece_uids: Vec<Option<String>>,
    pub section_map: Vec<i32>,

    pub extra_axes: Vec<Vec<(Axis, f64)>>,
    pub extra_axes_map: Vec<i32>,

    pub states: Vec<State>,
    pub state_map: Vec<i32>,
}

impl SoA {
    pub fn new() -> Self {
        SoA {
            types: Vec::new(),
            endpoints: Vec::new(),
            arc_data: Vec::new(),
            arc_map: Vec::new(),
            bezier_data: Vec::new(),
            bezier_map: Vec::new(),
            quad_data: Vec::new(),
            quad_map: Vec::new(),
            scanline_data: Vec::new(),
            scanline_map: Vec::new(),
            dwell_durations: Vec::new(),
            dwell_map: Vec::new(),
            powers: Vec::new(),
            power_map: Vec::new(),
            speeds: Vec::new(),
            speed_map: Vec::new(),
            frequencies: Vec::new(),
            frequency_map: Vec::new(),
            pulse_widths: Vec::new(),
            pulse_width_map: Vec::new(),
            laser_uids: Vec::new(),
            laser_uid_map: Vec::new(),
            layer_uids: Vec::new(),
            layer_uid_map: Vec::new(),
            workpiece_uids: Vec::new(),
            workpiece_uid_map: Vec::new(),
            section_types: Vec::new(),
            section_workpiece_uids: Vec::new(),
            section_map: Vec::new(),
            extra_axes: Vec::new(),
            extra_axes_map: Vec::new(),
            states: Vec::new(),
            state_map: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn command_type(&self, idx: usize) -> CommandType {
        self.types[idx]
    }

    pub fn category(&self, idx: usize) -> CommandCategory {
        super::enums::category(self.types[idx])
    }

    pub fn endpoint(&self, idx: usize) -> Point3D {
        self.endpoints[idx]
    }

    pub fn set_endpoint(&mut self, idx: usize, end: Point3D) {
        self.endpoints[idx] = end;
    }

    pub fn arc_params(&self, idx: usize) -> &ArcParams {
        &self.arc_data[self.arc_map[idx] as usize]
    }

    pub fn arc_center_offset(&self, idx: usize) -> (f64, f64) {
        let ap = &self.arc_data[self.arc_map[idx] as usize];
        (ap.0, ap.1)
    }

    pub fn arc_clockwise(&self, idx: usize) -> bool {
        self.arc_data[self.arc_map[idx] as usize].2
    }

    pub fn set_arc_params(
        &mut self,
        idx: usize,
        center_offset: Option<(f64, f64)>,
        clockwise: Option<bool>,
    ) {
        let ai = self.arc_map[idx] as usize;
        let old = self.arc_data[ai];
        let co = center_offset.unwrap_or((old.0, old.1));
        let cw = clockwise.unwrap_or(old.2);
        self.arc_data[ai] = (co.0, co.1, cw);
    }

    pub fn bezier_params(&self, idx: usize) -> &BezierParams {
        &self.bezier_data[self.bezier_map[idx] as usize]
    }

    pub fn set_bezier_params(&mut self, idx: usize, c1: Point3D, c2: Point3D) {
        let ai = self.bezier_map[idx] as usize;
        self.bezier_data[ai] = (c1, c2);
    }

    pub fn quad_params(&self, idx: usize) -> &Point3D {
        &self.quad_data[self.quad_map[idx] as usize]
    }

    pub fn set_quad_params(&mut self, idx: usize, control: Point3D) {
        let qi = self.quad_map[idx] as usize;
        self.quad_data[qi] = control;
    }

    pub fn scanline_data(&self, idx: usize) -> &[u8] {
        let si = self.scanline_map[idx] as usize;
        &self.scanline_data[si]
    }

    pub fn set_scanline_data(&mut self, idx: usize, data: Vec<u8>) {
        let si = self.scanline_map[idx] as usize;
        self.scanline_data[si] = data;
    }

    pub fn dwell_duration(&self, idx: usize) -> f64 {
        self.dwell_durations[self.dwell_map[idx] as usize]
    }

    pub fn power(&self, idx: usize) -> f64 {
        self.powers[self.power_map[idx] as usize]
    }

    pub fn speed(&self, idx: usize) -> i32 {
        self.speeds[self.speed_map[idx] as usize]
    }

    pub fn frequency(&self, idx: usize) -> i32 {
        self.frequencies[self.frequency_map[idx] as usize]
    }

    pub fn pulse_width(&self, idx: usize) -> f64 {
        self.pulse_widths[self.pulse_width_map[idx] as usize]
    }

    pub fn laser_uid(&self, idx: usize) -> &str {
        &self.laser_uids[self.laser_uid_map[idx] as usize]
    }

    pub fn layer_uid(&self, idx: usize) -> &str {
        &self.layer_uids[self.layer_uid_map[idx] as usize]
    }

    pub fn workpiece_uid(&self, idx: usize) -> &str {
        &self.workpiece_uids[self.workpiece_uid_map[idx] as usize]
    }

    pub fn section_type(&self, idx: usize) -> SectionType {
        let si = self.section_map[idx] as usize;
        self.section_types[si]
    }

    pub fn section_workpiece_uid(&self, idx: usize) -> Option<&str> {
        let si = self.section_map[idx] as usize;
        self.section_workpiece_uids[si].as_deref()
    }

    pub fn extra_axes(&self, idx: usize) -> Option<&[(Axis, f64)]> {
        let ei = self.extra_axes_map[idx];
        if ei == -1 {
            None
        } else {
            Some(&self.extra_axes[ei as usize])
        }
    }

    pub fn set_extra_axes(&mut self, idx: usize, ea: Vec<(Axis, f64)>) {
        let ei = self.extra_axes_map[idx];
        if ei == -1 {
            self.extra_axes_map[idx] = self.extra_axes.len() as i32;
            self.extra_axes.push(ea);
        } else {
            self.extra_axes[ei as usize] = ea;
        }
    }

    pub fn state(&self, idx: usize) -> Option<&State> {
        let si = self.state_map[idx];
        if si == -1 {
            None
        } else {
            Some(&self.states[si as usize])
        }
    }

    pub fn set_state(&mut self, idx: usize, st: State) {
        let si = self.state_map[idx];
        if si == -1 {
            self.state_map[idx] = self.states.len() as i32;
            self.states.push(st);
        } else {
            self.states[si as usize] = st;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn append(
        &mut self,
        ct: CommandType,
        end: Option<Point3D>,
        arc_params: Option<ArcParams>,
        bezier_params: Option<BezierParams>,
        quad_params: Option<Point3D>,
        scanline: Option<Vec<u8>>,
        dwell_duration: Option<f64>,
        power: Option<f64>,
        speed: Option<i32>,
        frequency: Option<i32>,
        pulse_width: Option<f64>,
        laser_uid: Option<String>,
        layer_uid: Option<String>,
        workpiece_uid: Option<String>,
        section_type: Option<SectionType>,
        section_workpiece_uid: Option<String>,
        extra_axes: Option<Vec<(Axis, f64)>>,
        state: Option<State>,
    ) {
        self.types.push(ct);
        self.endpoints.push(end.unwrap_or((0.0, 0.0, 0.0)));

        if let Some(ap) = arc_params {
            self.arc_map.push(self.arc_data.len() as i32);
            self.arc_data.push(ap);
        } else {
            self.arc_map.push(-1);
        }

        if let Some(bp) = bezier_params {
            self.bezier_map.push(self.bezier_data.len() as i32);
            self.bezier_data.push(bp);
        } else {
            self.bezier_map.push(-1);
        }

        if let Some(qp) = quad_params {
            self.quad_map.push(self.quad_data.len() as i32);
            self.quad_data.push(qp);
        } else {
            self.quad_map.push(-1);
        }

        if let Some(sl) = scanline {
            self.scanline_map.push(self.scanline_data.len() as i32);
            self.scanline_data.push(sl);
        } else {
            self.scanline_map.push(-1);
        }

        if let Some(dd) = dwell_duration {
            self.dwell_map.push(self.dwell_durations.len() as i32);
            self.dwell_durations.push(dd);
        } else {
            self.dwell_map.push(-1);
        }

        if let Some(p) = power {
            self.power_map.push(self.powers.len() as i32);
            self.powers.push(p);
        } else {
            self.power_map.push(-1);
        }

        if let Some(s) = speed {
            self.speed_map.push(self.speeds.len() as i32);
            self.speeds.push(s);
        } else {
            self.speed_map.push(-1);
        }

        if let Some(f) = frequency {
            self.frequency_map.push(self.frequencies.len() as i32);
            self.frequencies.push(f);
        } else {
            self.frequency_map.push(-1);
        }

        if let Some(pw) = pulse_width {
            self.pulse_width_map.push(self.pulse_widths.len() as i32);
            self.pulse_widths.push(pw);
        } else {
            self.pulse_width_map.push(-1);
        }

        if let Some(lu) = laser_uid {
            self.laser_uid_map.push(self.laser_uids.len() as i32);
            self.laser_uids.push(lu);
        } else {
            self.laser_uid_map.push(-1);
        }

        if let Some(layer) = layer_uid {
            self.layer_uid_map.push(self.layer_uids.len() as i32);
            self.layer_uids.push(layer);
        } else {
            self.layer_uid_map.push(-1);
        }

        if let Some(wu) = workpiece_uid {
            self.workpiece_uid_map
                .push(self.workpiece_uids.len() as i32);
            self.workpiece_uids.push(wu);
        } else {
            self.workpiece_uid_map.push(-1);
        }

        if let Some(st) = section_type {
            self.section_map.push(self.section_types.len() as i32);
            self.section_types.push(st);
            self.section_workpiece_uids.push(section_workpiece_uid);
        } else {
            self.section_map.push(-1);
        }

        if let Some(ea) = extra_axes {
            if !ea.is_empty() {
                self.extra_axes_map.push(self.extra_axes.len() as i32);
                self.extra_axes.push(ea);
            } else {
                self.extra_axes_map.push(-1);
            }
        } else {
            self.extra_axes_map.push(-1);
        }

        if let Some(s) = state {
            self.state_map.push(self.states.len() as i32);
            self.states.push(s);
        } else {
            self.state_map.push(-1);
        }
    }

    pub fn append_from_args(soa: &mut SoA, args: &AppendArgs) {
        soa.append(
            args.ct,
            args.end,
            args.arc_params,
            args.bezier_params,
            args.quad_params,
            args.scanline.clone(),
            args.dwell_duration,
            args.power,
            args.speed,
            args.frequency,
            args.pulse_width,
            args.laser_uid.clone(),
            args.layer_uid.clone(),
            args.workpiece_uid.clone(),
            args.section_type,
            args.section_workpiece_uid.clone(),
            args.extra_axes.clone(),
            args.state.clone(),
        );
    }

    pub fn copy_entry(&self, src_idx: usize) -> AppendArgs {
        let ct = self.types[src_idx];
        let cat = self.category(src_idx);

        let mut args = AppendArgs::new(ct);

        if cat == CommandCategory::Moving {
            args.end = Some(self.endpoints[src_idx]);
        }

        if ct == CommandType::ArcTo {
            args.arc_params = Some(*self.arc_params(src_idx));
        }
        if ct == CommandType::BezierTo {
            args.bezier_params = Some(*self.bezier_params(src_idx));
        }
        if ct == CommandType::QuadraticBezierTo {
            args.quad_params = Some(*self.quad_params(src_idx));
        }
        if ct == CommandType::ScanLine {
            args.scanline = Some(self.scanline_data(src_idx).to_vec());
        }
        if ct == CommandType::Dwell {
            args.dwell_duration = Some(self.dwell_duration(src_idx));
        }
        if ct == CommandType::SetPower {
            args.power = Some(self.power(src_idx));
        }
        if ct == CommandType::SetCutSpeed || ct == CommandType::SetTravelSpeed {
            args.speed = Some(self.speed(src_idx));
        }
        if ct == CommandType::SetFrequency {
            args.frequency = Some(self.frequency(src_idx));
        }
        if ct == CommandType::SetPulseWidth {
            args.pulse_width = Some(self.pulse_width(src_idx));
        }
        if ct == CommandType::SetLaser {
            args.laser_uid = Some(self.laser_uid(src_idx).to_string());
        }
        if ct == CommandType::LayerStart || ct == CommandType::LayerEnd {
            args.layer_uid = Some(self.layer_uid(src_idx).to_string());
        }
        if ct == CommandType::WorkpieceStart || ct == CommandType::WorkpieceEnd
        {
            args.workpiece_uid = Some(self.workpiece_uid(src_idx).to_string());
        }
        if ct == CommandType::OpsSectionStart
            || ct == CommandType::OpsSectionEnd
        {
            args.section_type = Some(self.section_type(src_idx));
            args.section_workpiece_uid =
                self.section_workpiece_uid(src_idx).map(|s| s.to_string());
        }

        if let Some(ea) = self.extra_axes(src_idx) {
            args.extra_axes = Some(ea.to_vec());
        }
        if let Some(s) = self.state(src_idx) {
            args.state = Some(s.clone());
        }

        args
    }

    pub fn deep_copy_entry(&self, src_idx: usize) -> AppendArgs {
        let mut args = self.copy_entry(src_idx);
        if let Some(ref sl) = args.scanline {
            args.scanline = Some(sl.clone());
        }
        if let Some(ref ea) = args.extra_axes {
            args.extra_axes = Some(ea.clone());
        }
        if let Some(ref s) = args.state {
            args.state = Some(s.clone());
        }
        args
    }
}

pub struct AppendArgs {
    pub ct: CommandType,
    pub end: Option<Point3D>,
    pub arc_params: Option<ArcParams>,
    pub bezier_params: Option<BezierParams>,
    pub quad_params: Option<Point3D>,
    pub scanline: Option<Vec<u8>>,
    pub dwell_duration: Option<f64>,
    pub power: Option<f64>,
    pub speed: Option<i32>,
    pub frequency: Option<i32>,
    pub pulse_width: Option<f64>,
    pub laser_uid: Option<String>,
    pub layer_uid: Option<String>,
    pub workpiece_uid: Option<String>,
    pub section_type: Option<SectionType>,
    pub section_workpiece_uid: Option<String>,
    pub extra_axes: Option<Vec<(Axis, f64)>>,
    pub state: Option<State>,
}

impl AppendArgs {
    pub fn new(ct: CommandType) -> Self {
        AppendArgs {
            ct,
            end: None,
            arc_params: None,
            bezier_params: None,
            quad_params: None,
            scanline: None,
            dwell_duration: None,
            power: None,
            speed: None,
            frequency: None,
            pulse_width: None,
            laser_uid: None,
            layer_uid: None,
            workpiece_uid: None,
            section_type: None,
            section_workpiece_uid: None,
            extra_axes: None,
            state: None,
        }
    }
}
