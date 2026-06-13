use crate::geo::algo::minkowski::{
    convolve_point_sequences, get_polygon_minkowski_sum_convex,
};
use crate::geo::shape::polygon::{
    get_polygon_signed_area, get_polygons_union, int_path_to_polygon,
    is_polygon_convex, polygon_to_int_path,
};
use crate::types::{IntPolygon, Point, Polygon};
use crate::CLIPPER_SCALE;

/// Scale factor for converting polygon coordinates to integer hash keys.
const POLYGON_KEY_SCALE: f64 = 10000.0;

pub fn no_fit_polygon(
    static_poly: &Polygon,
    orbiting: &Polygon,
) -> Vec<Polygon> {
    if static_poly.len() < 3 || orbiting.len() < 3 {
        return vec![];
    }

    let static_int = polygon_to_int_path(static_poly);
    let orbiting_int = polygon_to_int_path(orbiting);

    if is_polygon_convex(static_poly) && is_polygon_convex(orbiting) {
        nfp_convex_fast(&static_int, &orbiting_int)
    } else {
        nfp_minkowski(&static_int, &orbiting_int)
    }
}

pub fn nfp_convex_fast(
    static_poly: &IntPolygon,
    orbiting: &IntPolygon,
) -> Vec<Polygon> {
    if static_poly.len() < 3 || orbiting.len() < 3 {
        return vec![];
    }

    let x_shift = orbiting[0].0;
    let y_shift = orbiting[0].1;

    let orbiting_negated: IntPolygon =
        orbiting.iter().map(|(x, y)| (-x, -y)).collect();

    let nfp_paths =
        get_polygon_minkowski_sum_convex(static_poly, &orbiting_negated);

    let mut results = Vec::new();
    for path in &nfp_paths {
        if path.len() >= 3 {
            let shifted: IntPolygon = path
                .iter()
                .map(|(x, y)| (*x + x_shift, *y + y_shift))
                .collect();
            results.push(int_path_to_polygon(&shifted));
        }
    }
    results
}

pub fn nfp_minkowski(
    static_poly: &IntPolygon,
    orbiting: &IntPolygon,
) -> Vec<Polygon> {
    if static_poly.len() < 3 || orbiting.len() < 3 {
        return vec![];
    }

    let x_shift = orbiting[0].0;
    let y_shift = orbiting[0].1;

    let orbiting_negated: IntPolygon =
        orbiting.iter().map(|(x, y)| (-x, -y)).collect();

    let static_float = int_path_to_polygon(static_poly);
    let orbiting_neg_float = int_path_to_polygon(&orbiting_negated);

    let mut subjects: Vec<Polygon> = Vec::new();

    let parallelograms =
        convolve_point_sequences(static_poly, &orbiting_negated);
    for para in &parallelograms {
        if para.len() >= 3 {
            subjects.push(int_path_to_polygon(para));
        }
    }

    if !static_float.is_empty() && !orbiting_neg_float.is_empty() {
        let first_on = orbiting_neg_float[0];
        let shifted: Polygon = static_float
            .iter()
            .map(|p| Point(p.0 + first_on.0, p.1 + first_on.1))
            .collect();
        subjects.push(shifted);

        let first_s = static_float[0];
        let neg_shifted: Polygon = orbiting_neg_float
            .iter()
            .map(|p| Point(p.0 + first_s.0, p.1 + first_s.1))
            .collect();
        subjects.push(neg_shifted);
    }

    if subjects.is_empty() {
        return vec![];
    }

    let union_result = get_polygons_union(&subjects);

    let x_shift_f = x_shift as f64 / CLIPPER_SCALE;
    let y_shift_f = y_shift as f64 / CLIPPER_SCALE;

    let mut results = Vec::new();
    for poly in &union_result {
        if poly.len() >= 3 {
            let shifted: Polygon = poly
                .iter()
                .map(|p| Point(p.0 + x_shift_f, p.1 + y_shift_f))
                .collect();

            let area = get_polygon_signed_area(&shifted);
            if area > 0.0 {
                results.push(shifted);
            }
        }
    }

    results
}

pub fn normalize_polygon(poly: &Polygon) -> (Polygon, f64, f64) {
    if poly.is_empty() {
        return (poly.clone(), 0.0, 0.0);
    }
    let min_x = poly.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let min_y = poly.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let normalized: Polygon = poly
        .iter()
        .map(|p| Point(p.0 - min_x, p.1 - min_y))
        .collect();
    (normalized, min_x, min_y)
}

pub fn polygon_to_key(poly: &Polygon) -> Vec<(i64, i64)> {
    poly.iter()
        .map(|p| {
            (
                (p.0 * POLYGON_KEY_SCALE).round() as i64,
                (p.1 * POLYGON_KEY_SCALE).round() as i64,
            )
        })
        .collect()
}
