//! Errors of the CorelDRAW reader.

use thiserror::Error;

/// Something that stopped the CorelDRAW reader.
#[derive(Debug, Error)]
pub enum Error {
    #[error("not a CorelDRAW file")]
    NotCdr,
    #[error("unsupported Corel file: {0}")]
    Unsupported(String),
    #[error("truncated data at offset {0}")]
    Truncated(usize),
}

pub type Result<T> = std::result::Result<T, Error>;
