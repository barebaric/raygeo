//! CNC domain layer: operation orchestration.
//!
//! Sequences machining operations (entry, clearing, finish), resolves
//! tool-aware `State` via `StateStrategy`, and drives the `geo`/`ops`
//! primitives.  Depends on `geo` and `ops` but not vice versa.

pub mod machining;
