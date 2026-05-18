#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandType {
    MoveTo = 1,
    LineTo = 2,
    ArcTo = 3,
    ScanLine = 4,
    Dwell = 5,
    BezierTo = 6,
    QuadraticBezierTo = 7,
    SetPower = 10,
    SetCutSpeed = 11,
    SetTravelSpeed = 12,
    EnableAirAssist = 13,
    DisableAirAssist = 14,
    SetLaser = 15,
    SetFrequency = 16,
    SetPulseWidth = 17,
    JobStart = 100,
    JobEnd = 101,
    LayerStart = 102,
    LayerEnd = 103,
    WorkpieceStart = 104,
    WorkpieceEnd = 105,
    OpsSectionStart = 106,
    OpsSectionEnd = 107,
}

impl TryFrom<u8> for CommandType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(CommandType::MoveTo),
            2 => Ok(CommandType::LineTo),
            3 => Ok(CommandType::ArcTo),
            4 => Ok(CommandType::ScanLine),
            5 => Ok(CommandType::Dwell),
            6 => Ok(CommandType::BezierTo),
            7 => Ok(CommandType::QuadraticBezierTo),
            10 => Ok(CommandType::SetPower),
            11 => Ok(CommandType::SetCutSpeed),
            12 => Ok(CommandType::SetTravelSpeed),
            13 => Ok(CommandType::EnableAirAssist),
            14 => Ok(CommandType::DisableAirAssist),
            15 => Ok(CommandType::SetLaser),
            16 => Ok(CommandType::SetFrequency),
            17 => Ok(CommandType::SetPulseWidth),
            100 => Ok(CommandType::JobStart),
            101 => Ok(CommandType::JobEnd),
            102 => Ok(CommandType::LayerStart),
            103 => Ok(CommandType::LayerEnd),
            104 => Ok(CommandType::WorkpieceStart),
            105 => Ok(CommandType::WorkpieceEnd),
            106 => Ok(CommandType::OpsSectionStart),
            107 => Ok(CommandType::OpsSectionEnd),
            _ => Err(format!("unknown command type: {}", value)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandCategory {
    Moving,
    State,
    Marker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionType {
    VectorOutline,
    RasterFill,
}

pub fn category(ct: CommandType) -> CommandCategory {
    match ct {
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
            assert_eq!(category(*ct), CommandCategory::Moving);
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
            assert_eq!(category(*ct), CommandCategory::State);
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
            assert_eq!(category(*ct), CommandCategory::Marker);
        }
    }

    #[test]
    fn test_try_from_valid() {
        assert_eq!(CommandType::try_from(1), Ok(CommandType::MoveTo));
        assert_eq!(CommandType::try_from(107), Ok(CommandType::OpsSectionEnd));
    }

    #[test]
    fn test_try_from_invalid() {
        assert!(CommandType::try_from(0).is_err());
        assert!(CommandType::try_from(8).is_err());
        assert!(CommandType::try_from(99).is_err());
        assert!(CommandType::try_from(200).is_err());
    }
}
