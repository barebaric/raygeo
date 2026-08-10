use std::sync::Arc;

use super::axis::Axis;
use super::enums::{CommandType, RasterMode, SectionType};
use super::state::{AirAssistMode, CoolantMode, HeadCoolantMode, State};
use crate::error::RaygeoError;
use crate::geo::types::{Point, Point3D};

/// Position and heading of the cutting tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolPose {
    pub pos: Point3D,
    pub heading: f64,
}

/// Milling rotational direction. All cutting moves respect this
/// for the whole run.  Resume strategies use it to determine the
/// frontier walk direction, and `probe_step` uses it to vet resume
/// positions with one-sided deflection bounds.  The main stepper
/// loop uses symmetric bounds and relies on heading momentum +
/// cleared-area geometry to maintain the rotational bias.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CutDirection {
    /// Clockwise.
    Cw,
    /// Counter-clockwise.
    #[default]
    Ccw,
}

impl CutDirection {
    /// One-sided angle bounds for the adaptive step solver, relative to
    /// the heading.  `max_deflection` is the magnitude of the allowed
    /// turn.  Returns `(angle_min, angle_max)` in radians.
    ///
    /// When walking CCW around the cleared area, uncut material is on
    /// the RIGHT of the heading, so the tool must deflect right
    /// (negative angle).  CW cutting is the mirror.
    pub fn angle_bounds(&self, max_deflection: f64) -> (f64, f64) {
        match self {
            CutDirection::Cw => (0.0, max_deflection),
            CutDirection::Ccw => (-max_deflection, 0.0),
        }
    }

    /// Sign of the steering angle the tool should prefer to honour
    /// this rotational direction.
    ///
    /// * `Cw`  → `+1.0` (deflect left / positive angle)
    /// * `Ccw` → `−1.0` (deflect right / negative angle)
    pub fn sign(&self) -> f64 {
        match self {
            CutDirection::Cw => 1.0,
            CutDirection::Ccw => -1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArcToData {
    pub center: Point,
    pub cw: bool,
}

#[derive(Clone, Debug)]
pub struct BezierToData {
    pub control1: Point3D,
    pub control2: Point3D,
}

#[derive(Clone, Debug)]
pub enum MoveCmd {
    MoveTo,
    LineTo,
    ArcTo(Box<ArcToData>),
    BezierTo(Box<BezierToData>),
    QuadraticBezierTo { control: Point3D },
    ScanLine { power_values: Arc<Vec<u8>> },
}

#[derive(Clone, Debug)]
pub enum StateCmd {
    SetPower(f64),
    SetFeedRate(i32),
    SetRapidRate(i32),
    Dwell(f64),
    SetHead(Arc<str>),
    SetFrequency(i32),
    SetPulseWidth(f64),
    SetSpindleRpm(u32),
    SetCoolant(CoolantMode),
    SetAirAssist(AirAssistMode),
    SetHeadCoolant(HeadCoolantMode),
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
        raster_mode: Option<RasterMode>,
    },
    OpsSectionEnd {
        section_type: SectionType,
        workpiece_uid: Option<Arc<str>>,
        raster_mode: Option<RasterMode>,
    },
    StateBlockStart {
        name: Option<Arc<str>>,
    },
    StateBlockEnd,
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
    pub state: Option<Box<State>>,
    pub extra_axes: Option<Arc<Vec<(Axis, f64)>>>,
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
                end: Point3D::new(x, y, z),
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
                end: Point3D::new(x, y, z),
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
                end: Point3D::new(x, y, z),
                cmd: MoveCmd::ArcTo(Box::new(ArcToData {
                    center: Point::new(i, j),
                    cw: clockwise,
                })),
            },
            state: None,
            extra_axes: extra.map(Arc::from),
        }
    }

    pub fn bezier_to(
        control1: Point3D,
        control2: Point3D,
        end: Point3D,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpNode {
            category: OpCategory::Moving {
                end,
                cmd: MoveCmd::BezierTo(Box::new(BezierToData {
                    control1,
                    control2,
                })),
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
        power_values: Vec<u8>,
        extra: Option<Vec<(Axis, f64)>>,
    ) -> Self {
        OpNode {
            category: OpCategory::Moving {
                end: Point3D::new(x, y, z),
                cmd: MoveCmd::ScanLine {
                    power_values: Arc::from(power_values),
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

    pub fn set_feed_rate(feed_rate: i32) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetFeedRate(feed_rate)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_rapid_rate(rapid_rate: i32) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetRapidRate(rapid_rate)),
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

    pub fn set_head(head_uid: &str) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetHead(Arc::from(head_uid))),
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

    pub fn set_spindle_rpm(rpm: u32) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetSpindleRpm(rpm)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_coolant(mode: CoolantMode) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetCoolant(mode)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_air_assist(mode: AirAssistMode) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetAirAssist(mode)),
            state: None,
            extra_axes: None,
        }
    }

    pub fn set_head_coolant(mode: HeadCoolantMode) -> Self {
        OpNode {
            category: OpCategory::State(StateCmd::SetHeadCoolant(mode)),
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
        raster_mode: Option<RasterMode>,
    ) -> Result<Self, RaygeoError> {
        match (section_type, raster_mode) {
            (SectionType::VectorOutline, Some(mode)) => {
                return Err(RaygeoError::InvalidCommand(format!(
                    "VectorOutline section with raster_mode={mode:?} is invalid"
                )));
            }
            (SectionType::RasterFill, None) => {
                return Err(RaygeoError::InvalidCommand(
                    "RasterFill section without raster_mode is invalid".into(),
                ));
            }
            _ => {}
        }
        Ok(OpNode {
            category: OpCategory::Marker(MarkerCmd::OpsSectionStart {
                section_type,
                workpiece_uid: Some(Arc::from(workpiece_uid)),
                raster_mode,
            }),
            state: None,
            extra_axes: None,
        })
    }

    pub fn ops_section_end(
        section_type: SectionType,
        raster_mode: Option<RasterMode>,
    ) -> Result<Self, RaygeoError> {
        match (section_type, raster_mode) {
            (SectionType::VectorOutline, Some(mode)) => {
                return Err(RaygeoError::InvalidCommand(format!(
                    "VectorOutline section end with raster_mode={mode:?} is invalid"
                )));
            }
            (SectionType::RasterFill, None) => {
                return Err(RaygeoError::InvalidCommand(
                    "RasterFill section end without raster_mode is invalid"
                        .into(),
                ));
            }
            _ => {}
        }
        Ok(OpNode {
            category: OpCategory::Marker(MarkerCmd::OpsSectionEnd {
                section_type,
                workpiece_uid: None,
                raster_mode,
            }),
            state: None,
            extra_axes: None,
        })
    }

    pub fn state_block_start(name: Option<&str>) -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::StateBlockStart {
                name: name.map(Arc::from),
            }),
            state: None,
            extra_axes: None,
        }
    }

    pub fn state_block_end() -> Self {
        OpNode {
            category: OpCategory::Marker(MarkerCmd::StateBlockEnd),
            state: None,
            extra_axes: None,
        }
    }

    pub fn command_type(&self) -> CommandType {
        match &self.category {
            OpCategory::Moving { cmd, .. } => match cmd {
                MoveCmd::MoveTo => CommandType::MoveTo,
                MoveCmd::LineTo => CommandType::LineTo,
                MoveCmd::ArcTo(_) => CommandType::ArcTo,
                MoveCmd::BezierTo(_) => CommandType::BezierTo,
                MoveCmd::QuadraticBezierTo { .. } => {
                    CommandType::QuadraticBezierTo
                }
                MoveCmd::ScanLine { .. } => CommandType::ScanLine,
            },
            OpCategory::State(cmd) => match cmd {
                StateCmd::SetPower(_) => CommandType::SetPower,
                StateCmd::SetFeedRate(_) => CommandType::SetFeedRate,
                StateCmd::SetRapidRate(_) => CommandType::SetRapidRate,
                StateCmd::Dwell(_) => CommandType::Dwell,
                StateCmd::SetHead(_) => CommandType::SetHead,
                StateCmd::SetFrequency(_) => CommandType::SetFrequency,
                StateCmd::SetPulseWidth(_) => CommandType::SetPulseWidth,
                StateCmd::SetSpindleRpm(_) => CommandType::SetSpindleRpm,
                StateCmd::SetCoolant(_) => CommandType::SetCoolant,
                StateCmd::SetAirAssist(_) => CommandType::SetAirAssist,
                StateCmd::SetHeadCoolant(_) => CommandType::SetHeadCoolant,
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
                MarkerCmd::StateBlockStart { .. } => {
                    CommandType::StateBlockStart
                }
                MarkerCmd::StateBlockEnd => CommandType::StateBlockEnd,
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
            Point3D::new(0.0, 0.0, 0.0)
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
        self.state.as_deref()
    }

    pub fn set_state(&mut self, st: State) {
        self.state = Some(Box::new(st));
    }

    pub fn extra_axes(&self) -> Option<&[(Axis, f64)]> {
        self.extra_axes.as_ref().map(|v| v.as_slice())
    }

    pub fn set_extra_axes(&mut self, ea: Arc<Vec<(Axis, f64)>>) {
        self.extra_axes = Some(ea);
    }

    pub fn clear_extra_axes(&mut self) {
        self.extra_axes = None;
    }
}
