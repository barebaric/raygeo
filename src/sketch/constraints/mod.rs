//! Sketch constraint implementations.
//!
//! One module per constraint, mirroring the reference sketch model's
//! constraint classes.  Each type owns its residual and gradient
//! math and implements the [`SketchConstraint`] contract.

pub mod angle;
pub mod aspect_ratio;
pub mod coincident;
pub mod collinear;
pub mod diameter;
pub mod distance;
pub mod drag;
pub mod equal_distance;
pub mod equal_length;
pub mod horizontal;
pub mod json;
pub mod parallelogram;
pub mod perpendicular;
pub mod point_on_curve;
pub mod point_on_line;
pub mod radius;
pub mod rotational;
pub mod symmetry;
pub mod tangent;
pub mod vertical;
