use thiserror::Error;

#[derive(Error, Debug)]
pub enum RaygeoError {
    #[error("unknown command type: {0}")]
    UnknownCommandType(i32),

    #[error("{0:?} combines multiple axes, expected a single axis")]
    MultiAxis(AxisRepr),

    #[error("{0:?} is not a single axis")]
    NotSingleAxis(AxisRepr),
}

#[derive(Debug)]
pub struct AxisRepr(pub u8);

impl std::fmt::Display for AxisRepr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Axis(0x{:02x})", self.0)
    }
}
