//! Smooth: Polyline smoothing using Gaussian filtering.
//!
//! Provides functions for computing Gaussian kernels and applying them
//! to open or closed polylines with optional corner preservation.

use crate::geo::shape::line::get_angle_at_vertex;
use crate::types::{Point, Point3D};

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
            new_x += pt.0 * k_weight;
            new_y += pt.1 * k_weight;
        }
        out.push(Point3D(new_x, new_y, points[i].2));
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
            new_x += pt.0 * k_weight;
            new_y += pt.1 * k_weight;
        }
        out.push(Point3D(new_x, new_y, points[i].2));
    }

    if !out.is_empty() {
        out.push(out[0]);
    }
}

/// Resample a polyline so that no segment is longer than `max_segment_length`.
/// New points are added by linear interpolation along existing segments.
/// Writes into a caller-provided buffer to reuse allocations across calls.
pub fn resample_polyline(
    points: &[Point3D],
    max_segment_length: f64,
    is_closed: bool,
    out: &mut Vec<Point3D>,
) {
    out.clear();
    if points.is_empty() {
        return;
    }

    out.push(points[0]);
    let num_segments = if is_closed {
        points.len()
    } else {
        points.len() - 1
    };

    for i in 0..num_segments {
        let p1 = points[i];
        let p2 = points[(i + 1) % points.len()];
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let dist = dx.hypot(dy);

        if dist > max_segment_length {
            let num_sub = (dist / max_segment_length).ceil() as i32;
            for j in 1..num_sub {
                let t = j as f64 / num_sub as f64;
                let px = p1.0 * (1.0 - t) + p2.0 * t;
                let py = p1.1 * (1.0 - t) + p2.1 * t;
                out.push(Point3D(px, py, p1.2));
            }
        }

        if !(is_closed && i == num_segments - 1) {
            out.push(p2);
        }
    }
}

/// Smooth a polyline using Gaussian filtering with optional corner preservation.
///
/// Angles sharper than `corner_angle_threshold` are preserved as anchors
/// and not smoothed. The polyline is first resampled to ensure sufficient
/// point density for the Gaussian kernel.
pub fn smooth_polyline(
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
            (points[0].0 - points[points.len() - 1].0)
                .hypot(points[0].1 - points[points.len() - 1].1)
                < tol
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
    resample_polyline(work_points, max_len, is_closed, &mut prepared);
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
            Point(p_prev.0, p_prev.1),
            Point(p_curr.0, p_curr.1),
            Point(p_next.0, p_next.1),
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

fn approx_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}
