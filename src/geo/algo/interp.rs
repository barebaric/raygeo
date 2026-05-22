//! Interp: Interpolation helpers and generic math utilities.
//!
//! Provides reusable helpers for segment parameter projection, scanline
//! slicing, and quadratic equation solving.

use crate::constants::EPSILON_COLLINEAR;
use crate::types::Point3D;

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
pub fn compute_segment_delta(start: Point3D, end: Point3D) -> SegmentDelta {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let dz = end.2 - start.2;
    let len_sq = dx * dx + dy * dy + dz * dz;
    SegmentDelta { dx, dy, dz, len_sq }
}

/// Project a point onto a line segment, returning the normalized parameter `t` in [0, 1].
///
/// - `origin`: Start of the segment.
/// - `point`: The point to project.
/// - `delta`: Pre-computed segment delta (from `compute_segment_delta`).
/// - Returns: The parameter `t` clamped to [0, 1].
pub fn project_t_along_segment(
    origin: Point3D,
    point: Point3D,
    delta: &SegmentDelta,
) -> f64 {
    if delta.len_sq <= EPSILON_COLLINEAR {
        return 0.0;
    }
    let t = ((point.0 - origin.0) * delta.dx
        + (point.1 - origin.1) * delta.dy
        + (point.2 - origin.2) * delta.dz)
        / delta.len_sq;
    t.clamp(0.0, 1.0)
}

/// Compute the parameter range `(t_start, t_end)` for a clipped sub-segment.
///
/// - `origin`: Start of the original segment.
/// - `new_start`: Start of the clipped sub-segment.
/// - `new_end`: End of the clipped sub-segment.
/// - `delta`: Pre-computed segment delta (from `compute_segment_delta`).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_segment_delta() {
        let d = compute_segment_delta((0.0, 0.0, 0.0), (3.0, 4.0, 0.0));
        assert!((d.dx - 3.0).abs() < 1e-9);
        assert!((d.dy - 4.0).abs() < 1e-9);
        assert!((d.len_sq - 25.0).abs() < 1e-9);
    }

    #[test]
    fn test_project_t_along_segment() {
        let d = compute_segment_delta((0.0, 0.0, 0.0), (10.0, 0.0, 0.0));
        let t = project_t_along_segment((0.0, 0.0, 0.0), (5.0, 0.0, 0.0), &d);
        assert!((t - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_compute_t_range() {
        let d = compute_segment_delta((0.0, 0.0, 0.0), (10.0, 0.0, 0.0));
        let (t_s, t_e) = compute_t_range(
            (0.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (8.0, 0.0, 0.0),
            &d,
        );
        assert!((t_s - 0.2).abs() < 1e-9);
        assert!((t_e - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_slice_scanline_data() {
        let data = vec![10u8, 20, 30, 40, 50];
        let sliced = slice_scanline_data(&data, 0.2, 0.6);
        assert_eq!(sliced, vec![20, 30]);
    }

    #[test]
    fn test_solve_quadratic_two_roots() {
        let (r1, r2) = solve_quadratic(1.0, -3.0, 2.0);
        assert!((r1.unwrap() - 1.0).abs() < 1e-9);
        assert!((r2.unwrap() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_solve_quadratic_one_root() {
        let (r1, r2) = solve_quadratic(0.0, 2.0, -4.0);
        assert!((r1.unwrap() - 2.0).abs() < 1e-9);
        assert!(r2.is_none());
    }

    #[test]
    fn test_solve_quadratic_no_roots() {
        let (r1, r2) = solve_quadratic(0.0, 0.0, 1.0);
        assert!(r1.is_none());
        assert!(r2.is_none());
    }

    #[test]
    fn test_solve_quadratic_negative_discriminant() {
        let (r1, r2) = solve_quadratic(1.0, 0.0, 1.0);
        assert!(r1.is_none());
        assert!(r2.is_none());
    }
}
