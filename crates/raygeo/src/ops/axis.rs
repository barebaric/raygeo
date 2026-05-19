use crate::error::{AxisRepr, RaygeoError};

use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Axis: u8 {
        const X = 0x01;
        const Y = 0x02;
        const Z = 0x04;
        const A = 0x08;
        const B = 0x10;
        const C = 0x20;
        const U = 0x40;
    }
}

impl Axis {
    pub fn assert_single_axis(self) -> Result<(), RaygeoError> {
        if self.bits().count_ones() != 1 {
            return Err(RaygeoError::MultiAxis(AxisRepr(self.bits())));
        }
        Ok(())
    }

    pub fn label(self) -> Result<&'static str, RaygeoError> {
        match self {
            Axis::X => Ok("x"),
            Axis::Y => Ok("y"),
            Axis::Z => Ok("z"),
            Axis::A => Ok("a"),
            Axis::B => Ok("b"),
            Axis::C => Ok("c"),
            Axis::U => Ok("u"),
            _ => Err(RaygeoError::NotSingleAxis(AxisRepr(self.bits()))),
        }
    }

    pub fn name(self) -> Result<&'static str, RaygeoError> {
        match self {
            Axis::X => Ok("X"),
            Axis::Y => Ok("Y"),
            Axis::Z => Ok("Z"),
            Axis::A => Ok("A"),
            Axis::B => Ok("B"),
            Axis::C => Ok("C"),
            Axis::U => Ok("U"),
            _ => Err(RaygeoError::NotSingleAxis(AxisRepr(self.bits()))),
        }
    }

    pub fn from_str_name(s: &str) -> Option<Axis> {
        match s {
            "X" => Some(Axis::X),
            "Y" => Some(Axis::Y),
            "Z" => Some(Axis::Z),
            "A" => Some(Axis::A),
            "B" => Some(Axis::B),
            "C" => Some(Axis::C),
            "U" => Some(Axis::U),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_axis() {
        assert!(Axis::X.assert_single_axis().is_ok());
        assert!(Axis::Y.assert_single_axis().is_ok());
        assert!(Axis::Z.assert_single_axis().is_ok());
        assert!(Axis::A.assert_single_axis().is_ok());
        assert!(Axis::B.assert_single_axis().is_ok());
        assert!(Axis::C.assert_single_axis().is_ok());
        assert!(Axis::U.assert_single_axis().is_ok());
    }

    #[test]
    fn test_combined_axis_rejected() {
        assert!((Axis::X | Axis::Y).assert_single_axis().is_err());
        assert!((Axis::X | Axis::Y | Axis::Z).assert_single_axis().is_err());
    }

    #[test]
    fn test_label() {
        assert_eq!(Axis::X.label().unwrap(), "x");
        assert_eq!(Axis::Y.label().unwrap(), "y");
        assert_eq!(Axis::Z.label().unwrap(), "z");
    }

    #[test]
    fn test_label_combined_returns_error() {
        assert!((Axis::X | Axis::Y).label().is_err());
    }

    #[test]
    fn test_bitflags_values() {
        assert_eq!(Axis::X.bits(), 0x01);
        assert_eq!(Axis::Y.bits(), 0x02);
        assert_eq!(Axis::Z.bits(), 0x04);
        assert_eq!(Axis::A.bits(), 0x08);
        assert_eq!(Axis::B.bits(), 0x10);
        assert_eq!(Axis::C.bits(), 0x20);
        assert_eq!(Axis::U.bits(), 0x40);
    }
}
