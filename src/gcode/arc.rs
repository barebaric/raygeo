//! Center resolution for radius-format (R word) G2/G3 arcs.

use std::f64::consts::{PI, TAU};

use crate::geo::types::Point;

/// Resolves the center of a radius-format G2/G3 arc.
///
/// RS274/NGC defines two possible centers for a given start, end and
/// radius. A positive radius selects the minor arc (sweep of at most 180
/// degrees), a negative radius the major arc. Returns `None` when the
/// chord is degenerate or longer than the diameter.
pub fn center_from_radius(
    start: Point,
    end: Point,
    radius: f64,
    clockwise: bool,
) -> Option<Point> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let chord = (dx * dx + dy * dy).sqrt();
    if chord <= 1e-12 || radius.abs() <= 1e-12 {
        return None;
    }
    let half = chord / 2.0;
    let minor = radius > 0.0;
    let radius = radius.abs();
    if half > radius + 1e-9 {
        return None;
    }
    let height = (radius * radius - half * half).max(0.0).sqrt();
    let mid_x = (start.x + end.x) / 2.0;
    let mid_y = (start.y + end.y) / 2.0;
    let perp_x = -dy / chord;
    let perp_y = dx / chord;
    let first = Point::new(mid_x + perp_x * height, mid_y + perp_y * height);
    let second = Point::new(mid_x - perp_x * height, mid_y - perp_y * height);

    let first_sweep = sweep(start, end, first, clockwise);
    let second_sweep = sweep(start, end, second, clockwise);
    if is_selected(minor, first_sweep) {
        Some(first)
    } else if is_selected(minor, second_sweep) {
        Some(second)
    } else {
        Some(first)
    }
}

fn is_selected(minor: bool, sweep: f64) -> bool {
    if minor {
        sweep <= PI + 1e-9
    } else {
        sweep > PI + 1e-9
    }
}

fn sweep(start: Point, end: Point, center: Point, clockwise: bool) -> f64 {
    let start_angle = (start.y - center.y).atan2(start.x - center.x);
    let end_angle = (end.y - center.y).atan2(end.x - center.x);
    let delta = if clockwise {
        start_angle - end_angle
    } else {
        end_angle - start_angle
    };
    delta.rem_euclid(TAU)
}
