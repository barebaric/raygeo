//! Bezier: Cubic Bezier curve operations.
//!
//! This module provides functions for working with cubic Bezier curves including:
//! - Point evaluation at parameter t
//! - Curve subdivision
//! - Bounding box computation
//! - Intersection with rectangles
//! - Clipping to rectangular regions
//! - Linearization into line segments
//! - Conversion to quadratic approximation

use std::f64::consts::PI;

use glam::{DVec2, DVec3};

use crate::constants::EPSILON_INTERSECT;
use crate::geo::algo::interp::solve_quadratic;
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::geo::shape::point::midpoint;
use crate::geo::shape::polygon::is_point_inside_polygon;
use crate::types::{CubicBezier, Point, Point3D, Polygon, Polygon3D, Rect};

/// Evaluates a cubic Bezier curve at parameter t [0, 1] using the Bernstein polynomial.
pub fn get_bezier_point_at(
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    t: f64,
) -> Point {
    let complement = 1.0 - t;
    let x = complement.powi(3) * p0.x
        + 3.0 * complement.powi(2) * t * c1.x
        + 3.0 * complement * t.powi(2) * c2.x
        + t.powi(3) * p1.x;
    let y = complement.powi(3) * p0.y
        + 3.0 * complement.powi(2) * t * c1.y
        + 3.0 * complement * t.powi(2) * c2.y
        + t.powi(3) * p1.y;
    Point::new(x, y)
}

/// Splits a Bezier curve into two sub-curves at parameter t.
/// Uses de Casteljau's algorithm for subdivision.
pub fn split_bezier(
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    t: f64,
) -> (CubicBezier, CubicBezier) {
    let mid_p0_c1 = _lerp2(p0, c1, t);
    let mid_c1_c2 = _lerp2(c1, c2, t);
    let mid_c2_p1 = _lerp2(c2, p1, t);
    let mid_p0c1_c1c2 = _lerp2(mid_p0_c1, mid_c1_c2, t);
    let mid_c1c2_c2p1 = _lerp2(mid_c1_c2, mid_c2_p1, t);
    let split_point = _lerp2(mid_p0c1_c1c2, mid_c1c2_c2p1, t);

    let left = CubicBezier(p0, mid_p0_c1, mid_p0c1_c1c2, split_point);
    let right = CubicBezier(split_point, mid_c1c2_c2p1, mid_c2_p1, p1);
    (left, right)
}

/// Computes the axis-aligned bounding box of a Bezier curve.
/// Finds extrema analytically by solving for points where the derivative is zero.
pub fn get_bezier_bounds(p0: Point, c1: Point, c2: Point, p1: Point) -> Rect {
    let mut candidates_x = vec![p0.x, p1.x];
    let mut candidates_y = vec![p0.y, p1.y];
    _add_axis_extrema(&mut candidates_x, p0.x, c1.x, c2.x, p1.x);
    _add_axis_extrema(&mut candidates_y, p0.y, c1.y, c2.y, p1.y);

    Rect::new(
        candidates_x.iter().cloned().fold(f64::INFINITY, f64::min),
        candidates_y.iter().cloned().fold(f64::INFINITY, f64::min),
        candidates_x
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max),
        candidates_y
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max),
    )
}

/// Tests if a Bezier curve is fully contained within all specified regions.
/// Samples key points (corners of bbox, endpoints, midpoint) for containment check.
pub fn is_bezier_inside_polygons(
    start_pos: Point,
    c1: Point,
    c2: Point,
    end_pos: Point,
    regions: &[Polygon],
) -> bool {
    let bbox = get_bezier_bounds(start_pos, c1, c2, end_pos);
    let mid = get_bezier_point_at(start_pos, c1, c2, end_pos, 0.5);

    let sample_points: Vec<Point> = vec![
        bbox.min,
        Point::new(bbox.max.x, bbox.min.y),
        bbox.max,
        Point::new(bbox.min.x, bbox.max.y),
        start_pos,
        end_pos,
        mid,
    ];

    for p in sample_points {
        if !regions
            .iter()
            .any(|region| is_point_inside_polygon(p, region))
        {
            return false;
        }
    }
    true
}

/// Finds all t parameters where a Bezier curve intersects a rectangle.
/// Solves the cubic equation for each edge of the rectangle.
pub fn get_bezier_rect_intersections(
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    rect: Rect,
) -> Vec<f64> {
    let mut t_crossings: Vec<f64> = Vec::new();
    let rect_edges: [(usize, f64); 4] = [
        (0, rect.min.x),
        (0, rect.max.x),
        (1, rect.min.y),
        (1, rect.max.y),
    ];

    for (axis_idx, edge_val) in rect_edges {
        let p0_coord = if axis_idx == 0 { p0.x } else { p0.y };
        let c1_coord = if axis_idx == 0 { c1.x } else { c1.y };
        let c2_coord = if axis_idx == 0 { c2.x } else { c2.y };
        let p1_coord = if axis_idx == 0 { p1.x } else { p1.y };

        let poly_a = p0_coord;
        let poly_b = 3.0 * (c1_coord - p0_coord);
        let poly_c = 3.0 * (c2_coord - c1_coord) - poly_b;
        let poly_d = p1_coord - poly_a - poly_b - poly_c;

        let roots = _solve_cubic(poly_d, poly_c, poly_b, poly_a - edge_val);
        for root in roots {
            if (-1e-9..=1.0 + 1e-9).contains(&root) {
                let clamped = root.clamp(0.0, 1.0);
                let point_on_curve =
                    get_bezier_point_at(p0, c1, c2, p1, clamped);
                let other_axis = 1 - axis_idx;
                let other_coord = if other_axis == 1 {
                    point_on_curve.y
                } else {
                    point_on_curve.x
                };
                let axis_lo = if other_axis == 1 {
                    rect.min.y
                } else {
                    rect.min.x
                };
                let axis_hi = if other_axis == 1 {
                    rect.max.y
                } else {
                    rect.max.x
                };
                if axis_lo - 1e-9 <= other_coord
                    && other_coord <= axis_hi + 1e-9
                {
                    let rounded = (clamped * 1e12).round() / 1e12;
                    if !t_crossings.contains(&rounded) {
                        t_crossings.push(rounded);
                    }
                }
            }
        }
    }

    if !t_crossings.contains(&0.0) {
        t_crossings.push(0.0);
    }
    if !t_crossings.contains(&1.0) {
        t_crossings.push(1.0);
    }

    t_crossings
        .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    t_crossings
}

pub fn clip_bezier_with_rect(
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    rect: Rect,
) -> Vec<CubicBezier> {
    let crossing_params = get_bezier_rect_intersections(p0, c1, c2, p1, rect);
    if crossing_params.len() < 2 {
        return vec![];
    }

    // Extract segments that fall inside the rectangle
    let mut inside_segments: Vec<CubicBezier> = vec![];

    for i in 0..crossing_params.len() - 1 {
        let t_start = crossing_params[i];
        let t_end = crossing_params[i + 1];
        if (t_end - t_start).abs() < 1e-12 {
            continue;
        }
        let t_mid = (t_start + t_end) / 2.0;
        let midpoint_pt = get_bezier_point_at(p0, c1, c2, p1, t_mid);

        // Check if midpoint is inside rect
        if rect.min.x - 1e-9 <= midpoint_pt.x
            && midpoint_pt.x <= rect.max.x + 1e-9
            && rect.min.y - 1e-9 <= midpoint_pt.y
            && midpoint_pt.y <= rect.max.y + 1e-9
        {
            let segment = _extract_subsegment(p0, c1, c2, p1, t_start, t_end);
            inside_segments.push(segment);
        }
    }

    inside_segments
}

/// Approximates a cubic Bezier curve with a quadratic (single control point).
/// Uses the least-squares optimal approximation:
/// Q = (3*C1 + 3*C2 - P0 - P1) / 4
pub fn convert_cubic_bezier_to_quadratic(
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
) -> (Point, Point, Point) {
    let quadratic_control = Point::new(
        (3.0 * c1.x + 3.0 * c2.x - p0.x - p1.x) / 4.0,
        (3.0 * c1.y + 3.0 * c2.y - p0.y - p1.y) / 4.0,
    );
    (p0, quadratic_control, p1)
}

pub fn get_bezier_closest_point(
    end: Point3D,
    control1: Point3D,
    control2: Point3D,
    start_pos: Point3D,
    x: f64,
    y: f64,
) -> Option<(f64, Point, f64)> {
    // Linearize and search for closest point on segments
    let bezier_segments =
        linearize_bezier_from_params(end, control1, control2, start_pos, 0.005);
    if bezier_segments.is_empty() {
        return None;
    }

    let mut min_dist_sq = f64::INFINITY;
    let mut best_result: Option<(usize, f64, Point, f64)> = None;

    for (seg_idx, (seg_start, seg_end)) in bezier_segments.iter().enumerate() {
        let t_sub = get_line_segment_closest_point(
            Point::new(seg_start.x, seg_start.y),
            Point::new(seg_end.x, seg_end.y),
            x,
            y,
        );
        if t_sub.2 < min_dist_sq {
            min_dist_sq = t_sub.2;
            best_result = Some((seg_idx, t_sub.0, t_sub.1, t_sub.2));
        }
    }

    best_result.map(|(best_seg_idx, best_t_sub, best_pt, best_dist_sq)| {
        let t_bezier =
            (best_seg_idx as f64 + best_t_sub) / bezier_segments.len() as f64;
        (t_bezier, best_pt, best_dist_sq)
    })
}

/// Converts a Bezier curve from parameters into line segments.
/// Estimates number of steps based on control point distances and resolution.
pub fn linearize_bezier_from_params(
    end: Point3D,
    control1: Point3D,
    control2: Point3D,
    start_point: Point3D,
    resolution: f64,
) -> Vec<(Point3D, Point3D)> {
    let p0 = start_point;
    let p1 = end;
    let c1_2d = Point::new(control1.x, control1.y);
    let c2_2d = Point::new(control2.x, control2.y);

    let z0 = p0.z;
    let z1 = p1.z;
    // Linear interpolation of Z coordinate for control points
    let c1: Point3D =
        Point3D::new(c1_2d.x, c1_2d.y, z0 * (2.0 / 3.0) + z1 * (1.0 / 3.0));
    let c2: Point3D =
        Point3D::new(c2_2d.x, c2_2d.y, z0 * (1.0 / 3.0) + z1 * (2.0 / 3.0));

    // Estimate curve length using polygon approximation
    let l01 = p0.distance(c1);
    let l12 = c1.distance(c2);
    let l23 = c2.distance(p1);
    let estimated_len = l01 + l12 + l23;
    let num_steps = (estimated_len / resolution).ceil().max(2.0) as usize;

    linearize_bezier(p0, c1, c2, p1, num_steps)
}

/// Converts a Bezier curve into line segments using uniform parameter steps.
pub fn linearize_bezier(
    p0: Point3D,
    c1: Point3D,
    c2: Point3D,
    p1: Point3D,
    num_steps: usize,
) -> Vec<(Point3D, Point3D)> {
    if num_steps < 1 {
        return vec![];
    }

    let mut result = Vec::with_capacity(num_steps);
    let step_f = num_steps as f64;

    for i in 0..num_steps {
        let t = i as f64 / step_f;
        let t_next = (i as f64 + 1.0) / step_f;

        let p_start = Point3D::new(
            (1.0 - t).powi(3) * p0.x
                + 3.0 * (1.0 - t).powi(2) * t * c1.x
                + 3.0 * (1.0 - t) * t.powi(2) * c2.x
                + t.powi(3) * p1.x,
            (1.0 - t).powi(3) * p0.y
                + 3.0 * (1.0 - t).powi(2) * t * c1.y
                + 3.0 * (1.0 - t) * t.powi(2) * c2.y
                + t.powi(3) * p1.y,
            (1.0 - t).powi(3) * p0.z
                + 3.0 * (1.0 - t).powi(2) * t * c1.z
                + 3.0 * (1.0 - t) * t.powi(2) * c2.z
                + t.powi(3) * p1.z,
        );

        let p_end = Point3D::new(
            (1.0 - t_next).powi(3) * p0.x
                + 3.0 * (1.0 - t_next).powi(2) * t_next * c1.x
                + 3.0 * (1.0 - t_next) * t_next.powi(2) * c2.x
                + t_next.powi(3) * p1.x,
            (1.0 - t_next).powi(3) * p0.y
                + 3.0 * (1.0 - t_next).powi(2) * t_next * c1.y
                + 3.0 * (1.0 - t_next) * t_next.powi(2) * c2.y
                + t_next.powi(3) * p1.y,
            (1.0 - t_next).powi(3) * p0.z
                + 3.0 * (1.0 - t_next).powi(2) * t_next * c1.z
                + 3.0 * (1.0 - t_next) * t_next.powi(2) * c2.z
                + t_next.powi(3) * p1.z,
        );

        result.push((p_start, p_end));
    }

    result
}

/// Tests whether a cubic Bezier curve is flat enough to approximate with a
/// line segment, using a chord-distance flatness test.
///
/// For non-degenerate curves (where start ≠ end), it checks whether both
/// control points lie within `tolerance_sq` of the chord line using the
/// squared cross-product distance. For degenerate curves (start ≈ end), it
/// checks whether both control points are within `tolerance_sq` of the start
/// point.
pub fn is_bezier_flat(
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    tolerance_sq: f64,
) -> bool {
    let vx = p1.x - p0.x;
    let vy = p1.y - p0.y;
    let norm_sq = DVec2::new(vx, vy).length_squared();

    if norm_sq < 1e-9 {
        let d1_sq = c1.distance_squared(p0);
        let d2_sq = c2.distance_squared(p0);
        d1_sq <= tolerance_sq && d2_sq <= tolerance_sq
    } else {
        let cross1 = DVec2::new(vx, vy).perp_dot(c1 - p0);
        let cross2 = DVec2::new(vx, vy).perp_dot(c2 - p0);
        let dist1_sq = (cross1 * cross1) / norm_sq;
        let dist2_sq = (cross2 * cross2) / norm_sq;
        dist1_sq <= tolerance_sq && dist2_sq <= tolerance_sq
    }
}

pub fn linearize_bezier_adaptive(
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    tolerance_sq: f64,
    max_depth: usize,
) -> Polygon {
    let mut points: Polygon = vec![p0];

    let mut stack: Vec<(CubicBezier, usize)> =
        vec![(CubicBezier(p0, c1, c2, p1), 0)];

    while let Some((curve, depth)) = stack.pop() {
        let CubicBezier(sp0, sc1, sc2, sp1) = curve;
        if depth >= max_depth
            || is_bezier_flat(sp0, sc1, sc2, sp1, tolerance_sq)
        {
            points.push(sp1);
        } else {
            let (left, right) = split_bezier(sp0, sc1, sc2, sp1, 0.5);
            stack.push((right, depth + 1));
            stack.push((left, depth + 1));
        }
    }

    points
}

const BEZIER_SEG_MAX_DEPTH: usize = 10;

pub fn flatten_bezier(
    a: Point3D,
    b: Point3D,
    c: Point3D,
    d: Point3D,
    tolerance_sq: f64,
    depth: usize,
    points: &mut Polygon3D,
) {
    // Recursive subdivision with flatness test based on perpendicular distance
    if depth >= BEZIER_SEG_MAX_DEPTH
        || get_bezier_flatness_sq(a, b, c, d) <= tolerance_sq
    {
        points.push(d);
        return;
    }

    let m01 = midpoint(a, b);
    let m12 = midpoint(b, c);
    let m23 = midpoint(c, d);
    let q01 = midpoint(m01, m12);
    let q12 = midpoint(m12, m23);
    let r = midpoint(q01, q12);

    flatten_bezier(a, m01, q01, r, tolerance_sq, depth + 1, points);
    flatten_bezier(r, q12, m23, d, tolerance_sq, depth + 1, points);
}

/// Default tolerance for Bezier linearization (0.01 units).
const BEZIER_SEG_DEFAULT_TOLERANCE: f64 = 0.01;

/// Converts a 3D Bezier curve to a polygon using adaptive subdivision.
/// Uses perpendicular distance for flatness testing.
pub fn linearize_bezier_segment(
    p0: Point3D,
    c1: Point3D,
    c2: Point3D,
    p1: Point3D,
    tolerance: Option<f64>,
) -> Polygon3D {
    let tolerance = tolerance.unwrap_or(BEZIER_SEG_DEFAULT_TOLERANCE);
    let tolerance_sq = tolerance * tolerance;

    let mut points: Polygon3D = vec![p0];
    flatten_bezier(p0, c1, c2, p1, tolerance_sq, 0, &mut points);
    points
}

fn _lerp2(a: Point, b: Point, t: f64) -> Point {
    a.lerp(b, t)
}

fn _add_axis_extrema(
    candidates: &mut Vec<f64>,
    p0: f64,
    c1: f64,
    c2: f64,
    p1: f64,
) {
    let coeff_a = -p0 + 3.0 * c1 - 3.0 * c2 + p1;
    let coeff_b = 2.0 * (p0 - 2.0 * c1 + c2);
    let coeff_c = -p0 + c1;

    if coeff_a.abs() < 1e-12 {
        if coeff_b.abs() < 1e-12 {
            return;
        }
        let t = -coeff_c / coeff_b;
        if 0.0 < t && t < 1.0 {
            candidates.push(evaluate_cubic(p0, c1, c2, p1, t));
        }
        return;
    }

    let discriminant = coeff_b * coeff_b - 4.0 * coeff_a * coeff_c;
    if discriminant < 0.0 {
        return;
    }

    let sqrt_disc = discriminant.sqrt();
    for sign in [-1.0, 1.0] {
        let t = (-coeff_b + sign * sqrt_disc) / (2.0 * coeff_a);
        if 0.0 < t && t < 1.0 {
            candidates.push(evaluate_cubic(p0, c1, c2, p1, t));
        }
    }
}

pub fn evaluate_cubic(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let mt = 1.0 - t;
    mt.powi(3) * p0
        + 3.0 * mt.powi(2) * t * p1
        + 3.0 * mt * t.powi(2) * p2
        + t.powi(3) * p3
}

pub fn compute_cubic_bezier_bounds_1d(
    p0: &[f64],
    p1: &[f64],
    p2: &[f64],
    p3: &[f64],
) -> (Vec<f64>, Vec<f64>) {
    let n = p0.len();
    let mut local_min: Vec<f64> =
        p0.iter().zip(p3.iter()).map(|(a, b)| a.min(*b)).collect();
    let mut local_max: Vec<f64> =
        p0.iter().zip(p3.iter()).map(|(a, b)| a.max(*b)).collect();

    for i in 0..n {
        let p0i = p0[i];
        let p1i = p1[i];
        let p2i = p2[i];
        let p3i = p3[i];

        let a_coeff = 3.0 * (-p0i + 3.0 * p1i - 3.0 * p2i + p3i);
        let b_coeff = 6.0 * (p0i - 2.0 * p1i + p2i);
        let c_coeff = 3.0 * (p1i - p0i);

        let (t1, t2) = solve_quadratic(a_coeff, b_coeff, c_coeff);

        if let Some(t) = t1 {
            if t > 0.0 && t < 1.0 {
                let val = evaluate_cubic(p0i, p1i, p2i, p3i, t);
                local_min[i] = local_min[i].min(val);
                local_max[i] = local_max[i].max(val);
            }
        }

        if let Some(t) = t2 {
            if t > 0.0 && t < 1.0 {
                let val = evaluate_cubic(p0i, p1i, p2i, p3i, t);
                local_min[i] = local_min[i].min(val);
                local_max[i] = local_max[i].max(val);
            }
        }
    }

    (local_min, local_max)
}

fn _extract_subsegment(
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    t_start: f64,
    t_end: f64,
) -> CubicBezier {
    let starts_at_zero = t_start < 1e-12;
    let ends_at_one = (t_end - 1.0).abs() < 1e-12;

    if starts_at_zero && ends_at_one {
        return CubicBezier(p0, c1, c2, p1);
    }
    if starts_at_zero {
        let (left, _) = split_bezier(p0, c1, c2, p1, t_end);
        return left;
    }
    if ends_at_one {
        let (_, right) = split_bezier(p0, c1, c2, p1, t_start);
        return right;
    }

    let (_, right_after_start) = split_bezier(p0, c1, c2, p1, t_start);
    let reparam_end = (t_end - t_start) / (1.0 - t_start);
    let (left_of_end, _) = split_bezier(
        right_after_start.0,
        right_after_start.1,
        right_after_start.2,
        right_after_start.3,
        reparam_end,
    );
    left_of_end
}

fn _solve_cubic(a: f64, b: f64, c: f64, d: f64) -> Vec<f64> {
    if a.abs() < EPSILON_INTERSECT {
        if b.abs() < EPSILON_INTERSECT {
            if c.abs() < EPSILON_INTERSECT {
                return vec![];
            }
            return vec![-d / c];
        }
        let discriminant = c * c - 4.0 * b * d;
        if discriminant < 0.0 {
            return vec![];
        }
        let sqrt_disc = discriminant.sqrt();
        return vec![
            (-c + sqrt_disc) / (2.0 * b),
            (-c - sqrt_disc) / (2.0 * b),
        ];
    }

    let b = b / a;
    let c = c / a;
    let d = d / a;
    let _a = 1.0;

    let depressed_q = (3.0 * c - b * b) / 9.0;
    let depressed_r = (9.0 * b * c - 27.0 * d - 2.0 * b * b * b) / 54.0;
    let discriminant = depressed_q.powi(3) + depressed_r.powi(2);

    if discriminant > -EPSILON_INTERSECT {
        let sqrt_disc = discriminant.sqrt();
        let cube_root_sum = _cbrt(depressed_r + sqrt_disc);
        let cube_root_diff = _cbrt(depressed_r - sqrt_disc);
        let real_root = cube_root_sum + cube_root_diff - b / 3.0;
        return vec![real_root];
    }

    let neg_q_cubed = -depressed_q.powi(3);
    let cos_arg = (depressed_r / neg_q_cubed.sqrt()).clamp(-1.0, 1.0);
    let theta = cos_arg.acos();
    let amplitude = 2.0 * (-depressed_q).sqrt();
    let offset = b / 3.0;

    vec![
        amplitude * (theta / 3.0).cos() - offset,
        amplitude * ((theta + 2.0 * PI) / 3.0).cos() - offset,
        amplitude * ((theta + 4.0 * PI) / 3.0).cos() - offset,
    ]
}

fn _cbrt(x: f64) -> f64 {
    if x >= 0.0 {
        x.powf(1.0 / 3.0)
    } else {
        -((-x).powf(1.0 / 3.0))
    }
}

pub fn get_perpendicular_dist_sq(
    pt: Point3D,
    origin: Point3D,
    vx: f64,
    vy: f64,
    vz: f64,
    norm_sq: f64,
) -> f64 {
    let p = pt - origin;
    let cross = p.cross(DVec3::new(vx, vy, vz));
    cross.length_squared() / norm_sq
}

pub fn get_bezier_flatness_sq(
    a: Point3D,
    b: Point3D,
    c: Point3D,
    d: Point3D,
) -> f64 {
    let v = d - a;
    let norm_sq = v.length_squared();

    if norm_sq < 1e-9 {
        return (b - a).length_squared().max((c - a).length_squared());
    }

    let dist_b = get_perpendicular_dist_sq(b, a, v.x, v.y, v.z, norm_sq);
    let dist_c = get_perpendicular_dist_sq(c, a, v.x, v.y, v.z, norm_sq);
    dist_b.max(dist_c)
}

/// Computes the arc length of a cubic Bezier curve using adaptive
/// step sizing based on the control polygon length.
pub fn get_bezier_length(p0: Point, c1: Point, c2: Point, p1: Point) -> f64 {
    let l01 = p0.distance(c1);
    let l12 = c1.distance(c2);
    let l23 = c2.distance(p1);
    let estimated_len = l01 + l12 + l23;
    let num_steps = (estimated_len / 0.1).ceil().max(2.0) as usize;
    let step_f = num_steps as f64;
    let mut total = 0.0;
    let mut prev = p0;
    for i in 1..=num_steps {
        let t = i as f64 / step_f;
        let pt = get_bezier_point_at(p0, c1, c2, p1, t);
        total += pt.distance(prev);
        prev = pt;
    }
    total
}

/// Fit a cubic Bezier curve to a sequence of 2D points using least-squares.
///
/// The endpoints are fixed to the first and last point.  Returns `None`
/// when fewer than 2 points are given.
pub fn fit_cubic_bezier(points: &[Point]) -> Option<CubicBezier> {
    let n = points.len();
    if n < 2 {
        return None;
    }
    let p0 = points[0];
    let p3 = points[n - 1];

    if n == 2 {
        return Some(CubicBezier(p0, p0, p3, p3));
    }

    // Parameterise by cumulative chord length, normalised to [0, 1].
    let mut chords = vec![0.0f64; n];
    for i in 1..n {
        chords[i] = chords[i - 1] + points[i].distance(points[i - 1]);
    }
    let total = chords[n - 1];
    if total < 1e-12 {
        return Some(CubicBezier(p0, p0, p3, p3));
    }
    for c in &mut chords {
        *c /= total;
    }

    // Set up the least-squares system for the two interior control points.
    // For each data point Q_j at parameter t_j:
    //   w1_j * C1 + w2_j * C2 = R_j
    // where w1_j = 3*(1-t_j)^2*t_j, w2_j = 3*(1-t_j)*t_j^2
    // and   R_j = Q_j - (1-t_j)^3*P0 - t_j^3*P3
    let mut a11 = 0.0f64;
    let mut a12 = 0.0f64;
    let mut a22 = 0.0f64;
    let mut b1x = 0.0f64;
    let mut b1y = 0.0f64;
    let mut b2x = 0.0f64;
    let mut b2y = 0.0f64;

    for j in 0..n {
        let t = chords[j];
        let mt = 1.0 - t;
        let w1 = 3.0 * mt * mt * t;
        let w2 = 3.0 * mt * t * t;
        let rx = points[j].x - mt.powi(3) * p0.x - t.powi(3) * p3.x;
        let ry = points[j].y - mt.powi(3) * p0.y - t.powi(3) * p3.y;

        a11 += w1 * w1;
        a12 += w1 * w2;
        a22 += w2 * w2;
        b1x += w1 * rx;
        b1y += w1 * ry;
        b2x += w2 * rx;
        b2y += w2 * ry;
    }

    // Solve the 2×2 system [a11 a12; a12 a22] * [c1; c2] = [b1; b2].
    let det = a11 * a22 - a12 * a12;
    if det.abs() < 1e-18 {
        // Degenerate — place control points evenly along the chord.
        return Some(CubicBezier(
            p0,
            p0.lerp(p3, 1.0 / 3.0),
            p0.lerp(p3, 2.0 / 3.0),
            p3,
        ));
    }
    let inv = 1.0 / det;
    let c1x = (a22 * b1x - a12 * b2x) * inv;
    let c1y = (a22 * b1y - a12 * b2y) * inv;
    let c2x = (a11 * b2x - a12 * b1x) * inv;
    let c2y = (a11 * b2y - a12 * b1y) * inv;

    Some(CubicBezier(
        p0,
        Point::new(c1x, c1y),
        Point::new(c2x, c2y),
        p3,
    ))
}

/// Find the circle of `radius` that passes through `point` and is tangent
/// to a cubic Bezier curve.
///
/// Samples the Bezier and its analytic derivative at `N_SAMPLES` points,
/// computes the two candidate centres (one per normal direction) at each
/// sample, and returns the centre whose distance to `point` is closest
/// to `radius`.
///
/// Returns `(centre, tangent_point, t_parameter)` or `None` when no valid
/// circle is found within tolerance.
pub fn nearest_tangent_circle_on_bezier(
    point: Point,
    bezier: &CubicBezier,
    radius: f64,
) -> Option<(Point, Point, f64)> {
    let CubicBezier(p0, c1, c2, p3) = *bezier;

    const N_SAMPLES: usize = 200;
    let mut best_t = -1.0;
    let mut best_center = Point::ZERO;
    let mut best_tangent = Point::ZERO;
    let mut best_err = f64::MAX;

    for i in 0..=N_SAMPLES {
        let t = i as f64 / N_SAMPLES as f64;
        let mt = 1.0 - t;

        // Bezier point B(t).
        let bt = get_bezier_point_at(p0, c1, c2, p3, t);

        // Derivative B'(t) = 3*(1-t)^2*(C1-P0) + 6*(1-t)*t*(C2-C1) + 3*t^2*(P3-C2)
        let deriv = 3.0 * mt * mt * (c1 - p0)
            + 6.0 * mt * t * (c2 - c1)
            + 3.0 * t * t * (p3 - c2);
        let dl = deriv.length();
        if dl < 1e-12 {
            continue;
        }

        // Unit normal (perp of tangent).
        let n = Point::new(-deriv.y / dl, deriv.x / dl);

        for &side in &[1.0, -1.0] {
            let center = bt + side * radius * n;
            let err = (center.distance(point) - radius).abs();
            if err < best_err {
                best_err = err;
                best_t = t;
                best_center = center;
                best_tangent = bt;
            }
        }
    }

    if best_err < radius * 0.3 {
        Some((best_center, best_tangent, best_t))
    } else {
        None
    }
}
