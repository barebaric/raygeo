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
    #[strum(serialize = "SET_COOLANT")]
    SetCoolant = 20,
    #[strum(serialize = "SET_AIR_ASSIST")]
    SetAirAssist = 21,
    #[strum(serialize = "SET_HEAD_COOLANT")]
    SetHeadCoolant = 22,
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
            | CommandType::SetCoolant
            | CommandType::SetAirAssist
            | CommandType::SetHeadCoolant => CommandCategory::State,
            CommandType::JobStart
            | CommandType::JobEnd
            | CommandType::LayerStart
            | CommandType::LayerEnd
            | CommandType::WorkpieceStart
            | CommandType::WorkpieceEnd
            | CommandType::OpsSectionStart
            | CommandType::OpsSectionEnd => CommandCategory::Marker,
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
    pub fn name(&self) -> &'static str {
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
}

impl SectionType {
    pub fn name(&self) -> &'static str {
        match self {
            SectionType::VectorOutline => "VECTOR_OUTLINE",
            SectionType::RasterFill => "RASTER_FILL",
        }
    }

    pub fn from_name(s: &str) -> Option<SectionType> {
        s.parse().ok()
    }
}
