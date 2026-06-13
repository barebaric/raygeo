//! Minkowski: Minkowski sum/difference operations for polygons.
//!
//! This module provides functions for calculating Minkowski sums and differences
//! of polygons, which are used in packing and nesting algorithms for computing
//! No-Fit Polygons (NFP) and Inner-Fit Polygons (IFP).

use crate::geo::shape::polygon::{
    get_polygon_bounds, get_polygon_convex_hull, int_path_to_polygon,
    polygon_to_int_path,
};
use crate::types::{IntPolygon, Point, Polygon};

pub fn convolve_two_segments(
    a1: (i64, i64),
    a2: (i64, i64),
    b1: (i64, i64),
    b2: (i64, i64),
) -> Vec<(i64, i64)> {
    vec![
        (a1.0 + b2.0, a1.1 + b2.1),
        (a1.0 + b1.0, a1.1 + b1.1),
        (a2.0 + b1.0, a2.1 + b1.1),
        (a2.0 + b2.0, a2.1 + b2.1),
    ]
}

pub fn convolve_point_sequences(
    seq_a: &IntPolygon,
    seq_b: &IntPolygon,
) -> Vec<IntPolygon> {
    let mut parallelograms: Vec<IntPolygon> = Vec::new();
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
        for &(x, y) in poly {
            max_abs = max_abs.max(x.abs()).max(y.abs());
        }
    }
    if max_abs < 1.0 {
        max_abs = 1.0;
    }
    0.1 * (max_int as f64) / max_abs
}

pub fn get_polygon_minkowski_sum_convex(
    poly_a: &IntPolygon,
    poly_b: &IntPolygon,
) -> Vec<IntPolygon> {
    if poly_a.is_empty() || poly_b.is_empty() {
        return vec![];
    }
    let mut all_points: Vec<Point> = Vec::new();
    for p1 in poly_a {
        for p2 in poly_b {
            all_points
                .push((p1.0 as f64 + p2.0 as f64, p1.1 as f64 + p2.1 as f64));
        }
    }
    let hull = get_polygon_convex_hull(&all_points);
    if hull.len() < 3 {
        return vec![];
    }
    vec![hull.iter().map(|(x, y)| (*x as i64, *y as i64)).collect()]
}

/// Calculate the No-Fit Polygon (NFP) for two polygons.
///
/// Assumes polygons are convex for performance.
pub fn get_no_fit_polygon(
    stationary: &Polygon,
    orbiting: &Polygon,
) -> Vec<Polygon> {
    if stationary.is_empty() || orbiting.is_empty() {
        return vec![];
    }
    let static_path = polygon_to_int_path(stationary);
    let orbiting_path = polygon_to_int_path(orbiting);
    let orbiting_negated: IntPolygon =
        orbiting_path.iter().map(|(x, y)| (-*x, -*y)).collect();
    let nfp_paths =
        get_polygon_minkowski_sum_convex(&static_path, &orbiting_negated);
    let mut results = Vec::new();
    let first_pt = orbiting_path[0];
    for path in nfp_paths {
        let shifted: IntPolygon = path
            .iter()
            .map(|(x, y)| (*x + first_pt.0, *y + first_pt.1))
            .collect();
        results.push(int_path_to_polygon(&shifted));
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
        (ifp_min_x, ifp_min_y),
        (ifp_max_x, ifp_min_y),
        (ifp_max_x, ifp_max_y),
        (ifp_min_x, ifp_max_y),
    ];
    vec![ifp]
}
