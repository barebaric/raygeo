use num_enum::TryFromPrimitive;
use strum::{Display, EnumString};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive, EnumString, Display,
)]
#[repr(u8)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandType {
    #[strum(serialize = "MOVE_TO")]
    MoveTo = 1,
    #[strum(serialize = "LINE_TO")]
    LineTo = 2,
    #[strum(serialize = "ARC_TO")]
    ArcTo = 3,
    #[strum(serialize = "SCAN_LINE")]
    ScanLine = 4,
    Dwell = 5,
    #[strum(serialize = "BEZIER_TO")]
    BezierTo = 6,
    #[strum(serialize = "QUADRATIC_BEZIER_TO")]
    QuadraticBezierTo = 7,
    #[strum(serialize = "SET_POWER")]
    SetPower = 10,
    #[strum(serialize = "SET_FEED_RATE")]
    SetFeedRate = 11,
    #[strum(serialize = "SET_RAPID_RATE")]
    SetRapidRate = 12,
    #[strum(serialize = "SET_HEAD")]
    SetHead = 15,
    #[strum(serialize = "SET_FREQUENCY")]
    SetFrequency = 16,
    #[strum(serialize = "SET_PULSE_WIDTH")]
    SetPulseWidth = 17,
    #[strum(serialize = "SET_SPINDLE_RPM")]
    SetSpindleRpm = 18,
    #[strum(serialize = "SET_SPINDLE_DIRECTION")]
    SetSpindleDirection = 24,
    #[strum(serialize = "SET_SYNC_PITCH")]
    SetSyncPitch = 25,
    #[strum(serialize = "SET_COOLANT")]
    SetCoolant = 20,
    #[strum(serialize = "SET_AIR_ASSIST")]
    SetAirAssist = 21,
    #[strum(serialize = "SET_HEAD_COOLANT")]
    SetHeadCoolant = 22,
    #[strum(serialize = "SET_POWER_MODE")]
    SetPowerMode = 23,
    #[strum(serialize = "CUSTOM")]
    Custom = 30,
    #[strum(serialize = "JOB_START")]
    JobStart = 100,
    #[strum(serialize = "JOB_END")]
    JobEnd = 101,
    #[strum(serialize = "LAYER_START")]
    LayerStart = 102,
    #[strum(serialize = "LAYER_END")]
    LayerEnd = 103,
    #[strum(serialize = "WORKPIECE_START")]
    WorkpieceStart = 104,
    #[strum(serialize = "WORKPIECE_END")]
    WorkpieceEnd = 105,
    #[strum(serialize = "OPS_SECTION_START")]
    OpsSectionStart = 106,
    #[strum(serialize = "OPS_SECTION_END")]
    OpsSectionEnd = 107,
    #[strum(serialize = "STATE_BLOCK_START")]
    StateBlockStart = 108,
    #[strum(serialize = "STATE_BLOCK_END")]
    StateBlockEnd = 109,
}

impl CommandType {
    pub fn from_name(s: &str) -> Option<CommandType> {
        s.parse().ok()
    }

    pub fn category(&self) -> CommandCategory {
        match self {
            CommandType::MoveTo
            | CommandType::LineTo
            | CommandType::ArcTo
            | CommandType::BezierTo
            | CommandType::QuadraticBezierTo
            | CommandType::ScanLine => CommandCategory::Moving,
            CommandType::Dwell
            | CommandType::SetPower
            | CommandType::SetFeedRate
            | CommandType::SetRapidRate
            | CommandType::SetFrequency
            | CommandType::SetPulseWidth
            | CommandType::SetHead
            | CommandType::SetSpindleRpm
            | CommandType::SetSpindleDirection
            | CommandType::SetSyncPitch
            | CommandType::SetCoolant
            | CommandType::SetAirAssist
            | CommandType::SetHeadCoolant
            | CommandType::SetPowerMode
            | CommandType::Custom => CommandCategory::State,
            CommandType::JobStart
            | CommandType::JobEnd
            | CommandType::LayerStart
            | CommandType::LayerEnd
            | CommandType::WorkpieceStart
            | CommandType::WorkpieceEnd
            | CommandType::OpsSectionStart
            | CommandType::OpsSectionEnd
            | CommandType::StateBlockStart
            | CommandType::StateBlockEnd => CommandCategory::Marker,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandCategory {
    Moving,
    State,
    Marker,
}

impl CommandCategory {
    pub fn name(&self) -> &str {
        match self {
            CommandCategory::Moving => "MOVING",
            CommandCategory::State => "STATE",
            CommandCategory::Marker => "MARKER",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum SectionType {
    VectorOutline,
    RasterFill,
    /// One hole-making cycle (drill, peck drill, countersink), expanded
    /// into explicit moves. Tags the moves so encoders or downstream
    /// consumers can substitute a canned cycle (G81/G82/G73/G83).
    DrillCycle,
    /// One rigid-tapping cycle (right- or left-hand, optionally pecked),
    /// expanded into spindle-synchronized moves. Tags the moves so
    /// encoders or downstream consumers can substitute a canned cycle
    /// (G84/G74).
    TappingCycle,
}

impl SectionType {
    pub fn name(&self) -> &str {
        match self {
            SectionType::VectorOutline => "VECTOR_OUTLINE",
            SectionType::RasterFill => "RASTER_FILL",
            SectionType::DrillCycle => "DRILL_CYCLE",
            SectionType::TappingCycle => "TAPPING_CYCLE",
        }
    }

    pub fn from_name(s: &str) -> Option<SectionType> {
        s.parse().ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, Display)]
#[repr(u8)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum RasterMode {
    VariablePower = 1,
    ConstantPower = 2,
    DepthMap = 3,
    Dither = 4,
}

impl RasterMode {
    pub fn name(&self) -> &str {
        match self {
            RasterMode::VariablePower => "VARIABLE_POWER",
            RasterMode::ConstantPower => "CONSTANT_POWER",
            RasterMode::DepthMap => "DEPTH_MAP",
            RasterMode::Dither => "DITHER",
        }
    }

    pub fn from_name(s: &str) -> Option<RasterMode> {
        s.parse().ok()
    }
}
