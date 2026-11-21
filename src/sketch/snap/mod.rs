//! Core snap engine: value types, producer algorithms and geometry
//! fingerprints.
//!
//! The producers are pure functions over the entity registry and a
//! [`types::SnapContext`]; the Python binding layer materializes
//! their results into the host's snap objects.  The orchestration
//! (producer registry, index cache, query flow) lives in the binding
//! layer because its producers and results are host objects.

pub mod fingerprint;
pub mod producers;
pub mod types;
