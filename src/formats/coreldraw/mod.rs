//! CorelDRAW (`.cdr`) import.
//!
//! The file is parsed natively: RIFF and ZIP containers, compressed chunk
//! lists, object records and their transforms are all read in Rust, so the
//! reader needs no external tool and no C library, and builds for wasm.
//!
//! Supported are the record format of CorelDRAW 7 through X7 (versions 7
//! to 17) in a plain RIFF file and in a ZIP package, including the
//! external data streams of version 16 and later. Older files are read
//! with the 16-bit encodings but are not verified; CMX and the
//! CorelDRAW 1/2 (`WL`) format are rejected with an error.
//!
//! Drawing geometry and text are extracted, flattened into layers in
//! inches with the page origin at the bottom-left corner. Text is drawn
//! as glyph outlines with the system's closest font: CorelDRAW's embedded
//! fonts are not used, so glyph shapes follow the installed fonts.
//! Bitmaps, fills, outlines and object kinds the reader does not know are
//! not converted; what cannot be read is reported in
//! [`CdrDocument::issues`].
//!
//! - `container`: detects a bare RIFF file or a ZIP package
//! - `riff`: walks the chunk tree, including compressed lists and
//!   redirect records
//! - `loda`: the argument table of an object record
//! - `zip`: reads members of a ZIP package
//! - `parse`: extracts the page geometry

pub mod detect;

pub(crate) mod container;
pub(crate) mod error;
pub(crate) mod loda;
pub(crate) mod parse;
pub(crate) mod riff;
pub(crate) mod stlt;
pub(crate) mod text;
pub(crate) mod zip;

use crate::error::{RaygeoError, RaygeoResult};
use crate::geo::geometry::Geometry;
use error::Error;

/// A CorelDRAW page and its drawing geometry.
#[derive(Debug, Clone)]
pub struct CdrDocument {
    /// The page width in inches.
    pub page_width: f64,
    /// The page height in inches.
    pub page_height: f64,
    /// The number of non-master pages in the document.
    pub page_count: usize,
    /// The layers of the first page, bottom to top, each with its geometry
    /// in inches relative to the page's bottom-left corner.
    pub layers: Vec<(String, Geometry)>,
    /// Content the reader did not convert, one note per category.
    pub issues: Vec<String>,
}

impl CdrDocument {
    /// Multiplier converting the native unit (inches) to millimetres.
    pub const UNITS_TO_MM: f64 = 25.4;
}

/// Reads a CorelDRAW drawing into the geometry of its first page.
///
/// Errors are [`RaygeoError::CdrUnsupportedFormat`] when the data is not a
/// CorelDRAW document, and [`RaygeoError::CdrParseError`] when the
/// container, a record or an argument table cannot be read.
pub fn parse(data: &[u8]) -> RaygeoResult<CdrDocument> {
    parse::parse(data).map_err(map_error)
}

fn map_error(error: Error) -> RaygeoError {
    match error {
        Error::NotCdr => RaygeoError::CdrUnsupportedFormat,
        Error::Unsupported(message) => RaygeoError::CdrParseError(message),
        Error::Truncated(at) => {
            RaygeoError::CdrParseError(format!("truncated data at offset {at}"))
        }
    }
}
