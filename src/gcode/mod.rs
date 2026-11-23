//! G-code (RS274/NGC) geometry extraction.
//!
//! Parses the common subset of RS274/NGC written by laser cutters, CAM
//! tools and LightBurn. Only material-removing moves are emitted: rapid
//! moves, and feed moves whose spindle speed is zero, are treated as
//! travel and split the geometry into separate contours.
//!
//! The result is always in millimetres, Y-up, with one `Geometry` holding
//! all cutting paths. Speed, feed and power values are deliberately
//! ignored, so
//! importing a program recovers its shape, not its machining parameters.

pub mod arc;
pub mod lexer;
pub mod parser;

use std::collections::BTreeSet;

use crate::geo::geometry::Geometry;

/// The cutting geometry and skipped instructions of one G-code program.
pub struct GcodeParseOutput {
    /// Cutting moves in millimetres, Y-up, one subpath per contour.
    pub geometry: Geometry,
    /// Instructions that were recognized but not applied, sorted.
    pub unsupported: BTreeSet<String>,
}

/// Parses a G-code program into cutting geometry in millimetres.
///
/// Supported motion is G0 (rapid travel), G1 (linear cut), G2/G3 (arcs
/// in I/J or R form), with G20/G21 units, G90/G91 distance mode and
/// G90.1/G91.1 arc-centre mode. Everything that does not change XY
/// geometry (feed, spindle speed, coolant, work offsets) is ignored;
/// instructions that would change it but are not implemented (canned
/// cycles, cutter compensation, subroutines, non-XY planes) are listed in
/// ``unsupported``.
///
/// When ``s0_is_travel`` is true, feed moves made while the modal spindle
/// speed is zero are treated as travel. This is how LightBurn encodes
/// fill passes and overscan.
pub fn parse_gcode(text: &str, s0_is_travel: bool) -> GcodeParseOutput {
    let mut parser = parser::Parser::new(s0_is_travel);
    parser.run(text);
    parser.finish()
}
