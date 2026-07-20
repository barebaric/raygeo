//! Smooth: In-place path smoothing for Ops sequences.
//!
//! Replaces the Python `Smooth` transformer loop with a single
//! Rust method that linearizes arcs, segments the path, smooths
//! line-only segments with a Gaussian filter, and transfers
//! non-line segments unchanged.

use crate::geo::algo::smooth::smooth_polyline_3d;
use crate::ops::cache::Cacheable;
use crate::ops::container::Ops;
use crate::ops::enums::CommandType;
use crate::ops::transform::{Phase, TransformCtx, Transformer};
use crate::types::Point3D;

/// Parameters for the [`smooth`] transformer.
#[derive(Clone, Debug, PartialEq)]
pub struct SmoothSpec {
    /// Smoothing strength (0-100); 0 is a no-op.
    pub amount: u32,
    /// Corners with an internal angle (degrees) smaller than this are
    /// preserved.
    pub corner_angle_threshold: f64,
}

impl Transformer for SmoothSpec {
    fn phase(&self) -> Phase {
        Phase::GeometryRefinement
    }

    fn apply(&self, ctx: &mut TransformCtx<'_>) {
        ctx.ops.smooth(self.amount, self.corner_angle_threshold);
    }

    fn name(&self) -> &'static str {
        "smooth"
    }
}

impl Cacheable<Ops> for SmoothSpec {}

/// Check whether a segment contains only MoveTo followed by LineTo
/// commands (at least 2 commands).
fn is_line_only_segment(ops: &Ops, indices: &[usize]) -> bool {
    if indices.len() < 2 {
        return false;
    }
    if ops.command_type(indices[0]) != CommandType::MoveTo {
        return false;
    }
    indices[1..]
        .iter()
        .all(|&idx| ops.command_type(idx) == CommandType::LineTo)
}

impl Ops {
    /// Smooth all line-only segments using a Gaussian filter.
    ///
    /// Arcs are linearized first. Segments containing curves are
    /// transferred unchanged.  The smoothing operates in place:
    /// `self` is cleared and rebuilt from a copy.
    pub fn smooth(&mut self, amount: u32, corner_angle_threshold: f64) {
        if amount == 0 {
            return;
        }

        self.linearize_arcs();
        let all_indices = self.segment_indices();
        let source = self.copy();
        self.clear();

        for indices in &all_indices {
            if is_line_only_segment(&source, indices) {
                let points: Vec<Point3D> =
                    indices.iter().map(|&idx| source.endpoint(idx)).collect();

                let smoothed = smooth_polyline_3d(
                    &points,
                    amount as i32,
                    corner_angle_threshold,
                    None,
                );

                if !smoothed.is_empty() {
                    self.move_to(
                        smoothed[0].x,
                        smoothed[0].y,
                        smoothed[0].z,
                        None,
                    );
                    for point in &smoothed[1..] {
                        self.line_to(point.x, point.y, point.z, None);
                    }
                }
            } else {
                for &idx in indices {
                    self.transfer_command_from(&source, idx);
                }
            }
        }
    }
}
