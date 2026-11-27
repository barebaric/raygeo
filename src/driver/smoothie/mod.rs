//! The Smoothie protocol implementation.
//!
//! Cross-protocol imports (sanctioned per AGENTS.md, justified here
//! in the importing module's docs): Smoothie answers `?` with
//! Grbl-style `<...>` status reports and shares the GRBL error and
//! alarm code tables, so the session reuses `crate::driver::grbl::parser`
//! (status parsing, error lookup, comment stripping, G54-G59
//! P-number mapping) and `crate::driver::grbl::errors` (alarm-code to
//! device-error mapping).

pub mod driver;
pub mod probe;
