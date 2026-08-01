//! Clipping: Line segment clipping and region operations.
//!
//! # Convention exception
//!
//! Unlike the rest of the codebase where `bare` = 2D and `_3d` = 3D, this
//! module uses the `_2d` suffix to mean "pure XY plane (no Z)". Functions
//! without the suffix are **2.5D wrappers** — the clip is performed in XY
//! and Z is linearly interpolated from the input points.
//!
//! Callers of the 2D cores must explicitly project 3D data before calling;
//! see [`crate::geo::types`] (or [`super::project`]) for helpers.

use crate::geo::shape::line::get_line_segment_polygon_intersections_into;
use crate::geo::shape::polygon::is_point_inside_polygon;
use crate::geo::types::{Point, Point3D, Polygon, Rect};

// ── Cohen–Sutherland outcodes ─────────────────────────────────────────

const INSIDE: i32 = 0;
const LEFT: i32 = 1;
const RIGHT: i32 = 2;
const BOTTOM: i32 = 4;
const TOP: i32 = 8;

fn compute_outcode_2d(x: f64, y: f64, rect: Rect) -> i32 {
    let mut code = INSIDE;
    if x < rect.min.x {
        code |= LEFT;
    } else if x > rect.max.x {
        code |= RIGHT;
    }
    if y < rect.min.y {
        code |= BOTTOM;
    } else if y > rect.max.y {
        code |= TOP;
    }
    code
}

// ── 2D core: rectangle clip ──────────────────────────────────────────

/// Pure 2D Cohen-Sutherland line clipping against an axis-aligned rectangle.
///
/// **XY-plane only.** No Z involvement. Returns the clipped segment or `None`
/// if the segment lies entirely outside the rectangle.
pub fn clip_line_segment_with_rect_2d(
    p1: Point,
    p2: Point,
    rect: Rect,
) -> Option<(Point, Point)> {
    let (mut x1, mut y1) = (p1.x, p1.y);
    let (mut x2, mut y2) = (p2.x, p2.y);
    let (dx, dy) = (x2 - x1, y2 - y1);
    let (x_min, y_min, x_max, y_max) =
        (rect.min.x, rect.min.y, rect.max.x, rect.max.y);

    let mut outcode1 = compute_outcode_2d(x1, y1, rect);
    let mut outcode2 = compute_outcode_2d(x2, y2, rect);

    loop {
        if (outcode1 | outcode2) == 0 {
            return Some((Point::new(x1, y1), Point::new(x2, y2)));
        }
        if (outcode1 & outcode2) != 0 {
            return None;
        }

        let outcode_out = if outcode1 != 0 { outcode1 } else { outcode2 };
        let (mut x, mut y) = (0.0, 0.0);

        if (outcode_out & TOP) != 0 {
            y = y_max;
            x = if dy != 0.0 {
                x1 + dx * (y_max - y1) / dy
            } else {
                x1
            };
        } else if (outcode_out & BOTTOM) != 0 {
            y = y_min;
            x = if dy != 0.0 {
                x1 + dx * (y_min - y1) / dy
            } else {
                x1
            };
        } else if (outcode_out & RIGHT) != 0 {
            x = x_max;
            y = if dx != 0.0 {
                y1 + dy * (x_max - x1) / dx
            } else {
                y1
            };
        } else if (outcode_out & LEFT) != 0 {
            x = x_min;
            y = if dx != 0.0 {
                y1 + dy * (x_min - x1) / dx
            } else {
                y1
            };
        }

        if outcode_out == outcode1 {
            x1 = x;
            y1 = y;
            outcode1 = compute_outcode_2d(x1, y1, rect);
        } else {
            x2 = x;
            y2 = y;
            outcode2 = compute_outcode_2d(x2, y2, rect);
        }
    }
}

// ── 2.5D wrapper: rectangle clip ─────────────────────────────────────

/// Clips a line segment to an axis-aligned 2D rectangle using
/// the Cohen-Sutherland algorithm.
///
/// **2.5D:** The clip test is performed in the XY plane. Z-coordinates are
/// linearly interpolated from the input points. This is not a true 3D clip;
/// for a pure 2D version see [`clip_line_segment_with_rect_2d`].
pub fn clip_line_segment_with_rect(
    p1: Point3D,
    p2: Point3D,
    rect: Rect,
) -> Option<(Point3D, Point3D)> {
    let p1_2d = Point::new(p1.x, p1.y);
    let p2_2d = Point::new(p2.x, p2.y);
    let clipped_2d = clip_line_segment_with_rect_2d(p1_2d, p2_2d, rect)?;

    let dz = p2.z - p1.z;
    let delta_2d = p2_2d - p1_2d;
    let len_sq = delta_2d.length_squared();

    let interpolate_z = |pt: Point| -> f64 {
        if len_sq < 1e-30 {
            p1.z
        } else {
            let t = (pt - p1_2d).dot(delta_2d) / len_sq;
            p1.z + t * dz
        }
    };

    let (c1, c2) = clipped_2d;
    Some((
        Point3D::new(c1.x, c1.y, interpolate_z(c1)),
        Point3D::new(c2.x, c2.y, interpolate_z(c2)),
    ))
}

// ── 2D core: polygon-based clip / subtract ───────────────────────────

/// Internal helper: compute kept t-ranges for a 2D segment against polygons.
///
/// When `keep_inside = true` the returned ranges are the portions **inside**
/// the regions (clip); when `false` they are the portions **outside** (subtract).
fn compute_kept_t_ranges(
    p1: Point,
    p2: Point,
    regions: &[Polygon],
    keep_inside: bool,
) -> Vec<(f64, f64)> {
    if regions.is_empty() {
        return vec![];
    }

    let mut cut_buf = Vec::new();
    get_line_segment_polygon_intersections_into(p1, p2, regions, &mut cut_buf);
    let sorted_cuts = &*cut_buf;

    let mut kept: Vec<(f64, f64)> = Vec::new();
    for i in 0..sorted_cuts.len().saturating_sub(1) {
        let t1 = sorted_cuts[i];
        let t2 = sorted_cuts[i + 1];
        if (t1 - t2).abs() < 1e-9 {
            continue;
        }

        let mid_t = (t1 + t2) / 2.0;
        let mid_p = p1.lerp(p2, mid_t);

        let is_inside =
            regions.iter().any(|r| is_point_inside_polygon(mid_p, r));

        if (keep_inside && is_inside) || (!keep_inside && !is_inside) {
            kept.push((t1, t2));
        }
    }

    kept
}

/// Pure 2D: returns the sub-segments of a 2D line segment that lie **inside**
/// a list of polygon regions.
///
/// **XY-plane only.** No Z involvement.
pub fn clip_line_segment_with_polygons_2d(
    p1: Point,
    p2: Point,
    regions: &[Polygon],
) -> Vec<(Point, Point)> {
    let ranges = compute_kept_t_ranges(p1, p2, regions, true);
    ranges
        .into_iter()
        .map(|(t1, t2)| (p1.lerp(p2, t1), p1.lerp(p2, t2)))
        .collect()
}

/// Pure 2D: returns the sub-segments of a 2D line segment that lie **outside**
/// a list of polygon regions (inverse of clip).
///
/// **XY-plane only.** No Z involvement.
pub fn subtract_polygons_from_line_segment_2d(
    p1: Point,
    p2: Point,
    regions: &[Polygon],
) -> Vec<(Point, Point)> {
    let ranges = compute_kept_t_ranges(p1, p2, regions, false);
    ranges
        .into_iter()
        .map(|(t1, t2)| (p1.lerp(p2, t1), p1.lerp(p2, t2)))
        .collect()
}

// ── 2.5D wrappers: polygon-based clip / subtract ─────────────────────

/// Returns the sub-segments of a line segment that lie **inside** a list of
/// polygons.  This is the inverse of `subtract_polygons_from_line_segment`.
///
/// **2.5D:** The point-in-polygon test is performed in the XY plane.
/// Z-coordinates are linearly interpolated from the input points. For a pure
/// 2D version see [`clip_line_segment_with_polygons_2d`].
pub fn clip_line_segment_with_polygons(
    p1: Point3D,
    p2: Point3D,
    regions: &[Polygon],
) -> Vec<(Point3D, Point3D)> {
    let p1_2d = Point::new(p1.x, p1.y);
    let p2_2d = Point::new(p2.x, p2.y);
    let ranges = compute_kept_t_ranges(p1_2d, p2_2d, regions, true);
    ranges
        .into_iter()
        .map(|(t1, t2)| (p1.lerp(p2, t1), p1.lerp(p2, t2)))
        .collect()
}

/// Calculates the sub-segments of a line that lie **outside** a list of
/// polygons.
///
/// **2.5D:** The point-in-polygon test is performed in the XY plane.
/// Z-coordinates are linearly interpolated from the input points. For a pure
/// 2D version see [`subtract_polygons_from_line_segment_2d`].
pub fn subtract_polygons_from_line_segment(
    p1: Point3D,
    p2: Point3D,
    regions: &[Polygon],
) -> Vec<(Point3D, Point3D)> {
    let p1_2d = Point::new(p1.x, p1.y);
    let p2_2d = Point::new(p2.x, p2.y);
    let ranges = compute_kept_t_ranges(p1_2d, p2_2d, regions, false);
    ranges
        .into_iter()
        .map(|(t1, t2)| (p1.lerp(p2, t1), p1.lerp(p2, t2)))
        .collect()
}
