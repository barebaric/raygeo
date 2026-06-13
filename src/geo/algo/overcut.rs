//! Overcut: Extend closed contours past their start point.
//!
//! When laser-cutting closed contours, the laser slows down at corners and
//! may not cut through completely. Extending the path slightly past the
//! starting point (overcut) ensures a clean cut through the full contour.

use crate::constants::EPSILON_COLLINEAR;
use crate::geo::geometry::Geometry;
use crate::geo::query::extract_overcut_rows;
use crate::types::Command;

/// Apply overcut to a closed geometry.
pub fn apply_overcut(geo: &Geometry, overcut: f64) -> Geometry {
    if overcut <= 0.0 || geo.is_empty() || geo.data().len() < 2 {
        return geo.copy();
    }
    if !geo.is_closed(EPSILON_COLLINEAR * 10.0) {
        return geo.copy();
    }

    let mut result = geo.copy();

    let overcut_cmds = match extract_overcut_rows(geo.data(), overcut) {
        Some(cmds) => cmds,
        None => return result,
    };

    for cmd in &overcut_cmds {
        let end = cmd.end_point();
        match cmd {
            Command::Line { .. } => {
                result.line_to(end.0, end.1, end.2);
            }
            Command::Arc {
                center_offset,
                clockwise,
                ..
            } => {
                result.arc_to(
                    end.0,
                    end.1,
                    center_offset.0,
                    center_offset.1,
                    *clockwise,
                    end.2,
                );
            }
            Command::Bezier {
                control1, control2, ..
            } => {
                result.bezier_to(*control1, *control2, end);
            }
            Command::Move { .. } => {}
        }
    }

    result
}
