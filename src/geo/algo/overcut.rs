//! Overcut: Extend closed contours past their start point.
//!
//! When laser-cutting closed contours, the laser slows down at corners and
//! may not cut through completely. Extending the path slightly past the
//! starting point (overcut) ensures a clean cut through the full contour.

use crate::constants::EPSILON_COLLINEAR;
use crate::geo::query::extract_overcut_rows;
use crate::types::Command;
use crate::Geometry;

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
                result.bezier_to((*control1, *control2, (end.0, end.1)), end.2);
            }
            Command::Move { .. } => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overcut_empty_geometry() {
        let geo = Geometry::new();
        let result = apply_overcut(&geo, 5.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_overcut_zero_is_noop() {
        let mut geo = Geometry::new();
        geo.move_to(0.0, 0.0, 0.0);
        geo.line_to(10.0, 0.0, 0.0);
        geo.line_to(10.0, 10.0, 0.0);
        geo.close_path();
        let result = apply_overcut(&geo, 0.0);
        assert_eq!(result.data().len(), geo.data().len());
    }

    #[test]
    fn test_overcut_open_path_is_noop() {
        let mut geo = Geometry::new();
        geo.move_to(0.0, 0.0, 0.0);
        geo.line_to(10.0, 0.0, 0.0);
        geo.line_to(10.0, 10.0, 0.0);
        let result = apply_overcut(&geo, 5.0);
        assert_eq!(result.data().len(), geo.data().len());
    }

    #[test]
    fn test_overcut_closed_rectangle() {
        let mut geo = Geometry::new();
        geo.move_to(0.0, 0.0, 0.0);
        geo.line_to(10.0, 0.0, 0.0);
        geo.line_to(10.0, 10.0, 0.0);
        geo.line_to(0.0, 10.0, 0.0);
        geo.close_path();
        let result = apply_overcut(&geo, 5.0);
        assert!(result.data().len() > geo.data().len());
    }

    #[test]
    fn test_overcut_larger_than_first_side() {
        let mut geo = Geometry::new();
        geo.move_to(0.0, 0.0, 0.0);
        geo.line_to(10.0, 0.0, 0.0);
        geo.line_to(10.0, 10.0, 0.0);
        geo.close_path();
        let result = apply_overcut(&geo, 15.0);
        assert!(result.data().len() > geo.data().len());
    }
}
