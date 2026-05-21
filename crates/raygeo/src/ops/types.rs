use std::sync::Arc;

use super::axis::Axis;
use super::enums::{CommandType, SectionType};
use super::state::State;
use crate::types::Point3D;

#[derive(Clone, Debug)]
pub enum MoveCmd {
    MoveTo,
    LineTo,
    ArcTo { center: (f64, f64), cw: bool },
    BezierTo { c1: Point3D, c2: Point3D },
    QuadraticBezierTo { control: Point3D },
    ScanLine { power_values: Arc<[u8]> },
}

#[derive(Clone, Debug)]
pub enum StateCmd {
    SetPower(f64),
    SetCutSpeed(i32),
    SetTravelSpeed(i32),
    Dwell(f64),
    EnableAirAssist,
    DisableAirAssist,
    SetLaser(Arc<str>),
    SetFrequency(i32),
    SetPulseWidth(f64),
}

#[derive(Clone, Debug)]
pub enum MarkerCmd {
    JobStart,
    JobEnd,
    LayerStart(Arc<str>),
    LayerEnd(Arc<str>),
    WorkpieceStart(Arc<str>),
    WorkpieceEnd(Arc<str>),
    OpsSectionStart {
        section_type: SectionType,
        workpiece_uid: Option<Arc<str>>,
    },
    OpsSectionEnd {
        section_type: SectionType,
        workpiece_uid: Option<Arc<str>>,
    },
}

#[derive(Clone, Debug)]
pub enum OpCategory {
    Moving { end: Point3D, cmd: MoveCmd },
    State(StateCmd),
    Marker(MarkerCmd),
}

#[derive(Clone, Debug)]
pub struct OpNode {
    pub category: OpCategory,
    pub state: Option<State>,
    pub extra_axes: Option<Arc<[(Axis, f64)]>>,
}

impl OpNode {
    pub fn move_to(
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpNode {
            category: OpCategory::Moving {
                end: (x, y, z),
                cmd: MoveCmd::MoveTo,
            },
            state: None,
            extra_axes: extra.map(Arc::from),
        }
    }

    pub fn line_to(
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpNode {
            category: OpCategory::Moving {
                end: (x, y, z),
                cmd: MoveCmd::LineTo,
            },
            state: None,
            extra_axes: extra.map(Arc::from),
        }
    }

    pub fn close_path(end: Point3D) -> Self {
        OpNode {
            category: OpCategory::Moving {
                end,
                cmd: MoveCmd::LineTo,
            },
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
        OpNode {
            category: OpCategory::Moving {
                end: (x, y, z),
                cmd: MoveCmd::ArcTo {
                    center: (i, j),
                    cw: clockwise,
                },
            },
            state: None,
            extra_axes: extra.map(Arc::from),
        }
    }

    pub fn bezier_to(
        c1: Point3D,
        c2: Point3D,
        end: Point3D,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpNode {
            category: OpCategory::Moving {
                end,
                cmd: MoveCmd::BezierTo { c1, c2 },
            },
            state: None,
            extra_axes: extra.map(Arc::from),
        }
    }

    pub fn quadratic_bezier_to(
        control: Point3D,
        end: Point3D,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpNode {
            category: OpCategory::Moving {
                end,
                cmd: MoveCmd::QuadraticBezierTo { control },
            },
            state: None,
            extra_axes: extra.map(Arc::from),
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
        OpNode {
            category: OpCategory::Moving {
                end: (x, y, z),
                cmd: MoveCmd::ScanLine {
                    power_values: Arc::from(pv),
                },
            },
            state: None,
            extra_axes: extra.map(Arc::from),
        }
    }

    pub fn set_power(power: f64) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetPower(power)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_cut_speed(speed: i32) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetCutSpeed(speed)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_travel_speed(speed: i32) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetTravelSpeed(speed)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn dwell(duration_ms: f64) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::Dwell(duration_ms)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn enable_air_assist(enabled: bool) -> Self {
        OpNode {
            category: OpCategory::State(if enabled {
                StateCmd::EnableAirAssist
            } else {
                StateCmd::DisableAirAssist
            }),
            state: None,
            extra_axes: None,
        }
    }

    pub fn disable_air_assist() -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::DisableAirAssist),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_laser(laser_uid: &str) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetLaser(Arc::from(
                laser_uid,
            ))),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_frequency(frequency: i32) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetFrequency(frequency)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_pulse_width(pulse_width: f64) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetPulseWidth(pulse_width)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn job_start() -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::JobStart),
            state: None,
            extra_axes: None,
        }
    }

    pub fn job_end() -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::JobEnd),
            state: None,
            extra_axes: None,
        }
    }

    pub fn layer_start(layer_uid: &str) -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::LayerStart(Arc::from(
                layer_uid,
            ))),
            state: None,
            extra_axes: None,
        }
    }

    pub fn layer_end(layer_uid: &str) -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::LayerEnd(Arc::from(
                layer_uid,
            ))),
            state: None,
            extra_axes: None,
        }
    }

    pub fn workpiece_start(workpiece_uid: &str) -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::WorkpieceStart(Arc::from(
                workpiece_uid,
            ))),
            state: None,
            extra_axes: None,
        }
    }

    pub fn workpiece_end(workpiece_uid: &str) -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::WorkpieceEnd(Arc::from(
                workpiece_uid,
            ))),
            state: None,
            extra_axes: None,
        }
    }

    pub fn ops_section_start(
        section_type: SectionType,
        workpiece_uid: &str,
    ) -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::OpsSectionStart {
                section_type,
                workpiece_uid: Some(Arc::from(workpiece_uid)),
            }),
            state: None,
            extra_axes: None,
        }
    }

    pub fn ops_section_end(section_type: SectionType) -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::OpsSectionEnd {
                section_type,
                workpiece_uid: None,
            }),
            state: None,
            extra_axes: None,
        }
    }

    pub fn command_type(&self) -> CommandType {
        match &self.category {
            OpCategory::Moving { cmd, .. } => match cmd {
                MoveCmd::MoveTo => CommandType::MoveTo,
                MoveCmd::LineTo => CommandType::LineTo,
                MoveCmd::ArcTo { .. } => CommandType::ArcTo,
                MoveCmd::BezierTo { .. } => CommandType::BezierTo,
                MoveCmd::QuadraticBezierTo { .. } => {
                    CommandType::QuadraticBezierTo
                }
                MoveCmd::ScanLine { .. } => CommandType::ScanLine,
            },
            OpCategory::State(cmd) => match cmd {
                StateCmd::SetPower(_) => CommandType::SetPower,
                StateCmd::SetCutSpeed(_) => CommandType::SetCutSpeed,
                StateCmd::SetTravelSpeed(_) => CommandType::SetTravelSpeed,
                StateCmd::Dwell(_) => CommandType::Dwell,
                StateCmd::EnableAirAssist => CommandType::EnableAirAssist,
                StateCmd::DisableAirAssist => CommandType::DisableAirAssist,
                StateCmd::SetLaser(_) => CommandType::SetLaser,
                StateCmd::SetFrequency(_) => CommandType::SetFrequency,
                StateCmd::SetPulseWidth(_) => CommandType::SetPulseWidth,
            },
            OpCategory::Marker(cmd) => match cmd {
                MarkerCmd::JobStart => CommandType::JobStart,
                MarkerCmd::JobEnd => CommandType::JobEnd,
                MarkerCmd::LayerStart(_) => CommandType::LayerStart,
                MarkerCmd::LayerEnd(_) => CommandType::LayerEnd,
                MarkerCmd::WorkpieceStart(_) => CommandType::WorkpieceStart,
                MarkerCmd::WorkpieceEnd(_) => CommandType::WorkpieceEnd,
                MarkerCmd::OpsSectionStart { .. } => {
                    CommandType::OpsSectionStart
                }
                MarkerCmd::OpsSectionEnd { .. } => CommandType::OpsSectionEnd,
            },
        }
    }

    pub fn is_moving(&self) -> bool {
        matches!(self.category, OpCategory::Moving { .. })
    }

    pub fn is_state_cmd(&self) -> bool {
        matches!(self.category, OpCategory::State(_))
    }

    pub fn is_marker(&self) -> bool {
        matches!(self.category, OpCategory::Marker(_))
    }

    pub fn end_point(&self) -> Point3D {
        if let OpCategory::Moving { end, .. } = &self.category {
            *end
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    pub fn as_moving(&self) -> Option<(&Point3D, &MoveCmd)> {
        if let OpCategory::Moving { end, cmd } = &self.category {
            Some((end, cmd))
        } else {
            None
        }
    }

    pub fn as_state(&self) -> Option<&StateCmd> {
        if let OpCategory::State(cmd) = &self.category {
            Some(cmd)
        } else {
            None
        }
    }

    pub fn as_marker(&self) -> Option<&MarkerCmd> {
        if let OpCategory::Marker(cmd) = &self.category {
            Some(cmd)
        } else {
            None
        }
    }

    pub fn set_endpoint(&mut self, end: Point3D) -> Option<Point3D> {
        if let OpCategory::Moving { end: ref mut e, .. } = &mut self.category {
            let old = *e;
            *e = end;
            Some(old)
        } else {
            None
        }
    }

    pub fn state(&self) -> Option<&State> {
        self.state.as_ref()
    }

    pub fn set_state(&mut self, st: State) {
        self.state = Some(st);
    }

    pub fn extra_axes(&self) -> Option<&[(Axis, f64)]> {
        self.extra_axes.as_deref()
    }

    pub fn set_extra_axes(&mut self, ea: Arc<[(Axis, f64)]>) {
        self.extra_axes = Some(ea);
    }
}
