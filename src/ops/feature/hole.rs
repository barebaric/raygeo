//! Circular hole detection in part faces.
//!
//! Classifies a face's island contours (inner contours of the part
//! geometry) as drillable holes: an island whose outline lies on a
//! circle within a tolerance is reported as a [`Hole`] with center and
//! diameter. Non-circular islands (bosses, slots, arbitrary pockets)
//! are rejected.
//!
//! The circle fit itself is pure geometry — Kasa least squares from
//! [`fit_circle_to_points_3d`]; the machining decision ("this contour
//! is a hole") lives here.
//!
//! Contours are densified along their perimeter before fitting, so a
//! coarse polygon (a square, a triangle) cannot pass merely because
//! its few vertices happen to lie on a common circle.

use crate::geo::algo::fitting::fit_circle_to_points_3d;
use crate::geo::types::{Point, Point3D, Polygon};

/// Approximate spacing of the contour samples used for the circle fit.
const SAMPLE_SPACING: f64 = 0.2;

/// A circular hole detected in a part face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hole {
    pub center: Point,
    pub diameter: f64,
}

/// Find circular holes among island contours.
///
/// An island counts as a hole when every densified sample of its
/// outline lies on a common circle within `tolerance` (mm). Islands
/// with fewer than 3 points are ignored. Results are returned in the
/// order the islands were passed.
pub fn find_holes(islands: &[Polygon], tolerance: f64) -> Vec<Hole> {
    islands
        .iter()
        .filter_map(|island| find_hole_in_contour(island, tolerance))
        .collect()
}

/// Fit a circle to one contour and accept it as a hole when the fit
/// residual is within `tolerance`.
fn find_hole_in_contour(contour: &Polygon, tolerance: f64) -> Option<Hole> {
    if contour.len() < 3 {
        return None;
    }
    let samples = densify(contour, SAMPLE_SPACING);
    let (center, radius, max_err) = fit_circle_to_points_3d(&samples)?;
    if !max_err.is_finite() || max_err > tolerance {
        return None;
    }
    Some(Hole {
        center,
        diameter: 2.0 * radius,
    })
}

/// Sample a closed polygon roughly every `spacing` mm along its
/// perimeter (vertices included).
fn densify(contour: &Polygon, spacing: f64) -> Vec<Point3D> {
    let spacing = spacing.max(1e-9);
    let n = contour.len();
    let mut out = Vec::new();
    for i in 0..n {
        let a = contour[i];
        let b = contour[(i + 1) % n];
        out.push(Point3D::new(a.x, a.y, 0.0));
        let seg_len = a.distance(b);
        if seg_len <= spacing {
            continue;
        }
        let steps = (seg_len / spacing).floor() as usize;
        for k in 1..=steps {
            let t = k as f64 / (steps + 1) as f64;
            let p = a * (1.0 - t) + b * t;
            out.push(Point3D::new(p.x, p.y, 0.0));
        }
    }
    out
}
