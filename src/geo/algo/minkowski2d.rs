//! Minkowski: Minkowski sum/difference operations for polygons.
//!
//! **Planar (XY-plane only).** Z is not modeled.
//!
//! This module provides functions for calculating Minkowski sums and differences
//! of polygons, which are used in packing and nesting algorithms for computing
//! No-Fit Polygons (NFP) and Inner-Fit Polygons (IFP).

use crate::geo::shape::polygon::{get_polygon_bounds, get_polygon_convex_hull};
use crate::types::{Point, Polygon};

pub fn convolve_two_segments(
    a1: Point,
    a2: Point,
    b1: Point,
    b2: Point,
) -> Vec<Point> {
    vec![
        Point::new(a1.x + b2.x, a1.y + b2.y),
        Point::new(a1.x + b1.x, a1.y + b1.y),
        Point::new(a2.x + b1.x, a2.y + b1.y),
        Point::new(a2.x + b2.x, a2.y + b2.y),
    ]
}

pub fn convolve_point_sequences(
    seq_a: &Polygon,
    seq_b: &Polygon,
) -> Vec<Polygon> {
    let mut parallelograms: Vec<Polygon> = Vec::new();
    if seq_a.len() < 2 || seq_b.len() < 2 {
        return parallelograms;
    }
    let n = seq_a.len();
    let m = seq_b.len();
    for i in 0..n {
        let p_a1 = seq_a[(i + n - 1) % n];
        let p_a2 = seq_a[i];
        for j in 0..m {
            let p_b1 = seq_b[(j + m - 1) % m];
            let p_b2 = seq_b[j];
            parallelograms.push(convolve_two_segments(p_a1, p_a2, p_b1, p_b2));
        }
    }
    parallelograms
}

pub fn calculate_input_scale(polygons: &[Polygon], max_int: i64) -> f64 {
    if polygons.is_empty() {
        return 0.1 * (max_int as f64);
    }
    let mut max_abs = 0.0f64;
    for poly in polygons {
        for p in poly {
            max_abs = max_abs.max(p.x.abs()).max(p.y.abs());
        }
    }
    if max_abs < 1.0 {
        max_abs = 1.0;
    }
    0.1 * (max_int as f64) / max_abs
}

pub fn get_polygon_minkowski_sum_convex(
    poly_a: &Polygon,
    poly_b: &Polygon,
) -> Vec<Polygon> {
    if poly_a.is_empty() || poly_b.is_empty() {
        return vec![];
    }
    let mut all_points: Vec<Point> = Vec::new();
    for p1 in poly_a {
        for p2 in poly_b {
            all_points.push(Point::new(p1.x + p2.x, p1.y + p2.y));
        }
    }
    let hull = get_polygon_convex_hull(&all_points);
    if hull.len() < 3 {
        return vec![];
    }
    vec![hull]
}

/// Calculate the No-Fit Polygon (NFP) for two polygons.
///
/// Assumes polygons are convex for performance.
pub fn get_no_fit_polygon(
    stationary: &Polygon,
    orbiting: &Polygon,
) -> Vec<Polygon> {
    if stationary.len() < 3 || orbiting.len() < 3 {
        return vec![];
    }
    let orbiting_negated: Polygon =
        orbiting.iter().map(|p| Point::new(-p.x, -p.y)).collect();
    let nfp_paths =
        get_polygon_minkowski_sum_convex(stationary, &orbiting_negated);
    let mut results = Vec::new();
    let first_pt = orbiting[0];
    for path in nfp_paths {
        if path.len() >= 3 {
            let shifted: Polygon = path
                .iter()
                .map(|p| Point::new(p.x + first_pt.x, p.y + first_pt.y))
                .collect();
            results.push(shifted);
        }
    }
    results
}

/// Calculate the Inner-Fit Polygon (IFP) using a simple formula based on
/// bounding boxes, which is exact for axis-aligned rectangles and a robust
/// approximation for other convex shapes.
pub fn get_inner_fit_polygon(
    container: &Polygon,
    part: &Polygon,
) -> Vec<Polygon> {
    if container.is_empty() || part.is_empty() {
        return vec![];
    }
    let c_rect = get_polygon_bounds(container);
    let p_rect = get_polygon_bounds(part);
    let p_width = p_rect.2 - p_rect.0;
    let p_height = p_rect.3 - p_rect.1;
    let c_width = c_rect.2 - c_rect.0;
    let c_height = c_rect.3 - c_rect.1;
    if p_width > c_width + 1e-9 || p_height > c_height + 1e-9 {
        return vec![];
    }
    let ifp_min_x = c_rect.0 - p_rect.0;
    let ifp_max_x = c_rect.2 - p_rect.2;
    let ifp_min_y = c_rect.1 - p_rect.1;
    let ifp_max_y = c_rect.3 - p_rect.3;
    if ifp_min_x > ifp_max_x || ifp_min_y > ifp_max_y {
        return vec![];
    }
    let ifp = vec![
        Point::new(ifp_min_x, ifp_min_y),
        Point::new(ifp_max_x, ifp_min_y),
        Point::new(ifp_max_x, ifp_max_y),
        Point::new(ifp_min_x, ifp_max_y),
    ];
    vec![ifp]
}
