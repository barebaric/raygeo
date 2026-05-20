use num_enum::TryFromPrimitive;
use strum::{Display, EnumString};

#[derive(Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive, EnumString, Display)]
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
    #[strum(serialize = "SET_CUT_SPEED")]
    SetCutSpeed = 11,
    #[strum(serialize = "SET_TRAVEL_SPEED")]
    SetTravelSpeed = 12,
    #[strum(serialize = "ENABLE_AIR_ASSIST")]
    EnableAirAssist = 13,
    #[strum(serialize = "DISABLE_AIR_ASSIST")]
    DisableAirAssist = 14,
    #[strum(serialize = "SET_LASER")]
    SetLaser = 15,
    #[strum(serialize = "SET_FREQUENCY")]
    SetFrequency = 16,
    #[strum(serialize = "SET_PULSE_WIDTH")]
    SetPulseWidth = 17,
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
    pub fn name(&self) -> &'static str {
        match self {
            CommandType::MoveTo => "MOVE_TO",
            CommandType::LineTo => "LINE_TO",
            CommandType::ArcTo => "ARC_TO",
            CommandType::ScanLine => "SCAN_LINE",
            CommandType::Dwell => "DWELL",
            CommandType::BezierTo => "BEZIER_TO",
            CommandType::QuadraticBezierTo => "QUADRATIC_BEZIER_TO",
            CommandType::SetPower => "SET_POWER",
            CommandType::SetCutSpeed => "SET_CUT_SPEED",
            CommandType::SetTravelSpeed => "SET_TRAVEL_SPEED",
            CommandType::EnableAirAssist => "ENABLE_AIR_ASSIST",
            CommandType::DisableAirAssist => "DISABLE_AIR_ASSIST",
            CommandType::SetLaser => "SET_LASER",
            CommandType::SetFrequency => "SET_FREQUENCY",
            CommandType::SetPulseWidth => "SET_PULSE_WIDTH",
            CommandType::JobStart => "JOB_START",
            CommandType::JobEnd => "JOB_END",
            CommandType::LayerStart => "LAYER_START",
            CommandType::LayerEnd => "LAYER_END",
            CommandType::WorkpieceStart => "WORKPIECE_START",
            CommandType::WorkpieceEnd => "WORKPIECE_END",
            CommandType::OpsSectionStart => "OPS_SECTION_START",
            CommandType::OpsSectionEnd => "OPS_SECTION_END",
        }
    }

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
            | CommandType::SetCutSpeed
            | CommandType::SetTravelSpeed
            | CommandType::SetFrequency
            | CommandType::SetPulseWidth
            | CommandType::EnableAirAssist
            | CommandType::DisableAirAssist
            | CommandType::SetLaser => CommandCategory::State,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_type_values() {
        assert_eq!(CommandType::MoveTo as u8, 1);
        assert_eq!(CommandType::LineTo as u8, 2);
        assert_eq!(CommandType::ArcTo as u8, 3);
        assert_eq!(CommandType::ScanLine as u8, 4);
        assert_eq!(CommandType::Dwell as u8, 5);
        assert_eq!(CommandType::BezierTo as u8, 6);
        assert_eq!(CommandType::QuadraticBezierTo as u8, 7);
        assert_eq!(CommandType::SetPower as u8, 10);
        assert_eq!(CommandType::SetCutSpeed as u8, 11);
        assert_eq!(CommandType::SetTravelSpeed as u8, 12);
        assert_eq!(CommandType::EnableAirAssist as u8, 13);
        assert_eq!(CommandType::DisableAirAssist as u8, 14);
        assert_eq!(CommandType::SetLaser as u8, 15);
        assert_eq!(CommandType::SetFrequency as u8, 16);
        assert_eq!(CommandType::SetPulseWidth as u8, 17);
        assert_eq!(CommandType::JobStart as u8, 100);
        assert_eq!(CommandType::JobEnd as u8, 101);
        assert_eq!(CommandType::LayerStart as u8, 102);
        assert_eq!(CommandType::LayerEnd as u8, 103);
        assert_eq!(CommandType::WorkpieceStart as u8, 104);
        assert_eq!(CommandType::WorkpieceEnd as u8, 105);
        assert_eq!(CommandType::OpsSectionStart as u8, 106);
        assert_eq!(CommandType::OpsSectionEnd as u8, 107);
    }

    #[test]
    fn test_category_moving() {
        for ct in &[
            CommandType::MoveTo,
            CommandType::LineTo,
            CommandType::ArcTo,
            CommandType::BezierTo,
            CommandType::QuadraticBezierTo,
            CommandType::ScanLine,
        ] {
            assert_eq!(ct.category(), CommandCategory::Moving);
        }
    }

    #[test]
    fn test_category_state() {
        for ct in &[
            CommandType::Dwell,
            CommandType::SetPower,
            CommandType::SetCutSpeed,
            CommandType::SetTravelSpeed,
            CommandType::SetFrequency,
            CommandType::SetPulseWidth,
            CommandType::EnableAirAssist,
            CommandType::DisableAirAssist,
            CommandType::SetLaser,
        ] {
            assert_eq!(ct.category(), CommandCategory::State);
        }
    }

    #[test]
    fn test_category_marker() {
        for ct in &[
            CommandType::JobStart,
            CommandType::JobEnd,
            CommandType::LayerStart,
            CommandType::LayerEnd,
            CommandType::WorkpieceStart,
            CommandType::WorkpieceEnd,
            CommandType::OpsSectionStart,
            CommandType::OpsSectionEnd,
        ] {
            assert_eq!(ct.category(), CommandCategory::Marker);
        }
    }

    #[test]
    fn test_try_from_valid() {
        assert_eq!(
            CommandType::try_from(1),
            Ok(CommandType::MoveTo)
        );
        assert_eq!(
            CommandType::try_from(107),
            Ok(CommandType::OpsSectionEnd)
        );
    }

    #[test]
    fn test_try_from_invalid() {
        assert!(CommandType::try_from(0).is_err());
        assert!(CommandType::try_from(8).is_err());
        assert!(CommandType::try_from(99).is_err());
        assert!(CommandType::try_from(200).is_err());
    }

    #[test]
    fn test_from_name_roundtrip() {
        assert_eq!(CommandType::from_name("MOVE_TO"), Some(CommandType::MoveTo));
        assert_eq!(CommandType::from_name("LINE_TO"), Some(CommandType::LineTo));
        assert_eq!(CommandType::from_name("ARC_TO"), Some(CommandType::ArcTo));
        assert_eq!(CommandType::from_name("SCAN_LINE"), Some(CommandType::ScanLine));
        assert_eq!(CommandType::from_name("BEZIER_TO"), Some(CommandType::BezierTo));
        assert_eq!(CommandType::from_name("SET_POWER"), Some(CommandType::SetPower));
        assert_eq!(CommandType::from_name("JOB_START"), Some(CommandType::JobStart));
        assert_eq!(CommandType::from_name("OPS_SECTION_END"), Some(CommandType::OpsSectionEnd));
        assert_eq!(CommandType::from_name("INVALID"), None);
    }

    #[test]
    fn test_section_type_from_name() {
        assert_eq!(SectionType::from_name("VECTOR_OUTLINE"), Some(SectionType::VectorOutline));
        assert_eq!(SectionType::from_name("RASTER_FILL"), Some(SectionType::RasterFill));
        assert_eq!(SectionType::from_name("INVALID"), None);
    }

    #[test]
    fn test_name_method() {
        assert_eq!(CommandType::MoveTo.name(), "MOVE_TO");
        assert_eq!(CommandType::ArcTo.name(), "ARC_TO");
        assert_eq!(CommandType::ScanLine.name(), "SCAN_LINE");
        assert_eq!(SectionType::VectorOutline.name(), "VECTOR_OUTLINE");
        assert_eq!(SectionType::RasterFill.name(), "RASTER_FILL");
    }
}
