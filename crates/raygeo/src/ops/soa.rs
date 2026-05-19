use std::sync::Arc;

use super::axis::Axis;
use super::enums::{CommandCategory, CommandType, SectionType};
use super::state::State;
use crate::types::Point3D;

pub type ArcParams = (f64, f64, bool);
pub type BezierParams = (Point3D, Point3D);

#[derive(Clone, Debug)]
pub enum OpMetadata {
    None,
    Arc(ArcParams),
    Bezier(BezierParams),
    QuadraticBezier(Point3D),
    ScanLine(Arc<[u8]>),
    Dwell(f64),
    SetPower(f64),
    SetSpeed(i32),
    SetFrequency(i32),
    SetPulseWidth(f64),
    SetLaser(Arc<str>),
    LayerMarker(Arc<str>),
    WorkpieceMarker(Arc<str>),
    SectionMarker {
        section_type: SectionType,
        workpiece_uid: Option<Arc<str>>,
    },
}

#[derive(Clone, Debug)]
pub struct OpCommand {
    pub ct: CommandType,
    pub end: Point3D,
    pub metadata: OpMetadata,
    pub state: Option<State>,
    pub extra_axes: Option<Vec<(Axis, f64)>>,
}

impl OpCommand {
    pub fn new(ct: CommandType) -> Self {
        OpCommand {
            ct,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::None,
            state: None,
            extra_axes: None,
        }
    }

    pub fn move_to(
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpCommand {
            ct: CommandType::MoveTo,
            end: (x, y, z),
            metadata: OpMetadata::None,
            state: None,
            extra_axes: extra,
        }
    }

    pub fn line_to(
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpCommand {
            ct: CommandType::LineTo,
            end: (x, y, z),
            metadata: OpMetadata::None,
            state: None,
            extra_axes: extra,
        }
    }

    pub fn close_path(end: Point3D) -> Self {
        OpCommand {
            ct: CommandType::LineTo,
            end,
            metadata: OpMetadata::None,
            state: None,
            extra_axes: None,
        }
    }

    pub fn arc_to(
        x: f64,
        y: f64,
        i: f64,
        j: f64,
        clockwise: bool,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpCommand {
            ct: CommandType::ArcTo,
            end: (x, y, z),
            metadata: OpMetadata::Arc((i, j, clockwise)),
            state: None,
            extra_axes: extra,
        }
    }

    pub fn bezier_to(
        c1: Point3D,
        c2: Point3D,
        end: Point3D,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpCommand {
            ct: CommandType::BezierTo,
            end,
            metadata: OpMetadata::Bezier((c1, c2)),
            state: None,
            extra_axes: extra,
        }
    }

    pub fn quadratic_bezier_to(
        control: Point3D,
        end: Point3D,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpCommand {
            ct: CommandType::QuadraticBezierTo,
            end,
            metadata: OpMetadata::QuadraticBezier(control),
            state: None,
            extra_axes: extra,
        }
    }

    pub fn scan_to(
        x: f64,
        y: f64,
        z: f64,
        power_values: Option<Vec<u8>>,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        let pv = power_values.unwrap_or_else(|| vec![255]);
        OpCommand {
            ct: CommandType::ScanLine,
            end: (x, y, z),
            metadata: OpMetadata::ScanLine(Arc::from(pv)),
            state: None,
            extra_axes: extra,
        }
    }

    pub fn set_power(power: f64) -> Self {
        OpCommand {
            ct: CommandType::SetPower,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::SetPower(power),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_cut_speed(speed: i32) -> Self {
        OpCommand {
            ct: CommandType::SetCutSpeed,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::SetSpeed(speed),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_travel_speed(speed: i32) -> Self {
        OpCommand {
            ct: CommandType::SetTravelSpeed,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::SetSpeed(speed),
            state: None,
            extra_axes: None,
        }
    }

    pub fn dwell(duration_ms: f64) -> Self {
        OpCommand {
            ct: CommandType::Dwell,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::Dwell(duration_ms),
            state: None,
            extra_axes: None,
        }
    }

    pub fn enable_air_assist() -> Self {
        OpCommand {
            ct: CommandType::EnableAirAssist,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::None,
            state: None,
            extra_axes: None,
        }
    }

    pub fn disable_air_assist() -> Self {
        OpCommand {
            ct: CommandType::DisableAirAssist,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::None,
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_laser(laser_uid: &str) -> Self {
        OpCommand {
            ct: CommandType::SetLaser,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::SetLaser(Arc::from(laser_uid)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_frequency(frequency: i32) -> Self {
        OpCommand {
            ct: CommandType::SetFrequency,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::SetFrequency(frequency),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_pulse_width(pulse_width: f64) -> Self {
        OpCommand {
            ct: CommandType::SetPulseWidth,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::SetPulseWidth(pulse_width),
            state: None,
            extra_axes: None,
        }
    }

    pub fn job_start() -> Self {
        OpCommand::new(CommandType::JobStart)
    }

    pub fn job_end() -> Self {
        OpCommand::new(CommandType::JobEnd)
    }

    pub fn layer_start(layer_uid: &str) -> Self {
        OpCommand {
            ct: CommandType::LayerStart,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::LayerMarker(Arc::from(layer_uid)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn layer_end(layer_uid: &str) -> Self {
        OpCommand {
            ct: CommandType::LayerEnd,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::LayerMarker(Arc::from(layer_uid)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn workpiece_start(workpiece_uid: &str) -> Self {
        OpCommand {
            ct: CommandType::WorkpieceStart,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::WorkpieceMarker(Arc::from(workpiece_uid)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn workpiece_end(workpiece_uid: &str) -> Self {
        OpCommand {
            ct: CommandType::WorkpieceEnd,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::WorkpieceMarker(Arc::from(workpiece_uid)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn ops_section_start(
        section_type: SectionType,
        workpiece_uid: &str,
    ) -> Self {
        OpCommand {
            ct: CommandType::OpsSectionStart,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::SectionMarker {
                section_type,
                workpiece_uid: Some(Arc::from(workpiece_uid)),
            },
            state: None,
            extra_axes: None,
        }
    }

    pub fn ops_section_end(section_type: SectionType) -> Self {
        OpCommand {
            ct: CommandType::OpsSectionEnd,
            end: (0.0, 0.0, 0.0),
            metadata: OpMetadata::SectionMarker {
                section_type,
                workpiece_uid: None,
            },
            state: None,
            extra_axes: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SoA {
    pub commands: Vec<OpCommand>,
}

impl SoA {
    pub fn new() -> Self {
        SoA {
            commands: Vec::new(),
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
        super::enums::category(self.commands[idx].ct)
    }

    pub fn endpoint(&self, idx: usize) -> Point3D {
        self.commands[idx].end
    }

    pub fn set_endpoint(&mut self, idx: usize, end: Point3D) {
        self.commands[idx].end = end;
    }

    pub fn arc_params(&self, idx: usize) -> &ArcParams {
        match &self.commands[idx].metadata {
            OpMetadata::Arc(params) => params,
            _ => panic!("arc_params called on non-arc command"),
        }
    }

    pub fn arc_center_offset(&self, idx: usize) -> (f64, f64) {
        let ap = self.arc_params(idx);
        (ap.0, ap.1)
    }

    pub fn arc_clockwise(&self, idx: usize) -> bool {
        self.arc_params(idx).2
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

    pub fn bezier_params(&self, idx: usize) -> &BezierParams {
        match &self.commands[idx].metadata {
            OpMetadata::Bezier(params) => params,
            _ => panic!("bezier_params called on non-bezier command"),
        }
    }

    pub fn set_bezier_params(&mut self, idx: usize, c1: Point3D, c2: Point3D) {
        self.commands[idx].metadata = OpMetadata::Bezier((c1, c2));
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

    pub fn set_scanline_data(&mut self, idx: usize, data: Vec<u8>) {
        self.commands[idx].metadata = OpMetadata::ScanLine(Arc::from(data));
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
        self.commands[idx].extra_axes = Some(ea);
    }

    pub fn state(&self, idx: usize) -> Option<&State> {
        self.commands[idx].state.as_ref()
    }

    pub fn set_state(&mut self, idx: usize, st: State) {
        self.commands[idx].state = Some(st);
    }

    pub fn push(&mut self, cmd: OpCommand) {
        self.commands.push(cmd);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}
