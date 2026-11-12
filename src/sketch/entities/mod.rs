//! Sketch entities, mirroring the reference `entities` package:
//! one module per entity, each implementing the polymorphic
//! [`Entity`] trait exactly like the corresponding Python class
//! (`line.rs` ↔ `Line`, `arc.rs` ↔ `Arc`, ...).  [`point.rs`] holds
//! the sketch points (the reference keeps them outside the entity
//! hierarchy), and [`mirror.rs`] the mirror axis used by the
//! entities' `mirror` implementations.

pub mod arc;
pub mod bezier;
pub mod circle;
pub mod ellipse;
pub mod entity;
pub mod line;
pub mod mirror;
pub mod point;
pub mod polygon;
pub mod text_box;

pub use entity::{
    entity_from_json, quantize, ColorRgba, Entity, EntityBase, FontConfig,
};
pub use mirror::{MirrorAxis, MirrorDirection};
pub use point::SketchPoint;
