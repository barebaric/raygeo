//! Barcode and QR code encoding.
//!
//! The module turns text into a module matrix (dark and light squares)
//! for QR Code, Code 128 and EAN-13 symbols. It also renders that matrix
//! to a millimetre-space [`Geometry`](crate::geo::Geometry) with the
//! human-readable text, quiet zone and inverted variant handled here.
//!
//! Encoding is deliberately separate from styling: [`Encoded`] carries
//! the symbology's modules and metadata, while
//! [`geometry::to_geometry`] applies a [`geometry::BarcodeStyle`].
//!
//! QR encoding uses the `qrcodegen` crate; Code 128 and EAN-13 are
//! implemented directly so their exact code-set switching, check digit
//! and guard-bar behavior stay under our control.

pub mod code128;
pub mod ean13;
pub mod geometry;
pub mod qr;

use thiserror::Error;

/// The barcode symbologies this module can encode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbology {
    /// QR Code.
    Qr,
    /// Code 128.
    Code128,
    /// EAN-13.
    Ean13,
}

impl Symbology {
    /// The stable serialization value used by host applications.
    pub fn value(self) -> &'static str {
        match self {
            Symbology::Qr => "qr",
            Symbology::Code128 => "code128",
            Symbology::Ean13 => "ean13",
        }
    }

    /// Parses a serialization value produced by [`value`](Self::value).
    pub fn from_value(value: &str) -> Option<Self> {
        match value {
            "qr" => Some(Symbology::Qr),
            "code128" => Some(Symbology::Code128),
            "ean13" => Some(Symbology::Ean13),
            _ => None,
        }
    }
}

/// The QR Code error-correction level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrErrorCorrection {
    /// Low, roughly 7% recovery.
    Low,
    /// Medium, roughly 15% recovery.
    Medium,
    /// Quartile, roughly 25% recovery.
    Quartile,
    /// High, roughly 30% recovery.
    High,
}

impl QrErrorCorrection {
    /// The stable serialization value used by host applications.
    pub fn value(self) -> &'static str {
        match self {
            QrErrorCorrection::Low => "L",
            QrErrorCorrection::Medium => "M",
            QrErrorCorrection::Quartile => "Q",
            QrErrorCorrection::High => "H",
        }
    }

    /// Parses a serialization value produced by [`value`](Self::value).
    pub fn from_value(value: &str) -> Option<Self> {
        match value {
            "L" => Some(QrErrorCorrection::Low),
            "M" => Some(QrErrorCorrection::Medium),
            "Q" => Some(QrErrorCorrection::Quartile),
            "H" => Some(QrErrorCorrection::High),
            _ => None,
        }
    }
}

/// An encoded symbol: its modules and the metadata needed to draw it.
///
/// `modules` is row-major, with `modules[0]` at the top of the symbol
/// and `true` for a dark module. 1D symbologies use a single row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoded {
    /// The symbology that produced this symbol.
    pub symbology: Symbology,
    /// The module matrix, row-major, `true` for dark.
    pub modules: Vec<Vec<bool>>,
    /// Human-readable text; EAN-13 includes the check digit.
    pub text: String,
    /// Columns covered by EAN-13 guard bars.
    pub guards: Vec<usize>,
}

impl Encoded {
    /// The symbol width in modules.
    pub fn width(&self) -> usize {
        self.modules.first().map_or(0, Vec::len)
    }

    /// The symbol height in modules.
    pub fn height(&self) -> usize {
        self.modules.len()
    }

    /// Whether the given column is an EAN-13 guard bar.
    pub fn is_guard(&self, column: usize) -> bool {
        self.guards.contains(&column)
    }
}

/// Errors raised while encoding a symbol or building its geometry.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BarcodeError {
    /// The content is empty.
    #[error("barcode content is empty")]
    EmptyContent,
    /// Code 128 cannot represent the character.
    #[error("Code 128 cannot encode {0:?}; use ASCII 32-127")]
    Code128Character(char),
    /// EAN-13 requires exactly 12 or 13 digits.
    #[error("EAN-13 needs 12 or 13 digits")]
    Ean13Length,
    /// EAN-13 content contains a non-digit.
    #[error("EAN-13 content must contain digits only")]
    Ean13Digits,
    /// The supplied EAN-13 check digit does not match.
    #[error("wrong EAN-13 check digit {actual}, expected {expected}")]
    Ean13CheckDigit {
        /// The check digit in the supplied content.
        actual: u8,
        /// The check digit computed from the first twelve digits.
        expected: u8,
    },
    /// The content exceeds the largest QR code version.
    #[error("content is too long for a QR code")]
    QrTooLong,
    /// The module size is zero, negative or not finite.
    #[error("module size must be a positive finite number")]
    InvalidModuleSize,
    /// The bar height is negative or not finite.
    #[error("bar height must be a non-negative finite number")]
    InvalidBarHeight,
    /// The quiet zone is unreasonably large.
    #[error("quiet zone must be between 0 and 1000 modules")]
    InvalidQuietZone,
}

/// Encodes `text` in the requested symbology.
///
/// `error_correction` is used by QR only and ignored by the 1D
/// symbologies.
pub fn encode(
    text: &str,
    symbology: Symbology,
    error_correction: QrErrorCorrection,
) -> Result<Encoded, BarcodeError> {
    match symbology {
        Symbology::Qr => qr::encode(text, error_correction),
        Symbology::Code128 => code128::encode(text),
        Symbology::Ean13 => ean13::encode(text),
    }
}

/// The minimum quiet zone for a symbology, in modules.
pub fn min_quiet_zone(symbology: Symbology) -> u32 {
    match symbology {
        Symbology::Qr => 4,
        Symbology::Code128 => 10,
        Symbology::Ean13 => 9,
    }
}

#[cfg(feature = "python")]
impl From<BarcodeError> for pyo3::PyErr {
    fn from(error: BarcodeError) -> Self {
        pyo3::exceptions::PyValueError::new_err(error.to_string())
    }
}
