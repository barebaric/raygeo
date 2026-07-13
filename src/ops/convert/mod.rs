//! Convert: Ops ↔ other format conversions.
//!
//! This module groups all functions that convert between [`Ops`] and
//! other representations — polylines, geometries, GPU vertex arrays,
//! pixel textures, and images.  Unlike the `transform` module, which
//! produces new `Ops` from existing `Ops`, the converters here cross
//! format boundaries.

pub mod dump;
pub mod geometry;
pub mod image;
pub mod polyline;
pub mod texture;
pub mod vertex_arrays;
