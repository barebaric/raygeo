//! Mirror axis, mirroring the reference command-layer type but
//! needed by the entity `mirror` methods for their internal state.

/// Direction of a mirror reflection axis: `Vertical` is a
/// horizontal axis at y = position (flips Y), `Horizontal` is a
/// vertical axis at x = position (flips X).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MirrorDirection {
    Vertical,
    Horizontal,
}

/// A mirror reflection axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MirrorAxis {
    pub direction: MirrorDirection,
    pub position: f64,
}

impl MirrorAxis {
    /// Mirrors a (dx, dy) offset vector across this axis.
    pub fn flip_offset(&self, offset: [f64; 2]) -> [f64; 2] {
        match self.direction {
            MirrorDirection::Vertical => [offset[0], -offset[1]],
            MirrorDirection::Horizontal => [-offset[0], offset[1]],
        }
    }
}
