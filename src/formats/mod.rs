//! File-format import and export.
//!
//! Each sub-module translates between one external drawing format and
//! [`crate::geo`] primitives. The modules depend only on `geo` and the
//! shared error type; the layers below never import them.
//!
//! - `svg`: SVG parsing and path export
//! - `gcode`: G-code (RS274/NGC) geometry extraction
//! - `dxf`: DXF reading and writing

pub mod dxf;
pub mod gcode;
pub mod svg;
