//! Convert polylines to Ops.

use crate::geo::types::Point3D;
use crate::ops::container::Ops;
use crate::ops::state::State;

impl Ops {
    /// Build Ops from a 3-D polyline.
    ///
    /// When `move_first` is `true` the first point is emitted as a
    /// `MoveTo` and subsequent points as `LineTo`.  When `move_first`
    /// is `false` every point is emitted as a `LineTo` (for appending
    /// to an in-progress cut).  When `state` is `Some`, the state
    /// commands are applied before the polyline points.
    pub fn from_polyline(
        polyline: &[Point3D],
        move_first: bool,
        state: Option<&State>,
    ) -> Self {
        let mut ops = Ops::new();

        if polyline.is_empty() {
            if let Some(s) = state {
                ops.apply_state(s);
            }
            return ops;
        }

        if let Some(s) = state {
            ops.apply_state(s);
        }

        if move_first {
            let first = polyline[0];
            ops.move_to(first.x, first.y, first.z, None);
            for pt in &polyline[1..] {
                ops.line_to(pt.x, pt.y, pt.z, None);
            }
        } else {
            for pt in polyline {
                ops.line_to(pt.x, pt.y, pt.z, None);
            }
        }

        ops
    }
}
