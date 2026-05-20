use std::sync::Arc;

use super::axis::Axis;
use super::enums::{CommandType, SectionType};
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
    pub extra_axes: Option<Arc<[(Axis, f64)]>>,
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
            extra_axes: extra.map(|v| Arc::from(v)),
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
            extra_axes: extra.map(|v| Arc::from(v)),
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
            extra_axes: extra.map(|v| Arc::from(v)),
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
            extra_axes: extra.map(|v| Arc::from(v)),
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
            extra_axes: extra.map(|v| Arc::from(v)),
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
            extra_axes: extra.map(|v| Arc::from(v)),
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
