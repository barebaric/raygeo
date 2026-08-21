//! Domain execution glue — connects ops algorithms to the generic pipeline.
//!
//! Implements `pipeline::Compute` and `pipeline::Aggregate` for ops types
//! (assemblers, encoders, aggregation with markers). This is the module
//! where `ops` meets `pipeline`.

pub mod aggregate;
pub mod callbacks;
pub mod compute;
pub mod encode;
pub mod intent;
pub mod machine_transform;
pub mod material_fold;
pub mod specs;
