//! Smooth: Polyline smoothing using Gaussian filtering.
//!
//! Provides functions for computing Gaussian kernels and applying them
//! to open or closed polylines with optional corner preservation.

use prof_macros::prof;

use crate::geo::shape::does_path_sweep_intersect_polygon;
use crate::geo::shape::line::get_angle_at_vertex;
use crate::geo::shape::polygon3d::resample_polyline_3d;
use crate::geo::shape::polyline::resample_polyline as resample_polyline_2d;
use crate::types::{Point, Point3D, Polygon, Rect};

/// Compute a normalized Gaussian kernel based on smoothing amount.
///
/// `amount` ranges from 0 (none) to 200 (very heavy). The sigma is derived
/// as `(amount / 100.0) * 5.0 + 0.1`, and the radius is `ceil(sigma * 3)`.
/// Returns `(kernel, sigma)` where `kernel` is a normalized list of weights
/// that sums to 1.0.
pub fn compute_gaussian_kernel(amount: i32) -> (Vec<f64>, f64) {
    if amount == 0 {
        return (vec![1.0], 0.0);
    }

    let sigma = (amount as f64 / 100.0) * 5.0 + 0.1;
    let radius = (sigma * 3.0).ceil() as i32;
    let size = (2 * radius + 1) as usize;
    let mut kernel = vec![0.0; size];
    let mut kernel_sum = 0.0;

    for (i, k) in kernel.iter_mut().enumerate() {
        let x = i as f64 - radius as f64;
        let val = (-0.5 * (x / sigma).powi(2)).exp();
        *k = val;
        kernel_sum += val;
    }

    let norm: Vec<f64> = kernel.iter().map(|k| k / kernel_sum).collect();
    (norm, sigma)
}

/// Apply a Gaussian kernel to an open list of 3D points. Endpoints are preserved.
/// Writes into a caller-provided buffer to reuse allocations across calls.
pub fn smooth_sub_segment(
    points: &[Point3D],
    kernel: &[f64],
    out: &mut Vec<Point3D>,
) {
    out.clear();
    let num_pts = points.len();
    if num_pts < 3 {
        out.extend_from_slice(points);
        return;
    }

    let kernel_radius = (kernel.len() - 1) / 2;
    out.push(points[0]);

    for i in 1..(num_pts - 1) {
        let mut new_x = 0.0;
        let mut new_y = 0.0;
        for (k_idx, k_weight) in kernel.iter().enumerate() {
            let p_idx = (i as i32 - kernel_radius as i32 + k_idx as i32)
                .clamp(0, num_pts as i32 - 1) as usize;
            let pt = points[p_idx];
            new_x += pt.x * k_weight;
            new_y += pt.y * k_weight;
        }
        out.push(Point3D::new(new_x, new_y, points[i].z));
    }

    out.push(points[num_pts - 1]);
}

/// Apply a wrapping Gaussian filter to a closed loop of points.
/// Writes into a caller-provided buffer to reuse allocations across calls.
pub fn smooth_circularly(
    points: &[Point3D],
    kernel: &[f64],
    out: &mut Vec<Point3D>,
) {
    out.clear();
    let num_pts = points.len();
    if num_pts < 3 {
        out.extend_from_slice(points);
        return;
    }

    let kernel_radius = (kernel.len() - 1) / 2;

    for i in 0..num_pts {
        let mut new_x = 0.0;
        let mut new_y = 0.0;
        for (k_idx, k_weight) in kernel.iter().enumerate() {
            let p_idx = (i as i32 - kernel_radius as i32 + k_idx as i32)
                .rem_euclid(num_pts as i32) as usize;
            let pt = points[p_idx];
            new_x += pt.x * k_weight;
            new_y += pt.y * k_weight;
        }
        out.push(Point3D::new(new_x, new_y, points[i].z));
    }

    if !out.is_empty() {
        out.push(out[0]);
    }
}

/// Smooth a polyline using Gaussian filtering with optional corner preservation.
///
/// Angles sharper than `corner_angle_threshold` are preserved as anchors
/// and not smoothed. The polyline is first resampled to ensure sufficient
/// point density for the Gaussian kernel.
pub fn smooth_polyline_3d(
    points: &[Point3D],
    amount: i32,
    corner_angle_threshold: f64,
    is_closed: Option<bool>,
) -> Vec<Point3D> {
    if points.len() < 3 || amount == 0 {
        return points.to_vec();
    }

    let (kernel, sigma) = compute_gaussian_kernel(amount);
    if kernel.len() <= 1 {
        return points.to_vec();
    }

    // Auto-detect closed paths if not specified
    let is_closed = is_closed.unwrap_or_else(|| {
        if points.len() >= 3 {
            let tol = 1e-6;
            points[0].distance(points[points.len() - 1]) < tol
        } else {
            false
        }
    });

    // Remove the duplicate endpoint for closed paths before resampling
    let work_points = if is_closed {
        &points[..points.len() - 1]
    } else {
        points
    };
    let max_len = (0.1_f64).max(sigma / 4.0);
    let mut prepared = Vec::new();
    resample_polyline_3d(work_points, max_len, is_closed, &mut prepared);
    let num_points = prepared.len();

    if num_points < 3 {
        return points.to_vec();
    }

    // Identify corners (sharp angles) to preserve
    let corner_threshold_rad = corner_angle_threshold.to_radians();
    let mut anchor_indices: Vec<usize> = Vec::new();

    if !is_closed {
        anchor_indices.push(0);
        anchor_indices.push(num_points - 1);
    }

    for i in 0..num_points {
        let p_prev = prepared[(i + num_points - 1) % num_points];
        let p_curr = prepared[i];
        let p_next = prepared[(i + 1) % num_points];
        let angle = get_angle_at_vertex(
            Point::new(p_prev.x, p_prev.y),
            Point::new(p_curr.x, p_curr.y),
            Point::new(p_next.x, p_next.y),
        );

        if angle < corner_threshold_rad
            && !approx_equal(angle, corner_threshold_rad)
            && !anchor_indices.contains(&i)
        {
            anchor_indices.push(i);
        }
    }

    anchor_indices.sort_unstable();
    let mut final_points: Vec<Point3D> = Vec::new();
    let mut smooth_buf: Vec<Point3D> = Vec::new();
    let mut sub_seg_buf: Vec<Point3D> = Vec::new();

    if is_closed {
        if anchor_indices.is_empty() {
            smooth_circularly(&prepared, &kernel, &mut smooth_buf);
            return smooth_buf;
        }

        let num_anchors = anchor_indices.len();
        for i in 0..num_anchors {
            let start_idx = anchor_indices[i];
            let end_idx = anchor_indices[(i + 1) % num_anchors];
            sub_seg_buf.clear();
            if start_idx < end_idx {
                sub_seg_buf.extend_from_slice(&prepared[start_idx..=end_idx]);
            } else {
                sub_seg_buf.extend_from_slice(&prepared[start_idx..]);
                sub_seg_buf.extend_from_slice(&prepared[..=end_idx]);
            }
            smooth_sub_segment(&sub_seg_buf, &kernel, &mut smooth_buf);
            for p in smooth_buf.iter().take(smooth_buf.len() - 1) {
                final_points.push(*p);
            }
        }

        if !final_points.is_empty() {
            final_points.push(final_points[0]);
        }
        final_points
    } else {
        if anchor_indices.len() < 2 {
            smooth_sub_segment(&prepared, &kernel, &mut smooth_buf);
            return smooth_buf;
        }

        let mut last_anchor = anchor_indices[0];
        for &anchor_idx in anchor_indices.iter().skip(1) {
            sub_seg_buf.clear();
            sub_seg_buf.extend_from_slice(&prepared[last_anchor..=anchor_idx]);
            smooth_sub_segment(&sub_seg_buf, &kernel, &mut smooth_buf);
            for p in smooth_buf.iter().take(smooth_buf.len() - 1) {
                final_points.push(*p);
            }
            last_anchor = anchor_idx;
        }

        final_points.push(prepared[num_points - 1]);
        final_points
    }
}

/// Iteratively remove interior waypoints whose direct connection
/// (prev → next) is collision-free, repeating until no more points
/// can be removed.  Endpoints are always preserved.
pub(crate) fn shortcut_path(
    points: &[Point],
    obstacles: &[Polygon],
    obstacle_bounds: &[Rect],
    clearance: f64,
) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut current = points.to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        let mut i = 1;
        while i < current.len() - 1 {
            let prev = current[i - 1];
            let next = current[i + 1];
            let seg = [prev, next];
            if !does_path_sweep_intersect_polygon(
                &seg,
                clearance,
                obstacles,
                obstacle_bounds,
            ) {
                current.remove(i);
                changed = true;
            } else {
                i += 1;
            }
        }
    }
    current
}

/// Smooth a polyline while maintaining a minimum clearance from
/// obstacle polygons.
///
/// The function operates in two phases:
///
/// 1. **Shortcut** – greedily remove intermediate waypoints whose direct
///    connection is collision-free, producing a minimal-hop path.
///
/// 2. **Constrained Gaussian relaxation** – resample the shortcut path
///    for kernel coverage, then iteratively apply Gaussian smoothing
///    ([`smooth_sub_segment`]).  After each smoothing pass, any point
///    whose smoothed position would violate the clearance constraint is
///    reverted to its pre-smoothing location.  Iteration continues until
///    the path stabilises or `max_iterations` (10) is reached.
///
/// `smoothing_amount` uses the same 0–200 scale as
/// [`smooth_polyline`].  Pass `0` to apply shortcut only.
///
/// Endpoints are always preserved.
pub fn smooth_path(
    points: &[Point],
    obstacles: &[Polygon],
    clearance: f64,
    smoothing_amount: i32,
) -> Vec<Point> {
    let n = points.len();
    if n <= 2 {
        return points.to_vec();
    }

    // Phase 1: shortcut.
    let obs_bounds =
        crate::geo::shape::polygon::compute_polygon_bounds(obstacles);
    let shortcut = shortcut_path(points, obstacles, &obs_bounds, clearance);

    if shortcut.len() < 3 || smoothing_amount == 0 {
        return shortcut;
    }

    // Phase 2: constrained Gaussian relaxation.
    let (kernel, sigma) = compute_gaussian_kernel(smoothing_amount);
    if kernel.len() <= 1 {
        return shortcut;
    }

    let max_len = (0.1_f64).max(sigma / 4.0);

    let pts_3d: Vec<Point3D> = shortcut
        .iter()
        .map(|p| Point3D::new(p.x, p.y, 0.0))
        .collect();
    let mut prepared: Vec<Point3D> = Vec::new();
    resample_polyline_3d(&pts_3d, max_len, false, &mut prepared);

    let num = prepared.len();
    if num < 3 {
        return shortcut;
    }

    let mut current = prepared.clone();
    let mut smoothed: Vec<Point3D> = Vec::with_capacity(num);

    for _ in 0..10 {
        smooth_sub_segment(&current, &kernel, &mut smoothed);

        let mut moved = false;
        for i in 1..num - 1 {
            let prev = Point::new(current[i - 1].x, current[i - 1].y);
            let next = Point::new(current[i + 1].x, current[i + 1].y);
            let candidate = Point::new(smoothed[i].x, smoothed[i].y);
            let tri = [prev, candidate, next];
            if !does_path_sweep_intersect_polygon(
                &tri,
                clearance,
                obstacles,
                &obs_bounds,
            ) {
                let dx = smoothed[i].x - current[i].x;
                let dy = smoothed[i].y - current[i].y;
                if dx * dx + dy * dy > 1e-4 {
                    moved = true;
                }
                current[i] = smoothed[i];
            }
        }

        if !moved {
            break;
        }
    }

    current.iter().map(|p| Point::new(p.x, p.y)).collect()
}

/// Round sharp corners in a path using Chaikin corner cutting with
/// collision checking.  Corners sharper than 45° are cut;
/// gently curving sections are left untouched.
#[prof]
pub fn chaikin_corner_cut(
    points: &[Point],
    obstacles: &[Polygon],
    obstacle_bounds: &[Rect],
    clearance: f64,
    iterations: usize,
) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut current = points.to_vec();
    for _ in 0..iterations {
        if current.len() < 3 {
            break;
        }
        let mut next: Vec<Point> = vec![current[0]];
        for i in 1..current.len() - 1 {
            let prev = current[i - 1];
            let curr = current[i];
            let after = current[i + 1];
            let v1x = curr.x - prev.x;
            let v1y = curr.y - prev.y;
            let v2x = after.x - curr.x;
            let v2y = after.y - curr.y;
            let l1 = (v1x * v1x + v1y * v1y).sqrt();
            let l2 = (v2x * v2x + v2y * v2y).sqrt();
            if l1 < 1e-12 || l2 < 1e-12 {
                next.push(curr);
                continue;
            }
            let dot = (v1x * v2x + v1y * v2y) / (l1 * l2);
            if dot < 0.707 {
                let q_back = Point::new(
                    curr.x * 0.75 + prev.x * 0.25,
                    curr.y * 0.75 + prev.y * 0.25,
                );
                let q_fwd = Point::new(
                    curr.x * 0.75 + after.x * 0.25,
                    curr.y * 0.75 + after.y * 0.25,
                );
                let tri = [prev, q_back, q_fwd, after];
                if !does_path_sweep_intersect_polygon(
                    &tri,
                    clearance,
                    obstacles,
                    obstacle_bounds,
                ) {
                    next.push(q_back);
                    next.push(q_fwd);
                } else {
                    next.push(curr);
                }
            } else {
                next.push(curr);
            }
        }
        next.push(*current.last().unwrap());
        current = next;
    }
    current
}

/// Build a smooth path between two points using multi-stage processing.
///
/// Pipeline:
/// 1. Resample the base path for point density.
/// 2. Iteratively shortcut removable waypoints (collision-checked).
/// 3. Multi-scale resampling: alternate coarse / fine arc-length
///    resampling with light Gaussian smoothing.  Each coarse pass
///    at a different interval may skip V-vertices, progressively
///    diluting sharp corners into gentler curves.
/// 4. Final Gaussian smoothing pass.
#[prof]
pub fn build_smoothed_path(
    last: Point,
    first: Point,
    waypoints: &[Point],
    uncleared: &[Polygon],
    obstacle_bounds: &[Rect],
    clearance: f64,
    smoothing_amount: i32,
) -> Vec<Point> {
    // 1. Full path — prepend last / append first.
    let original: Vec<Point> = if waypoints.len() < 2 {
        vec![last, first]
    } else {
        let mut full = Vec::with_capacity(waypoints.len() + 2);
        full.push(last);
        full.extend_from_slice(waypoints);
        full.push(first);
        full
    };
    if original.len() < 3 {
        return original;
    }

    // 2. Shortcut the ORIGINAL path first (no resample).  If the
    //    shortcut can't remove any points, the path is fully
    //    constrained by obstacles and further processing (resample,
    //    Gaussian smooth) would only add redundant points without
    //    improving the path.  Return the simplified original.
    let shortcutted =
        shortcut_path(&original, uncleared, obstacle_bounds, clearance);
    if shortcutted.len() >= original.len() {
        return shortcutted;
    }

    // 3. Resample the shortened path for smoothing.
    let max_seg = clearance.max(0.5) * 0.5;
    let mut link = resample_polyline_2d(&shortcutted, max_seg);
    link = resample_polyline_2d(&link, max_seg);
    if link.len() < 3 {
        return shortcutted;
    }

    // 4. Aggressive Gaussian smoothing with PER-POINT collision
    //    checking.  Each point is individually tested: if its smoothed
    //    position maintains clearance, it is accepted; otherwise it
    //    stays put.  This allows V-shapes in open areas to be fully
    //    rounded while points near walls are preserved.
    let amt = smoothing_amount.max(120);
    let (kernel, sigma) = compute_gaussian_kernel(amt);
    if kernel.len() > 1 {
        let max_len = 0.1_f64.max(sigma / 4.0);
        let pts_3d: Vec<Point3D> =
            link.iter().map(|p| Point3D::new(p.x, p.y, 0.0)).collect();
        let mut current: Vec<Point3D> = Vec::new();
        resample_polyline_3d(&pts_3d, max_len, false, &mut current);
        let n = current.len();
        if n >= 3 {
            let mut buf: Vec<Point3D> = Vec::with_capacity(n);
            for _ in 0..100 {
                smooth_sub_segment(&current, &kernel, &mut buf);
                let mut moved = false;
                for i in 1..n - 1 {
                    let prev = Point::new(current[i - 1].x, current[i - 1].y);
                    let next = Point::new(current[i + 1].x, current[i + 1].y);
                    let candidate = Point::new(buf[i].x, buf[i].y);
                    let tri = [prev, candidate, next];
                    if !does_path_sweep_intersect_polygon(
                        &tri,
                        clearance,
                        uncleared,
                        obstacle_bounds,
                    ) {
                        let dx = buf[i].x - current[i].x;
                        let dy = buf[i].y - current[i].y;
                        if dx * dx + dy * dy > 1e-4 {
                            moved = true;
                        }
                        current[i] = buf[i];
                    }
                }
                if !moved {
                    break;
                }
            }
            link = current.iter().map(|p| Point::new(p.x, p.y)).collect();
        }
    }

    link
}

/// Insert tangent extension points at both ends of a connecting polyline
/// to ensure it meets the adjacent polylines tangentially (G1 continuity
/// at the junctions).
///
/// `prev_tail` — the last 2+ points of the preceding polyline (used to
///   compute the exit tangent direction).
/// `next_head` — the first 2+ points of the following polyline (used to
///   compute the entry tangent direction).
/// `margin` — controls how far the extension points are placed along
///   the tangent direction (clamped to a fraction of the adjacent
///   segment length).
///
/// Each end is processed independently: if the angle between the
/// tangent direction and the polyline direction at the junction exceeds
/// ≈ 25° (dot < 0.9), an intermediate point is inserted along the
/// tangent to move the angle discontinuity away from the junction.
#[prof]
pub fn blend_tangent(
    link: &mut Vec<Point>,
    prev_tail: &[Point],
    next_head: &[Point],
    margin: f64,
) {
    // Start junction: extend the previous polyline's tangent forward
    // into the connecting polyline.
    if link.len() >= 2 && prev_tail.len() >= 2 {
        let prev_pt = prev_tail[prev_tail.len() - 2];
        let curr = link[0];
        let nxt = link[1];
        let tx = curr.x - prev_pt.x;
        let ty = curr.y - prev_pt.y;
        let tlen = (tx * tx + ty * ty).sqrt();
        let dx = nxt.x - curr.x;
        let dy = nxt.y - curr.y;
        let dlen = (dx * dx + dy * dy).sqrt();
        if tlen > 1e-12 && dlen > 1e-12 {
            let dot = (tx * dx + ty * dy) / (tlen * dlen);
            if dot < 0.9 {
                let d = margin.max(2.0).min(dlen * 0.4);
                link.insert(
                    1,
                    Point::new(curr.x + tx / tlen * d, curr.y + ty / tlen * d),
                );
            }
        }
    }

    // End junction: extend the next polyline's tangent backward into
    // the connecting polyline.
    if link.len() >= 2 && next_head.len() >= 2 {
        let prev = link[link.len() - 2];
        let curr = link[link.len() - 1];
        let next = next_head[1];
        let tx = next.x - curr.x;
        let ty = next.y - curr.y;
        let tlen = (tx * tx + ty * ty).sqrt();
        let dx = curr.x - prev.x;
        let dy = curr.y - prev.y;
        let dlen = (dx * dx + dy * dy).sqrt();
        if tlen > 1e-12 && dlen > 1e-12 {
            let dot = (tx * dx + ty * dy) / (tlen * dlen);
            if dot < 0.9 {
                let d = margin.max(2.0).min(dlen * 0.4);
                let last_idx = link.len() - 1;
                link.insert(
                    last_idx,
                    Point::new(curr.x - tx / tlen * d, curr.y - ty / tlen * d),
                );
            }
        }
    }
}

fn approx_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}
