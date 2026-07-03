use std::fmt;

use thiserror::Error;

#[cfg(feature = "python")]
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};

/// Error type for all RayGeo operations.
#[derive(Error, Debug)]
pub enum RaygeoError {
    // ── SVG / Input ──────────────────────────────────────────────
    /// The SVG XML could not be parsed.
    #[error("failed to parse SVG: {0}")]
    SvgParseError(String),

    /// An SVG path `d` attribute contains malformed data.
    #[error("invalid SVG path data: {0}")]
    SvgInvalidPath(String),

    /// The SVG path data is empty or contains no usable commands.
    #[error("SVG path data is empty")]
    SvgEmptyPath,

    // ── Geometry ─────────────────────────────────────────────────
    /// Operation requires at least one command but the geometry is empty.
    #[error("geometry is empty")]
    EmptyGeometry,

    /// The geometry contains a degenerate segment (zero-length, etc.).
    #[error("degenerate geometry: {0}")]
    DegenerateGeometry(String),

    /// A polygon has fewer than 3 vertices.
    #[error("polygon must have at least 3 vertices, got {0}")]
    InvalidPolygon(usize),

    /// A contour operation failed.
    #[error("contour error: {0}")]
    ContourError(String),

    // ── Ops (command sequences) ──────────────────────────────────
    /// An unknown or unsupported command type was encountered.
    #[error("unknown command type: {0}")]
    UnknownCommandType(i32),

    /// An op-node or command sequence is in an invalid state.
    #[error("invalid command: {0}")]
    InvalidCommand(String),

    /// Serialization / deserialization of commands failed.
    #[error("serialization error: {0}")]
    SerializationError(String),

    // ── Axis ─────────────────────────────────────────────────────
    /// A multi-axis value was provided where a single axis was expected.
    #[error("{0:?} combines multiple axes, expected a single axis")]
    MultiAxis(AxisRepr),

    /// The value does not represent a single axis.
    #[error("{0:?} is not a single axis")]
    NotSingleAxis(AxisRepr),

    // ── Nesting ──────────────────────────────────────────────────
    /// A nesting / packing operation failed.
    #[error("nesting error: {0}")]
    NestingError(String),

    // ── Image ────────────────────────────────────────────────────
    /// An image processing operation failed.
    #[error("image error: {0}")]
    ImageError(String),

    // ── Clipping ─────────────────────────────────────────────────
    /// A clipping or region-subtraction operation failed.
    #[error("clipping error: {0}")]
    ClippingError(String),

    // ── Fitting ──────────────────────────────────────────────────
    /// Curve / primitive fitting failed.
    #[error("fitting error: {0}")]
    FittingError(String),

    // ── Adaptive Clearing ────────────────────────────────────────
    /// All resume strategies were exhausted and the pocket has not
    /// converged (uncut material remains).
    #[error("resume point not found: {0}")]
    ResumePointNotFound(String),

    /// A travel-path routing strategy could not find a collision-free
    /// path between two points.
    #[error("routing error: {0}")]
    RoutingError(String),

    // ── Internal ─────────────────────────────────────────────────
    /// An internal invariant was violated (should not happen).
    #[error("internal error: {0}")]
    InternalError(String),
}

/// A compact wrapper around a raw axis bitmask.
#[derive(Debug)]
pub struct AxisRepr(pub u8);

impl fmt::Display for AxisRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Axis(0x{:02x})", self.0)
    }
}

// Convenience type alias.
pub type RaygeoResult<T> = Result<T, RaygeoError>;

/// Converts a `RaygeoError` into a Python exception.
#[cfg(feature = "python")]
impl From<RaygeoError> for pyo3::PyErr {
    fn from(err: RaygeoError) -> Self {
        match &err {
            RaygeoError::SvgParseError(_)
            | RaygeoError::SvgInvalidPath(_)
            | RaygeoError::SvgEmptyPath => {
                PyValueError::new_err(err.to_string())
            }
            RaygeoError::EmptyGeometry | RaygeoError::DegenerateGeometry(_) => {
                PyRuntimeError::new_err(err.to_string())
            }
            RaygeoError::InvalidPolygon(_) | RaygeoError::ContourError(_) => {
                PyValueError::new_err(err.to_string())
            }
            RaygeoError::UnknownCommandType(_) => {
                PyTypeError::new_err(err.to_string())
            }
            RaygeoError::InvalidCommand(_)
            | RaygeoError::SerializationError(_) => {
                PyValueError::new_err(err.to_string())
            }
            RaygeoError::MultiAxis(_) | RaygeoError::NotSingleAxis(_) => {
                PyValueError::new_err(err.to_string())
            }
            RaygeoError::ResumePointNotFound(_)
            | RaygeoError::RoutingError(_)
            | RaygeoError::NestingError(_)
            | RaygeoError::ImageError(_)
            | RaygeoError::ClippingError(_)
            | RaygeoError::FittingError(_)
            | RaygeoError::InternalError(_) => {
                PyRuntimeError::new_err(err.to_string())
            }
        }
    }
}
