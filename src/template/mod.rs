//! Host-agnostic template support.
//!
//! Text templates are plain strings with `{...}` placeholders. A
//! placeholder contains an expression and an optional format spec after
//! the first colon, for example `{counter}` or `{counter:04d}`.
//!
//! This module owns the pieces that do not depend on a host language:
//!
//! * [`counter::CounterSettings`] — a document-level job counter with
//!   wrap-around arithmetic.
//! * [`placeholders`] — extraction of the variable names a template
//!   reads, used for dependency tracking and cache invalidation.
//!
//! The expression language itself is host-specific and stays outside
//! this module.

pub mod counter;
pub mod expr;
pub mod placeholders;
pub mod text;
