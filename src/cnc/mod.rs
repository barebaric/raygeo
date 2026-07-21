//! CNC domain layer: operation orchestration and runtime execution glue.
//!
//! Sequences machining operations (entry, clearing, finish), resolves
//! tool-aware `State` via `StateStrategy`, and drives the `geo`/`ops`
//! primitives.  Depends on `geo` and `ops` but not vice versa.
//!
//! ## Execution glue
//!
//! The `execution` submodule implements `pipeline::Compute` and
//! `pipeline::Aggregate` for all ops types (assemblers, encoders,
//! aggregation with markers). Callers construct
//! `pipeline::NodeRequest`s through these wrappers and dispatch them
//! via `pipeline::execute_stages`.

pub mod execution;
pub mod machining;
