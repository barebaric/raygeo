//! Interp: Interpolation helpers and generic math utilities.
//!
//! Provides reusable helpers for segment parameter projection, scanline
//! slicing, quadratic equation solving, and barycentric interpolation on
//! triangles.

use crate::constants::EPSILON_COLLINEAR;
use crate::geo::types::{Point, Point3D};

pub struct SegmentDelta {
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    pub len_sq: f64,
}

/// Compute the delta vector and squared length between two 3D points.
///
/// - `start`: Starting point.
/// - `end`: Ending point.
/// - Returns: A `SegmentDelta` containing the component deltas and squared length.
pub fn compute_segment_delta_3d(start: Point3D, end: Point3D) -> SegmentDelta {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let dz = end.z - start.z;
    let len_sq = end.distance_squared(start);
    SegmentDelta { dx, dy, dz, len_sq }
}

/// Project a point onto a line segment, returning the normalized parameter `t` in [0, 1].
///
/// - `origin`: Start of the segment.
/// - `point`: The point to project.
/// - `delta`: Pre-computed segment delta (from `compute_segment_delta_3d`).
/// - Returns: The parameter `t` clamped to [0, 1].
pub fn project_t_along_segment(
    origin: Point3D,
    point: Point3D,
    delta: &SegmentDelta,
) -> f64 {
    if delta.len_sq <= EPSILON_COLLINEAR {
        return 0.0;
    }
    let t = (point - origin).dot(Point3D::new(delta.dx, delta.dy, delta.dz))
        / delta.len_sq;
    t.clamp(0.0, 1.0)
}

/// Compute the parameter range `(t_start, t_end)` for a clipped sub-segment.
///
/// - `origin`: Start of the original segment.
/// - `new_start`: Start of the clipped sub-segment.
/// - `new_end`: End of the clipped sub-segment.
/// - `delta`: Pre-computed segment delta (from `compute_segment_delta_3d`).
/// - Returns: A tuple `(t_start, t_end)` in [0, 1].
pub fn compute_t_range(
    origin: Point3D,
    new_start: Point3D,
    new_end: Point3D,
    delta: &SegmentDelta,
) -> (f64, f64) {
    if delta.len_sq <= EPSILON_COLLINEAR {
        return (0.0, 1.0);
    }
    let t_start = project_t_along_segment(origin, new_start, delta);
    let t_end = project_t_along_segment(origin, new_end, delta);
    (t_start, t_end)
}

/// Slice a scanline power array by parameter range `[t_start, t_end)`.
///
/// - `data`: The full scanline power values.
/// - `t_start`: Start parameter in [0, 1].
/// - `t_end`: End parameter in [0, 1].
/// - Returns: A new `Vec<u8>` containing the sliced power values.
pub fn slice_scanline_data(data: &[u8], t_start: f64, t_end: f64) -> Vec<u8> {
    let num_values = data.len();
    let idx_start = (num_values as f64 * t_start) as usize;
    let idx_end = (num_values as f64 * t_end) as usize;
    data[idx_start..idx_end].to_vec()
}

/// Compute raw barycentric coordinates `(r, s, t)` for point `p`
/// relative to triangle `(va, vb, vc)`.
///
/// Returns `(r, s, t)` where:
/// - `r` is the weight for vertex `va`
/// - `s` is the weight for vertex `vb`
/// - `t` is the weight for vertex `vc`
///
/// The weights are unclamped — a point outside the triangle will have
/// one or more negative weights. The point is inside (or on the boundary
/// of) the triangle iff all three weights are in `[-ε, 1+ε]`.
pub fn get_barycentric_weights(
    p: Point,
    va: Point,
    vb: Point,
    vc: Point,
) -> (f64, f64, f64) {
    let v0 = vc - va;
    let v1 = vb - va;
    let v2 = p - va;

    let d00 = v0.length_squared();
    let d01 = v0.dot(v1);
    let d11 = v1.length_squared();
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-24 {
        return (0.0, 0.0, 0.0);
    }
    let inv = 1.0 / denom;
    let s = (d00 * d21 - d01 * d20) * inv;
    let t = (d11 * d20 - d01 * d21) * inv;
    let r = 1.0 - s - t;
    (r, s, t)
}

/// Interpolate a scalar field at a point inside a triangle using
/// barycentric coordinates.
///
/// Given triangle vertices `(va, vb, vc)` with scalar values `(ua, ub, uc)`,
/// returns the linearly interpolated value at point `p`. The function
/// works for any point in the plane but values outside the triangle are
/// clamped to the [0,1] barycentric range.
pub fn barycentric_interpolate(
    p: Point,
    va: Point,
    vb: Point,
    vc: Point,
    ua: f64,
    ub: f64,
    uc: f64,
) -> f64 {
    let (mut r, mut s, mut t) = get_barycentric_weights(p, va, vb, vc);
    let sum = r + s + t;
    if sum.abs() < 1e-24 {
        return (ua + ub + uc) / 3.0;
    }
    r = r.clamp(0.0, 1.0);
    s = s.clamp(0.0, 1.0);
    t = t.clamp(0.0, 1.0);
    (r * ua + s * ub + t * uc) / (r + s + t).max(1e-24)
}

/// Solve the quadratic equation `a x² + b x + c = 0`.
///
/// When `|a| <= EPSILON_COLLINEAR` the equation is treated as linear `b x + c = 0`.
/// Roots are returned in ascending order.
///
/// - `a`: Quadratic coefficient.
/// - `b`: Linear coefficient.
/// - `c`: Constant term.
/// - Returns: A tuple `(root1, root2)`, each `None` if no real root exists.
pub fn solve_quadratic(a: f64, b: f64, c: f64) -> (Option<f64>, Option<f64>) {
    if a.abs() <= EPSILON_COLLINEAR {
        if b.abs() <= EPSILON_COLLINEAR {
            return (None, None);
        }
        return (Some(-c / b), None);
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return (None, None);
    }
    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);
    if t1 <= t2 {
        (Some(t1), Some(t2))
    } else {
        (Some(t2), Some(t1))
    }
}
