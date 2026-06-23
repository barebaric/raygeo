//! Arc: Circular arc geometry operations.
//!
//! This module provides functions for working with circular arcs including:
//! - Angle normalization and computation
//! - Bounding box calculation
//! - Direction determination
//! - Linearization into line segments
//! - Intersection tests with rectangles, circles, and regions

use std::f64::consts::PI;

use glam::{DVec2, DVec3};

use crate::constants::EPSILON_COLLINEAR;
use crate::geo::shape::circle::get_circle_circle_intersections;
use crate::geo::shape::line::{
    does_line_segment_intersect_rect, get_line_segment_closest_point,
};
use crate::geo::shape::polygon::is_point_inside_polygon;
use crate::types::{Point, Point3D, Polygon, Rect};

/// Normal for a CCW arc in the XY plane (G17 G03).
pub const XY_NORMAL_CCW: Point3D = Point3D::new(0.0, 0.0, 1.0);

/// Convert the legacy `clockwise: bool` to a 3D plane normal.
///
/// `clockwise = false` (CCW in XY) → `(0, 0, +1)`.
/// `clockwise = true`  (CW  in XY) → `(0, 0, -1)`.
pub fn normal_from_clockwise_3d(clockwise: bool) -> Point3D {
    if clockwise {
        Point3D::new(0.0, 0.0, -1.0)
    } else {
        XY_NORMAL_CCW
    }
}

/// Compute the signed sweep angle for an arc in 3D.
///
/// Returns the sweep from start to end going in the positive (right-hand) direction
/// around `normal`.  The sweep is always in `(0, 2π]` so that the arc is always
/// traversed counter-clockwise around the normal.  A full circle is `2π`.
pub fn get_arc_sweep_3d(start_angle: f64, end_angle: f64) -> f64 {
    let mut sweep = end_angle - start_angle;
    if sweep.abs() < EPSILON_COLLINEAR {
        sweep = 2.0 * PI;
    } else if sweep < EPSILON_COLLINEAR {
        sweep += 2.0 * PI;
    }
    sweep
}

/// Computes the arc length given start position, end position, center
/// offset, and direction. Returns 0.0 if the radius is degenerate.
pub fn get_arc_length(
    start_pos: Point,
    end_pos: Point,
    center_offset: Point,
    clockwise: bool,
) -> f64 {
    let center = Point::new(
        start_pos.x + center_offset.x,
        start_pos.y + center_offset.y,
    );
    let radius = start_pos.distance(center);
    if radius < 1e-9 {
        return 0.0;
    }
    let (_, _, sweep) = get_arc_angles(start_pos, end_pos, center, clockwise);
    sweep.abs() * radius
}

/// Normalizes an angle to the range [0, 2*PI).
pub fn normalize_angle(angle: f64) -> f64 {
    ((angle % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI)
}

/// Compute the signed sweep angle for an arc, handling direction and
/// full-circle detection when the start and end angles are nearly equal.
pub fn get_arc_sweep(start_angle: f64, end_angle: f64, clockwise: bool) -> f64 {
    let mut sweep = end_angle - start_angle;
    if sweep.abs() < EPSILON_COLLINEAR {
        sweep = if clockwise { -2.0 * PI } else { 2.0 * PI };
    } else if clockwise {
        if sweep > EPSILON_COLLINEAR {
            sweep -= 2.0 * PI;
        }
    } else if sweep < -EPSILON_COLLINEAR {
        sweep += 2.0 * PI;
    }
    sweep
}

/// Computes the start angle, end angle, and sweep angle for an arc.
/// Handles the direction (CW/CCW) to compute the correct sweep,
/// including full-circle detection when start ≈ end.
pub fn get_arc_angles(
    start_pos: Point,
    end_pos: Point,
    center: Point,
    clockwise: bool,
) -> (f64, f64, f64) {
    let start_angle = (start_pos.y - center.y).atan2(start_pos.x - center.x);
    let end_angle = (end_pos.y - center.y).atan2(end_pos.x - center.x);
    let sweep = get_arc_sweep(start_angle, end_angle, clockwise);
    (start_angle, end_angle, sweep)
}

/// Computes the midpoint of an arc (at t=0.5 along the arc).
pub fn get_arc_midpoint(
    start_pos: Point,
    end_pos: Point,
    center: Point,
    clockwise: bool,
) -> Point {
    let (start_a, _, sweep) =
        get_arc_angles(start_pos, end_pos, center, clockwise);
    let mid_angle = start_a + sweep / 2.0;
    let radius = start_pos.distance(center);
    Point::new(
        center.x + radius * mid_angle.cos(),
        center.y + radius * mid_angle.sin(),
    )
}

/// Tests if an angle falls between start and end angles following the arc direction.
pub fn is_angle_between(
    target: f64,
    start: f64,
    end: f64,
    clockwise: bool,
) -> bool {
    let target = normalize_angle(target);
    let start = normalize_angle(start);
    let end = normalize_angle(end);

    if clockwise {
        // For clockwise: sweep goes from higher to lower (wrapping around 0)
        if start < end {
            target <= start || target >= end
        } else {
            end <= target && target <= start
        }
    } else {
        // For counter-clockwise: sweep goes from lower to higher
        if start > end {
            target >= start || target <= end
        } else {
            start <= target && target <= end
        }
    }
}

/// Computes the axis-aligned bounding box that contains the entire arc.
/// Checks cardinal directions (0, 90, 180, 270 degrees) to find extrema.
pub fn get_arc_bounds(
    start_pos: Point,
    end_pos: Point,
    center_offset: Point,
    clockwise: bool,
) -> Rect {
    let center_x = start_pos.x + center_offset.x;
    let center_y = start_pos.y + center_offset.y;
    let radius = center_offset.length();

    let mut min_x = start_pos.x.min(end_pos.x);
    let mut min_y = start_pos.y.min(end_pos.y);
    let mut max_x = start_pos.x.max(end_pos.x);
    let mut max_y = start_pos.y.max(end_pos.y);

    let start_angle = (start_pos.y - center_y).atan2(start_pos.x - center_x);
    let end_angle = (end_pos.y - center_y).atan2(end_pos.x - center_x);

    if is_angle_between(0.0, start_angle, end_angle, clockwise) {
        max_x = max_x.max(center_x + radius);
    }
    if is_angle_between(PI / 2.0, start_angle, end_angle, clockwise) {
        max_y = max_y.max(center_y + radius);
    }
    if is_angle_between(PI, start_angle, end_angle, clockwise) {
        min_x = min_x.min(center_x - radius);
    }
    if is_angle_between(3.0 * PI / 2.0, start_angle, end_angle, clockwise) {
        min_y = min_y.min(center_y - radius);
    }

    Rect::new(min_x, min_y, max_x, max_y)
}

/// Determines arc direction based on center, start point, and a third reference point.
/// Uses cross product of vectors from center to determine if the reference is on
/// the clockwise or counter-clockwise side. Used for interactive arc drawing.
pub fn get_arc_direction(center: Point, start: Point, mouse: Point) -> bool {
    let vec_s_x = start.x - center.x;
    let vec_s_y = start.y - center.y;
    let vec_m_x = mouse.x - center.x;
    let vec_m_y = mouse.y - center.y;

    // Negative cross product indicates clockwise
    let det =
        DVec2::new(vec_s_x, vec_s_y).perp_dot(DVec2::new(vec_m_x, vec_m_y));
    det < 0.0
}

/// Determines if points along an arc traverse in clockwise direction relative to center.
/// Uses cumulative cross product of successive radius vectors.
pub fn is_arc_clockwise(points: &[Point], center: Point) -> bool {
    let xc = center.x;
    let yc = center.y;
    let mut cross_product_sum = 0.0;

    for i in 0..points.len() - 1 {
        let x0 = points[i].x;
        let y0 = points[i].y;
        let x1 = points[i + 1].x;
        let y1 = points[i + 1].y;
        let v0x = x0 - xc;
        let v0y = y0 - yc;
        let v1x = x1 - xc;
        let v1y = y1 - yc;
        cross_product_sum +=
            DVec2::new(v0x, v0y).perp_dot(DVec2::new(v1x, v1y));
    }

    cross_product_sum < 0.0
}

/// Converts an arc into a series of line segments for approximation.
///
/// The arc is defined by its endpoint, center offset, plane normal, and start
/// point.  The sweep always goes counter-clockwise around the normal (right-hand
/// rule).  For XY-plane arcs, pass `normal = (0,0,+1)` for CCW or `(0,0,-1)` for CW.
///
/// `resolution` controls the maximum length of each chord segment.
/// Writes into a caller-provided buffer so allocations can be reused across calls.
pub fn linearize_arc(
    end: Point3D,
    center_offset: Point3D,
    normal: Point3D,
    start_point: Point3D,
    resolution: f64,
    out: &mut Vec<(Point3D, Point3D)>,
) {
    out.clear();
    let p0 = DVec3::new(start_point.x, start_point.y, start_point.z);
    let p1 = DVec3::new(end.x, end.y, end.z);
    let n = DVec3::new(normal.x, normal.y, normal.z).normalize();

    // Reject zero-length normals — treat as degenerate (straight line)
    if n.length() < 1e-30 {
        out.push((start_point, end));
        return;
    }

    let center =
        p0 + DVec3::new(center_offset.x, center_offset.y, center_offset.z);

    let r0 = p0 - center;
    let r1 = p1 - center;

    // Project radius vectors into the arc plane
    let r0_proj = r0 - n * r0.dot(n);
    let r1_proj = r1 - n * r1.dot(n);

    let radius_start = r0_proj.length();
    if radius_start < 1e-9 {
        out.push((start_point, end));
        return;
    }

    let radius_end = r1_proj.length();

    // Build orthonormal basis in the arc plane
    let u = r0_proj / radius_start;
    let v = n.cross(u).normalize();

    // Compute the sweep angle (always positive around the normal)
    let theta_end = f64::atan2(r1_proj.dot(v), r1_proj.dot(u));
    let sweep = get_arc_sweep_3d(0.0, theta_end);

    let avg_radius = (radius_start + radius_end) / 2.0;
    let arc_len = sweep * avg_radius;
    let num_segments = (arc_len / resolution).ceil().max(2.0) as usize;

    // Helical Z interpolation + linear radius interpolation (for spiral arcs)
    let z0 = p0.z;
    let z1 = p1.z;

    let mut prev_pt = start_point;
    for i in 1..=num_segments {
        let t = i as f64 / num_segments as f64;
        let angle = sweep * t;
        let radius = radius_start + (radius_end - radius_start) * t;
        let (s, c) = angle.sin_cos();
        let pt = center + u * (radius * c) + v * (radius * s);
        let z = z0 + (z1 - z0) * t;
        let next_pt = Point3D::new(pt.x, pt.y, z);
        out.push((prev_pt, next_pt));
        prev_pt = next_pt;
    }
}

/// Internal: Finds closest point on arc using linearized approximation.
fn find_closest_on_linearized_arc(
    end: Point3D,
    center_offset: Point3D,
    normal: Point3D,
    start_pos: Point3D,
    x: f64,
    y: f64,
) -> Option<(f64, Point, f64)> {
    let mut arc_segments = Vec::new();
    linearize_arc(
        end,
        center_offset,
        normal,
        start_pos,
        0.1,
        &mut arc_segments,
    );
    if arc_segments.is_empty() {
        return None;
    }

    let mut min_dist_sq = f64::INFINITY;
    let mut best_result: Option<(usize, f64, Point, f64)> = None;

    for (j, (p1_3d, p2_3d)) in arc_segments.iter().enumerate() {
        let t_sub = get_line_segment_closest_point(
            Point::new(p1_3d.x, p1_3d.y),
            Point::new(p2_3d.x, p2_3d.y),
            x,
            y,
        );
        if t_sub.2 < min_dist_sq {
            min_dist_sq = t_sub.2;
            best_result = Some((j, t_sub.0, t_sub.1, t_sub.2));
        }
    }

    best_result.map(|(j_best, t_sub_best, pt_best, dist_sq_best)| {
        let t_arc = (j_best as f64 + t_sub_best) / arc_segments.len() as f64;
        (t_arc, pt_best, dist_sq_best)
    })
}

fn find_closest_point_on_arc_impl(
    end: Point3D,
    center_offset: Point3D,
    normal: Point3D,
    start_pos: Point3D,
    x: f64,
    y: f64,
) -> Option<(f64, Point, f64)> {
    // Project to XY plane for the 2D closest-point computation.
    // The `normal` is used to determine the effective direction:
    //   normal.(0,0,+1) → CCW (clockwise=false)
    //   normal.(0,0,-1) → CW  (clockwise=true)
    // For non-Z normals the 2D projection is approximate.
    let clockwise = normal.z < 0.0;

    let p0 = Point::new(start_pos.x, start_pos.y);
    let p1 = Point::new(end.x, end.y);
    let center = Point::new(p0.x + center_offset.x, p0.y + center_offset.y);

    let radius_start = p0.distance(center);
    let radius_end = p1.distance(center);

    if (radius_start - radius_end).abs() > 1e-9 {
        return find_closest_on_linearized_arc(
            end,
            center_offset,
            normal,
            start_pos,
            x,
            y,
        );
    }

    let radius = radius_start;
    if radius < 1e-9 {
        let dist_sq = Point::new(x, y).distance_squared(p0);
        return Some((0.0, p0, dist_sq));
    }

    let vec_to_point = (x - center.x, y - center.y);
    let dist_to_center = Point::new(vec_to_point.0, vec_to_point.1).length();
    let closest_on_circle = if dist_to_center < 1e-9 {
        p0
    } else {
        center + Point::new(vec_to_point.0, vec_to_point.1).normalize() * radius
    };

    let start_angle = (p0.y - center.y).atan2(p0.x - center.x);
    let end_angle = (p1.y - center.y).atan2(p1.x - center.x);
    let point_angle =
        (closest_on_circle.y - center.y).atan2(closest_on_circle.x - center.x);

    let mut angle_range = end_angle - start_angle;
    let mut angle_to_check = point_angle - start_angle;

    if clockwise {
        if angle_range > 1e-9 {
            angle_range -= 2.0 * PI;
        }
        if angle_to_check > 1e-9 {
            angle_to_check -= 2.0 * PI;
        }
    } else {
        if angle_range < -1e-9 {
            angle_range += 2.0 * PI;
        }
        if angle_to_check < -1e-9 {
            angle_to_check += 2.0 * PI;
        }
    }

    let is_on_arc = if clockwise {
        angle_to_check >= angle_range - 1e-9 && angle_to_check <= 1e-9
    } else {
        angle_to_check <= angle_range + 1e-9 && angle_to_check >= -1e-9
    };

    let (closest_point, t) = if is_on_arc {
        (
            closest_on_circle,
            if angle_range.abs() > 1e-9 {
                angle_to_check / angle_range
            } else {
                0.0
            },
        )
    } else {
        let dist_sq_p0 = Point::new(x, y).distance_squared(p0);
        let dist_sq_p1 = Point::new(x, y).distance_squared(p1);
        if dist_sq_p0 <= dist_sq_p1 {
            (p0, 0.0)
        } else {
            (p1, 1.0)
        }
    };

    let dist_sq = Point::new(x, y).distance_squared(closest_point);
    let t = t.clamp(0.0, 1.0);
    Some((t, closest_point, dist_sq))
}

/// Finds the closest point on an arc to a given (x, y) coordinate.
/// Returns (t_parameter, closest_point, distance_squared).
///
/// For non-XY-plane arcs the projection is approximate.
pub fn get_arc_closest_point(
    end: Point3D,
    center_offset: Point3D,
    normal: Point3D,
    start_pos: Point3D,
    x: f64,
    y: f64,
) -> Option<(f64, Point, f64)> {
    find_closest_point_on_arc_impl(end, center_offset, normal, start_pos, x, y)
}

/// Tests if an arc intersects an axis-aligned rectangle.
/// Uses bounding box check followed by linearized segment testing.
pub fn does_arc_intersect_rect(
    start_pos: Point,
    end_pos: Point,
    center: Point,
    clockwise: bool,
    rect: Rect,
) -> bool {
    // Quick bounding box rejection test
    let arc_box = get_arc_bounds(
        start_pos,
        end_pos,
        Point::new(center.x - start_pos.x, center.y - start_pos.y),
        clockwise,
    );
    if arc_box.max.x < rect.min.x
        || arc_box.min.x > rect.max.x
        || arc_box.max.y < rect.min.y
        || arc_box.min.y > rect.max.y
    {
        return false;
    }

    // Linearize and test each segment
    let offset_3d =
        Point3D::new(center.x - start_pos.x, center.y - start_pos.y, 0.0);
    let normal = normal_from_clockwise_3d(clockwise);
    let radius = start_pos.distance(center);
    let start_3d: Point3D = Point3D::new(start_pos.x, start_pos.y, 0.0);
    let end_3d: Point3D = Point3D::new(end_pos.x, end_pos.y, 0.0);

    let mut segments = Vec::new();
    linearize_arc(
        end_3d,
        offset_3d,
        normal,
        start_3d,
        radius * 0.1,
        &mut segments,
    );
    for (p1_3d, p2_3d) in segments {
        if does_line_segment_intersect_rect(
            Point::new(p1_3d.x, p1_3d.y),
            Point::new(p2_3d.x, p2_3d.y),
            rect,
        ) {
            return true;
        }
    }

    false
}

/// Tests if an arc intersects a circle. Checks:
/// 1. If either endpoint is inside the circle
/// 2. Circle-arc intersection points fall on the arc
/// 3. Midpoint of arc is inside the circle
pub fn does_arc_intersect_circle(
    start_pos: Point,
    end_pos: Point,
    center: Point,
    clockwise: bool,
    circle_center: Point,
    circle_radius: f64,
) -> bool {
    let radius = start_pos.distance(center);
    if radius < 1e-9 {
        return start_pos.distance(circle_center) <= circle_radius;
    }

    // Check if either endpoint is inside the circle
    if start_pos.distance(circle_center) <= circle_radius {
        return true;
    }
    if end_pos.distance(circle_center) <= circle_radius {
        return true;
    }

    // Check if circle-arc intersection points fall on the arc
    let intersections = get_circle_circle_intersections(
        center,
        radius,
        circle_center,
        circle_radius,
    );
    if !intersections.is_empty() {
        let start_angle =
            (start_pos.y - center.y).atan2(start_pos.x - center.x);
        let end_angle = (end_pos.y - center.y).atan2(end_pos.x - center.x);
        for pt in intersections {
            let angle = (pt.y - center.y).atan2(pt.x - center.x);
            if is_angle_between(angle, start_angle, end_angle, clockwise) {
                return true;
            }
        }
    }

    // Check midpoint of arc as fallback
    let mid = get_arc_midpoint(start_pos, end_pos, center, clockwise);
    if mid.distance(circle_center) <= circle_radius {
        return true;
    }

    false
}

/// Tests if an arc is fully contained within all specified regions.
/// Samples key points (corners of bbox, endpoints, midpoint) for containment check.
pub fn is_arc_inside_polygons(
    start_pos: Point,
    end_pos: Point,
    center_offset: Point,
    clockwise: bool,
    regions: &[Polygon],
) -> bool {
    let center = Point::new(
        start_pos.x + center_offset.x,
        start_pos.y + center_offset.y,
    );
    let bbox = get_arc_bounds(start_pos, end_pos, center_offset, clockwise);
    let mid = get_arc_midpoint(start_pos, end_pos, center, clockwise);

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

/// Build a circular arc from `t_start` to `t_end` around `center` (radius
/// `r`), choosing the sweep direction so the arc passes through `t_mid`.
pub fn arc_through_point(
    t_start: Point,
    t_end: Point,
    t_mid: Point,
    center: Point,
    r: f64,
) -> Vec<Point> {
    let a1 = (t_start - center).y.atan2((t_start - center).x);
    let a2 = (t_end - center).y.atan2((t_end - center).x);
    let am = (t_mid - center).y.atan2((t_mid - center).x);

    let two_pi = 2.0 * PI;
    let sweep_ccw = (a2 - a1 + two_pi) % two_pi;
    let mid_ccw = (am - a1 + two_pi) % two_pi;

    let sweep = if mid_ccw <= sweep_ccw {
        sweep_ccw
    } else {
        -(two_pi - sweep_ccw)
    };

    let n_arc = (sweep.abs() * 4.0).ceil().clamp(4.0, 64.0) as usize;
    let mut arc = vec![t_start];
    for j in 1..n_arc {
        let t = j as f64 / n_arc as f64;
        let a = a1 + sweep * t;
        arc.push(center + Point::new(r * a.cos(), r * a.sin()));
    }
    arc.push(t_end);
    arc
}

/// Sign of the overall turning direction of a polyline, sampled at its
/// midpoint.
///
/// Computes the 2-D cross product of the edge vectors just before and
/// just after the midpoint vertex.  Returns `+1.0` when the polyline
/// turns counter-clockwise (left turn, centre of curvature on the left)
/// and `-1.0` when it turns clockwise (right turn).  Degenerate inputs
/// (fewer than 3 points, or a midpoint with no neighbours) default to
/// `+1.0`.
pub fn get_polyline_turn_sign(polyline: &[Point]) -> f64 {
    let n = polyline.len();
    if n < 3 {
        return 1.0;
    }
    let mid = n / 2;
    let prev = mid.saturating_sub(1);
    let next = (mid + 1).min(n - 1);
    if prev == next {
        return 1.0;
    }
    let d0 = polyline[mid] - polyline[prev];
    let d1 = polyline[next] - polyline[mid];
    if d0.x * d1.y - d0.y * d1.x >= 0.0 {
        1.0
    } else {
        -1.0
    }
}
