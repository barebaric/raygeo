//! Polyline shapes and operations.
//!
//! These functions operate on open polylines (`Vec<Point>`), as distinct
//! from closed polygons.

use prof_macros::prof;

use crate::geo::shape::line::get_interior_angle;
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::types::{Point, Rect};

/// Get the bounding box of an open polyline.
///
/// Returns `Rect::default()` when the polyline is empty.
#[prof]
pub fn get_polyline_bounds(pts: &[Point]) -> Rect {
    if pts.is_empty() {
        return Rect::default();
    }
    let mut min_x = pts[0].x;
    let mut max_x = pts[0].x;
    let mut min_y = pts[0].y;
    let mut max_y = pts[0].y;
    for p in pts {
        let x = p.x;
        let y = p.y;
        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }
        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }
    }
    Rect::new(min_x, min_y, max_x, max_y)
}

/// Return the edge index and parametric position of the closest point
/// on an open polyline.
pub fn get_polyline_closest_point(
    polyline: &[Point],
    p: Point,
) -> Option<(usize, f64)> {
    let n = polyline.len();
    if n < 2 {
        return None;
    }
    let mut best_i = 0usize;
    let mut best_t = 0.0;
    let mut best_d2 = f64::MAX;
    for i in 0..n - 1 {
        let (t, _, d2) = get_line_segment_closest_point(
            polyline[i],
            polyline[i + 1],
            p.x,
            p.y,
        );
        if d2 < best_d2 {
            best_d2 = d2;
            best_i = i;
            best_t = t;
        }
    }
    Some((best_i, best_t))
}

/// Trim an open polyline to the portion between two points.
///
/// Each point is projected onto the nearest edge of the polyline. The
/// returned polyline goes from the projection of `a` to the projection
/// of `b`, preserving intermediate vertices.  Adjacent duplicates are
/// removed.
pub fn trim_polyline_at(polyline: &[Point], a: Point, b: Point) -> Vec<Point> {
    let Some((ai, at)) = get_polyline_closest_point(polyline, a) else {
        return polyline.to_vec();
    };
    let Some((bi, bt)) = get_polyline_closest_point(polyline, b) else {
        return polyline.to_vec();
    };

    let (start_i, start_t, end_i, end_t) = if ai < bi || (ai == bi && at <= bt)
    {
        (ai, at, bi, bt)
    } else {
        (bi, bt, ai, at)
    };

    let sa = polyline[start_i];
    let start = sa + (polyline[start_i + 1] - sa) * start_t;
    let sb = polyline[end_i];
    let end = sb + (polyline[end_i + 1] - sb) * end_t;

    let mut result = vec![start];
    result.extend(polyline.iter().take(end_i + 1).skip(start_i + 1).copied());
    result.push(end);
    result.dedup_by(|a, b| a.distance_squared(*b) < 1e-12);
    result
}

/// Resample an open 2D polyline so that consecutive points are at most
/// `max_len` apart.
///
/// New points are linearly interpolated along each segment that exceeds
/// the threshold.  The first and last points are always preserved.
pub fn resample_polyline(points: &[Point], max_len: f64) -> Vec<Point> {
    if points.len() < 2 || max_len <= 0.0 {
        return points.to_vec();
    }
    let mut out = vec![points[0]];
    for i in 0..points.len() - 1 {
        let a = points[i];
        let b = points[i + 1];
        let dist = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        if dist > max_len {
            let n = (dist / max_len).ceil() as usize;
            for j in 1..n {
                let t = j as f64 / n as f64;
                out.push(Point::new(
                    a.x + (b.x - a.x) * t,
                    a.y + (b.y - a.y) * t,
                ));
            }
        }
        out.push(b);
    }
    out
}

/// Trim vertices from both ends of a contiguous subsequence of a closed
/// polygon where the interior angle jumps sharply (≥ `threshold_rad`)
/// compared to the adjacent vertex further inward.
///
/// This detects "transition" vertices at the boundary between two
/// differently-curved regions of the polygon (e.g., where an outer arc
/// meets an inner arc).  The function iteratively trims such vertices
/// from the start and end of the subsequence until no more trimming
/// occurs or the sequence is too short.
///
/// Returns the adjusted `(start, length)` within the original polygon.
/// The caller should check whether the result is still useful (at least
/// 3 vertices).
#[prof]
pub fn trim_polyline_angular_ends(
    polygon: &[Point],
    start: usize,
    length: usize,
    threshold_rad: f64,
) -> (usize, usize) {
    let n = polygon.len();
    if n < 3 || length < 3 {
        return (start, length);
    }
    let mut cut_start = start % n;
    let mut cut_len = length;
    let mut trimmed = true;
    while trimmed && cut_len > 3 {
        trimmed = false;
        let first = (cut_start + 1) % n;
        let b = polygon[first];
        let c = polygon[(first + 1) % n];
        let d = polygon[(first + 2) % n];
        let angle_curr = get_interior_angle(polygon[(first + n - 1) % n], b, c);
        let angle_next = get_interior_angle(b, c, d);
        if angle_curr + threshold_rad < angle_next {
            cut_start = (cut_start + 1) % n;
            cut_len -= 1;
            trimmed = true;
        }
        let last = (cut_start + cut_len - 2) % n;
        let a = polygon[(last + n - 1) % n];
        let b = polygon[last];
        let c = polygon[(last + 1) % n];
        let a_prev = polygon[(last + n - 2) % n];
        let angle_curr = get_interior_angle(a, b, c);
        let angle_prev = get_interior_angle(a_prev, a, b);
        if angle_curr + threshold_rad < angle_prev {
            cut_len -= 1;
            trimmed = true;
        }
    }
    (cut_start, cut_len)
}

/// Split a polyline at internal vertices where the interior angle is much
/// sharper than both neighbours (a V-junction), then trim each sub-polyline's
/// angular ends.
///
/// A vertex at index `i` is considered a V-junction when:
///
/// ```text
/// angle_curr + angle_threshold < angle_prev
///     && angle_curr + angle_threshold < angle_next
/// ```
///
/// where `angle_curr` is the interior angle at `polyline[i]`, and
/// `angle_prev` / `angle_next` are the angles at the adjacent vertices.
///
/// Each resulting sub-polyline is further cleaned with
/// [`trim_polyline_angular_ends`] using the same threshold.
///
/// Returns one or more sub-polylines.  When no split point is found the entire
/// input is returned as a single segment.
#[prof]
pub fn split_polyline_at_v_junctions(
    polyline: &[Point],
    angle_threshold: f64,
) -> Vec<Vec<Point>> {
    if polyline.len() < 5 {
        return vec![polyline.to_vec()];
    }

    let mut split_pts: Vec<usize> = Vec::new();
    for i in 2..polyline.len() - 2 {
        let angle_curr =
            get_interior_angle(polyline[i - 1], polyline[i], polyline[i + 1]);
        let angle_prev =
            get_interior_angle(polyline[i - 2], polyline[i - 1], polyline[i]);
        let angle_next =
            get_interior_angle(polyline[i], polyline[i + 1], polyline[i + 2]);
        if angle_curr + angle_threshold < angle_prev
            && angle_curr + angle_threshold < angle_next
        {
            split_pts.push(i);
        }
    }

    if split_pts.is_empty() {
        return vec![polyline.to_vec()];
    }

    let mut result = Vec::new();
    let mut start = 0;
    for &sp in &split_pts {
        let seg = &polyline[start..=sp];
        let (s, l) =
            trim_polyline_angular_ends(seg, 0, seg.len(), angle_threshold);
        if l >= 3 {
            result.push(seg[s..s + l].to_vec());
        }
        start = sp;
    }
    let tail = &polyline[start..];
    let (s, l) =
        trim_polyline_angular_ends(tail, 0, tail.len(), angle_threshold);
    if l >= 3 {
        result.push(tail[s..s + l].to_vec());
    }

    if result.is_empty() {
        vec![polyline.to_vec()]
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn test_split_no_split_on_smooth_arc() {
        let pts: Vec<Point> = (0..20)
            .map(|i| {
                let a = std::f64::consts::FRAC_PI_2 * i as f64 / 19.0;
                v(50.0 + 30.0 * a.cos(), 50.0 + 30.0 * a.sin())
            })
            .collect();
        let result = split_polyline_at_v_junctions(&pts, 0.436);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), pts.len());
    }

    #[test]
    fn test_split_no_split_on_line() {
        let pts = vec![v(0.0, 0.0), v(10.0, 0.0), v(20.0, 0.0), v(30.0, 0.0)];
        let result = split_polyline_at_v_junctions(&pts, 0.436);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_split_at_sharp_v() {
        let pts = vec![
            v(0.0, 0.0),
            v(5.0, 0.0),
            v(10.0, 0.0),
            v(10.0, 5.0),
            v(10.0, 10.0),
        ];
        let result = split_polyline_at_v_junctions(&pts, 0.1);
        assert!(
            result.len() >= 2,
            "sharp V should split, got {} segments",
            result.len()
        );
    }

    #[test]
    fn test_split_small_input() {
        let pts = vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0)];
        let result = split_polyline_at_v_junctions(&pts, 0.1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 3);
    }

    #[test]
    fn test_split_empty_input() {
        let result = split_polyline_at_v_junctions(&[], 0.1);
        assert_eq!(result.len(), 1);
        assert!(result[0].is_empty());
    }

    #[test]
    fn test_split_high_threshold_no_split() {
        let pts = vec![
            v(0.0, 0.0),
            v(5.0, 0.0),
            v(10.0, 0.0),
            v(10.0, 5.0),
            v(10.0, 10.0),
        ];
        let result = split_polyline_at_v_junctions(&pts, 100.0);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_split_multiple_splits() {
        let pts = vec![
            v(0.0, 0.0),
            v(5.0, 0.0),
            v(10.0, 0.0),
            v(10.0, 5.0),
            v(10.0, 10.0),
            v(5.0, 10.0),
            v(0.0, 10.0),
            v(0.0, 5.0),
            v(0.0, 0.0),
        ];
        let result = split_polyline_at_v_junctions(&pts, 0.1);
        assert!(
            result.len() >= 3,
            "expected at least 3 segments, got {}",
            result.len()
        );
    }
}
