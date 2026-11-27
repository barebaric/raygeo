//! The Marlin protocol implementation.
//!
//! Cross-protocol import (sanctioned per AGENTS.md): Marlin's WCS
//! offsets use the industry-standard G54-G59 to P1-P6 mapping that
//! GRBL also documents, so the session reuses
//! `crate::driver::grbl::parser::gcode_to_p_number`.

pub mod driver;
pub mod parser;
pub mod probe;
