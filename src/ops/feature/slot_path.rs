/// Find the 2D carrier path for a slot clearing operation.
///
/// A "slot carrier" is a polyline along the centre of a slot polygon
/// where the full tool disk (of `tool_radius`) fits within the slot at
/// every point. The carrier is oriented so the first point lies on the
/// entry side.
///
/// # Algorithm — disk-probe snake walk
///
/// Unlike the previous AABB-slice-centroid approach (which only works
/// for monotone rectangular slots), this algorithm follows the actual
/// corridor shape by probing a disk at each step and stepping to the
/// centroid of the intersection of that disk with the eroded region.
///
/// 1. Erode `slot_polygon` by `tool_radius` (negative offset, Miter
///    join). If the eroded region is empty → `None`.
/// 2. Determine the slot's long axis (longer AABB dimension) and which
///    end is the entry side (from `entry_edges[0]` midpoint or
///    `entry_point` relative to AABB centre).
/// 3. Place a probe disk at the near end of the eroded region (on the
///    entry side) and compute the centroid of its intersection with
///    the eroded region. That centroid is the start of the carrier.
/// 4. Initialise the heading along the long axis, pointing away from
///    the entry side.
/// 5. Walk: at each step, advance by `step = 0.4 × tool_radius` in the
///    current heading, place a probe disk (radius = `tool_radius`) at
///    the candidate, intersect with the eroded region, and step to the
///    centroid of the largest intersection polygon. Update heading with
///    exponential smoothing (60% new, 40% old).
/// 6. Dead-end recovery: if the forward probe is empty, try half and
///    quarter steps, then ±90° and 180° turns. If none succeed, stop.
/// 7. Stop on no-forward-progress, loop-back (quantized visited set),
///    or after `MAX_STEPS` iterations.
/// 8. If fewer than 2 carrier points survive → `None`.
use crate::geo::algo::clipping::clip_line_segment_with_polygons_2d;
use crate::geo::shape::polygon::{
    get_circle_polygon, get_polygon_area, get_polygon_centroid,
    get_polygon_group_bounds, get_polygons_group_intersection, offset_polygon,
    JoinStyle,
};
use crate::geo::types::{Point, Polygon};

const CIRCLE_N: usize = 24;
const MAX_STEPS: usize = 512;
const HEADING_NEW_WEIGHT: f64 = 0.6;
const MIN_PROGRESS_FRAC: f64 = 0.15;

fn probe_disk_centroid(
    center: Point,
    probe_radius: f64,
    eroded: &[Polygon],
) -> Option<Point> {
    let disk = get_circle_polygon(center, probe_radius, CIRCLE_N);
    let intersection = get_polygons_group_intersection(&[disk], eroded);
    if intersection.is_empty() {
        return None;
    }
    let best = intersection.iter().max_by(|a, b| {
        get_polygon_area(a)
            .partial_cmp(&get_polygon_area(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;
    if get_polygon_area(best) < 1e-12 {
        return None;
    }
    Some(get_polygon_centroid(best))
}

fn quantize(p: Point, q: f64) -> (i64, i64) {
    ((p.x / q).round() as i64, (p.y / q).round() as i64)
}

/// Find the 2D carrier path for a slot clearing operation.
///
/// Returns `Some(vec![...])` where the points are valid tool centres
/// within the eroded slot, or `None` if the slot is too narrow for the
/// tool or the carrier is shorter than 2 samples.
pub fn find_slot_path(
    slot_polygon: &Polygon,
    entry_edges: &[usize],
    entry_point: Point,
    tool_radius: f64,
) -> Option<Vec<Point>> {
    if slot_polygon.len() < 3 || entry_edges.is_empty() {
        return None;
    }

    let eroded = offset_polygon(slot_polygon, -tool_radius, JoinStyle::Miter);
    if eroded.is_empty() {
        return None;
    }

    let outer_bounds =
        get_polygon_group_bounds(std::slice::from_ref(slot_polygon));
    let eroded_bounds = get_polygon_group_bounds(&eroded);

    let w = outer_bounds.max.x - outer_bounds.min.x;
    let h = outer_bounds.max.y - outer_bounds.min.y;
    let long_is_x = w >= h;

    let ei = entry_edges[0];
    let n = slot_polygon.len();
    let edge_a = slot_polygon[ei];
    let edge_b = slot_polygon[(ei + 1) % n];
    let edge_mid = (edge_a + edge_b) * 0.5;

    let bounds_center = (outer_bounds.min + outer_bounds.max) * 0.5;

    let entry_coord = if long_is_x { edge_mid.x } else { edge_mid.y };
    let center_coord = if long_is_x {
        bounds_center.x
    } else {
        bounds_center.y
    };

    let decisive = if (entry_coord - center_coord).abs() < 1e-12 {
        if long_is_x {
            entry_point.x
        } else {
            entry_point.y
        }
    } else {
        entry_coord
    };

    let (near_long, far_long, heading_sign) = if long_is_x {
        if decisive < bounds_center.x {
            (eroded_bounds.min.x, eroded_bounds.max.x, 1.0)
        } else {
            (eroded_bounds.max.x, eroded_bounds.min.x, -1.0)
        }
    } else if decisive < bounds_center.y {
        (eroded_bounds.min.y, eroded_bounds.max.y, 1.0)
    } else {
        (eroded_bounds.max.y, eroded_bounds.min.y, -1.0)
    };

    let step_size = tool_radius * 0.4;
    let probe_radius = tool_radius;

    let transverse_mid = if long_is_x { edge_mid.y } else { edge_mid.x };
    let transverse_center = if long_is_x {
        (eroded_bounds.min.y + eroded_bounds.max.y) * 0.5
    } else {
        (eroded_bounds.min.x + eroded_bounds.max.x) * 0.5
    };
    let transverse_candidates = [transverse_mid, transverse_center];

    let mut start: Option<Point> = None;
    for &t_val in &transverse_candidates {
        for &inset_frac in &[0.1, 0.3, 0.5, 1.0] {
            let long_inset = tool_radius * inset_frac * heading_sign;
            let probe = if long_is_x {
                Point::new(near_long + long_inset, t_val)
            } else {
                Point::new(t_val, near_long + long_inset)
            };
            if let Some(pt) = probe_disk_centroid(probe, probe_radius, &eroded)
            {
                start = Some(pt);
                break;
            }
        }
        if start.is_some() {
            break;
        }
    }
    let start = start?;

    let mut heading = if long_is_x {
        Point::new(heading_sign, 0.0)
    } else {
        Point::new(0.0, heading_sign)
    };

    let mut carrier = vec![start];
    let mut current = start;

    let quant = step_size * 0.25;
    let mut visited = std::collections::HashSet::new();
    visited.insert(quantize(current, quant));

    for _ in 0..MAX_STEPS {
        let candidate = current + heading * step_size;

        let mut centroid =
            probe_disk_centroid(candidate, probe_radius, &eroded);

        if centroid.is_none() {
            for &f in &[0.5, 0.25] {
                let c = current + heading * (step_size * f);
                if let Some(pt) = probe_disk_centroid(c, probe_radius, &eroded)
                {
                    centroid = Some(pt);
                    break;
                }
            }
        }

        if centroid.is_none() {
            let perp = Point::new(-heading.y, heading.x);
            for &dir in &[perp, -perp] {
                for &mult in &[0.5, 1.0, 2.0, 3.0] {
                    let c = current + dir * (step_size * mult);
                    if let Some(pt) =
                        probe_disk_centroid(c, probe_radius, &eroded)
                    {
                        centroid = Some(pt);
                        break;
                    }
                }
                if centroid.is_some() {
                    break;
                }
            }
        }

        if centroid.is_none() {
            break;
        }

        let centroid = centroid.unwrap();

        let mut progress = centroid.distance(current);

        if progress < step_size * MIN_PROGRESS_FRAC {
            let perp = Point::new(-heading.y, heading.x);
            let mut best: Option<(Point, f64)> = None;
            for &dir in &[perp, -perp] {
                for &mult in &[0.5, 1.0, 2.0, 3.0] {
                    let c = current + dir * (step_size * mult);
                    if let Some(pt) =
                        probe_disk_centroid(c, probe_radius, &eroded)
                    {
                        let d = pt.distance(current);
                        if d > progress {
                            progress = d;
                            best = Some((pt, d));
                        }
                    }
                }
            }
            match best {
                Some((pt, _)) => {
                    let c = pt;
                    let new_dir = c - current;
                    let new_len = new_dir.length();
                    if new_len < 1e-12 {
                        break;
                    }
                    let new_heading = new_dir / new_len;
                    heading = new_heading * HEADING_NEW_WEIGHT
                        + heading * (1.0 - HEADING_NEW_WEIGHT);
                    let hlen = heading.length();
                    heading = if hlen > 1e-12 {
                        heading / hlen
                    } else {
                        new_heading
                    };
                    let key = quantize(c, quant);
                    if !visited.insert(key) {
                        break;
                    }
                    let cur_long = if long_is_x { c.x } else { c.y };
                    let reached_far = if heading_sign > 0.0 {
                        cur_long >= far_long
                    } else {
                        cur_long <= far_long
                    };
                    carrier.push(c);
                    current = c;
                    if reached_far {
                        break;
                    }
                    continue;
                }
                None => break,
            }
        }

        let new_dir = centroid - current;
        let new_len = new_dir.length();
        if new_len < 1e-12 {
            break;
        }
        let new_heading = new_dir / new_len;

        heading = new_heading * HEADING_NEW_WEIGHT
            + heading * (1.0 - HEADING_NEW_WEIGHT);
        let hlen = heading.length();
        heading = if hlen > 1e-12 {
            heading / hlen
        } else {
            new_heading
        };

        let key = quantize(centroid, quant);
        if !visited.insert(key) {
            break;
        }

        let cur_long = if long_is_x { centroid.x } else { centroid.y };
        let reached_far = if heading_sign > 0.0 {
            cur_long >= far_long
        } else {
            cur_long <= far_long
        };

        carrier.push(centroid);
        current = centroid;

        if reached_far {
            break;
        }
    }

    if carrier.len() < 2 {
        return None;
    }

    Some(carrier)
}

/// Measure the passage width perpendicular to `direction` at `point`.
///
/// Casts a line through `point` perpendicular to `direction`, clips it
/// to `passage`, and returns the longest resulting sub-segment length.
/// Works for arbitrary passage shapes (not just rectangles).
pub fn measure_passage_width_at(
    passage: &Polygon,
    point: Point,
    direction: Point,
) -> f64 {
    let len = (direction.x * direction.x + direction.y * direction.y).sqrt();
    if len < 1e-12 {
        return 0.0;
    }
    let nx = -direction.y / len;
    let ny = direction.x / len;
    let far = 10000.0;
    let p1 = Point::new(point.x + nx * far, point.y + ny * far);
    let p2 = Point::new(point.x - nx * far, point.y - ny * far);
    let clipped = clip_line_segment_with_polygons_2d(
        p1,
        p2,
        std::slice::from_ref(passage),
    );
    let mut best = 0.0;
    for (s, e) in &clipped {
        let dx = e.x - s.x;
        let dy = e.y - s.y;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len > best {
            best = seg_len;
        }
    }
    best
}

/// Measure the minimum passage width along a carrier polyline.
///
/// Samples [`measure_passage_width_at`] at regular intervals (every
/// ~2 mm) along the carrier and returns the minimum.  For a 2-point
/// carrier (typical of [`find_ramp_carrier`]) the samples lie on the
/// straight line between the endpoints.
pub fn measure_passage_min_width(passage: &Polygon, carrier: &[Point]) -> f64 {
    if carrier.len() < 2 {
        return 0.0;
    }
    let mut min_width = f64::MAX;
    for window in carrier.windows(2) {
        let dx = window[1].x - window[0].x;
        let dy = window[1].y - window[0].y;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len < 1e-12 {
            continue;
        }
        let dir = Point::new(dx / seg_len, dy / seg_len);
        let n_samples = (seg_len / 2.0).ceil() as usize;
        for i in 0..=n_samples {
            let t = if n_samples == 0 {
                0.0
            } else {
                i as f64 / n_samples as f64
            };
            let pos = Point::new(window[0].x + dx * t, window[0].y + dy * t);
            let w = measure_passage_width_at(passage, pos, dir);
            if w < min_width {
                min_width = w;
            }
        }
    }
    if min_width == f64::MAX {
        0.0
    } else {
        min_width
    }
}
