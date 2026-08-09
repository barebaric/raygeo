//! Generic runtime intent-tree executor.
//!
//! Executes a DAG of [`NodeRequest`]s on a rayon thread pool. The
//! pipeline knows nothing about any domain — it runs generic
//! [`Compute`] and [`Aggregate`] trait objects and passes opaque
//! `Box<dyn Any>` outputs between nodes.
//!
//! ## Wire surface
//!
//! The wire surface is normalized to two stage kinds —
//! [`StageSpec::Compute`] and [`StageSpec::Aggregate`] — which compose
//! into an arbitrarily nested intent tree. There is no `Encode`
//! variant: encoding is a `Compute` that reads from the dep map.
//!
//! ## Layering
//!
//! This module depends on nothing except `std` and `rayon`.

pub mod aggregate;
pub mod cache;
pub mod callbacks;
pub mod completed;
pub mod compute;
pub mod execute;
#[allow(clippy::module_inception)]
pub mod pipeline;
pub mod request;
pub mod stage;
mod stage_cache;
