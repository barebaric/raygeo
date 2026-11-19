//! Coordinate transforms applied to groups of sketch entities.
//!
//! Placement transforms are supplied structurally by the caller:
//! array placements, mirror axes and similar host objects implement
//! the two mapping operations below.

/// Maps absolute points and relative offsets into new coordinates.
pub trait PlacementTransform {
    /// Transforms an absolute point.
    fn transform_point(&self, x: f64, y: f64) -> (f64, f64);

    /// Transforms a relative offset (e.g. a bezier control point).
    fn transform_offset(&self, dx: f64, dy: f64) -> (f64, f64);
}

/// Whether the transform reverses the local frame's orientation.
pub fn flips_chirality(transform: &dyn PlacementTransform) -> bool {
    let ex = transform.transform_offset(1.0, 0.0);
    let ey = transform.transform_offset(0.0, 1.0);
    ex.0 * ey.1 - ex.1 * ey.0 < 0.0
}
