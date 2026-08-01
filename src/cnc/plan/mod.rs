//! Plan-time description of machining operations.
//!
//! [`Plan`]s are produced by planners and consumed by a downstream
//! runtime to derive its own executable steps.  They are never
//! executed directly.
//!
//! Each Rust file in this module has a 1:1 PyO3 mirror in
//! `src/python/cnc/plan/` exposing `raygeo.cnc.plan.<x>`.

pub mod clearing;
pub mod entry;
#[allow(clippy::module_inception)]
pub mod plan;
