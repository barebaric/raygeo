//! Circle-boundary overlap (engagement) metrics.
//!
//! Provides fast analytical engagement-angle / chord-depth from a
//! closest-boundary distance, plus an exact polygon-intersection area
//! for validation.

use crate::geo::shape::polygon::get_circle_polygon;
use crate::geo::shape::polygon::get_polygon_area;
use crate::geo::shape::polygon::get_polygons_group_intersection;
use crate::types::{Point, Polygon};

/// All-in-one engagement result from a closest‑distance query.
#[derive(Clone, Copy, Debug)]
pub struct Engagement {
    /// Contact arc of the circle that lies beyond the boundary (radians).
    /// `0.0` = no overlap, `2π` = fully on the far side.
    pub angle: f64,
    /// Estimated intersection area of the circle with the far side,
    /// derived analytically from `angle` via `R²/2 · (θ − sin θ)`.
    pub area: f64,
    /// Maximum perpendicular distance from the circle edge to the
    /// boundary, always in `[0, R]`.  Zero means no overlap.
    pub chord_depth: f64,
}

/// Compute engagement from the (signed) perpendicular distance between a
/// point and the nearest boundary.
///
/// `d_to_boundary` is the signed distance:
/// * **positive** — point is outside the boundary (the far side)
/// * **negative** — point is inside the boundary (the near side)
/// * **zero** — exactly on the boundary
///
/// The engagement angle is:
/// ```text
/// θ = 2π − 2·acos(clamp(d_to_boundary / R, −1, 1))
/// ```
/// * `d_to_boundary =  0` (on boundary)  →  θ = π          (half circle projects beyond)
/// * `d_to_boundary =  R` (at edge of circle) →  θ = 2π    (fully beyond)
/// * `d_to_boundary = −R` (fully inside)  →  θ = 0          (no projection)
pub fn compute_engagement(d_to_boundary: f64, radius: f64) -> Engagement {
    if radius <= 0.0 {
        return Engagement {
            angle: 0.0,
            area: 0.0,
            chord_depth: 0.0,
        };
    }
    // Clamp to [-R, R].  Outside this range the circle is entirely on one
    // side of the boundary.
    let clamped = (d_to_boundary / radius).clamp(-1.0, 1.0);
    let half_angle = clamped.acos();
    let angle = 2.0 * std::f64::consts::PI - 2.0 * half_angle;
    let area = radius * radius * 0.5 * (angle - angle.sin());
    let chord_depth = if d_to_boundary.abs() < radius {
        radius - d_to_boundary.abs()
    } else {
        0.0
    };
    Engagement {
        angle,
        area,
        chord_depth,
    }
}

/// Exact circle–polygon intersection area using clipper2.
///
/// Intersects the circle (approximated as an `n`‑gon) with `polys`,
/// then computes the area of the result.  Useful for commit‑time validation.
pub fn circle_polygon_intersection_area(
    center: Point,
    radius: f64,
    n: usize,
    polys: &[Polygon],
) -> f64 {
    if radius <= 0.0 || polys.is_empty() {
        return 0.0;
    }
    let circle = get_circle_polygon(center, radius, n);
    let clipped = get_polygons_group_intersection(&[circle], polys);
    clipped.iter().map(get_polygon_area).sum()
}
