//! Polygon shapes and boolean operations.
//!
//! # Planar-only (XY-plane) operations
//!
//! All Boolean functions in this module (`offset_polygon`,
//! `get_polygons_union`, `get_polygons_intersection`,
//! `get_polygons_difference`, `get_polygons_group_intersection`,
//! `get_polygons_group_difference`) are **strictly 2D** — they operate on
//! `Polygon` (= `Vec<Point>`) and use [Clipper2] under the hood.  Z
//! coordinates are not modeled.
//!
//! 3D callers must project to the XY plane before calling these functions
//! and (if desired) lift the result back to the source Z afterwards.  See
//! [`crate::geo::algo::project`] for helpers.
//!
//! [Clipper2]: https://www.angusj.com/clipper2/Docs/Overview.htm

use clipper2::{
    difference as clipper_difference, intersect as clipper_intersect,
    simplify as clipper_simplify, union as clipper_union, EndType, FillRule,
    JoinType, Path as GeoPath, Paths as GeoPaths, Point as GeoPoint,
    PointInPolygonResult, PointScaler,
};

use crate::geo::shape::arc::normalize_angle_signed;
use crate::geo::shape::line::get_line_segment_closest_point;
use crate::geo::shape::line::get_segment_segment_distance;
use crate::types::{Edge, Point, Polygon, Rect};
use prof_macros::prof;

/// Join style for offset operations, matching clipper2 semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum JoinStyle {
    #[default]
    Miter,
    Round,
    Square,
}

/// Custom point scaler matching Python's CLIPPER_SCALE = 10^7.
#[derive(Debug, Default, Clone, Copy, PartialEq, Hash)]
pub struct GeoScale;

impl PointScaler for GeoScale {
    const MULTIPLIER: f64 = 10_000_000.0;
}

/// A clipper2 path using our custom GeoScale.
pub type ClipperPath = GeoPath<GeoScale>;

/// A clipper2 paths collection using our custom GeoScale.
pub type ClipperPaths = GeoPaths<GeoScale>;

pub fn is_almost_equal(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

pub fn polygon_to_path(polygon: &Polygon) -> ClipperPath {
    let tuples: Vec<(f64, f64)> = polygon.iter().map(|p| (p.x, p.y)).collect();
    ClipperPath::from(tuples)
}

pub fn path_to_polygon(path: &ClipperPath) -> Polygon {
    let tuples: Vec<(f64, f64)> = Vec::from(path.clone());
    tuples.iter().map(|(x, y)| Point::new(*x, *y)).collect()
}

pub fn paths_to_polygons(paths: &ClipperPaths) -> Vec<Polygon> {
    let tuples: Vec<Vec<(f64, f64)>> = Vec::from(paths.clone());
    tuples
        .iter()
        .map(|path| path.iter().map(|(x, y)| Point::new(*x, *y)).collect())
        .collect()
}

pub fn polygons_to_paths(polygons: &[Polygon]) -> ClipperPaths {
    let v: Vec<Vec<(f64, f64)>> = polygons
        .iter()
        .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
        .collect();
    ClipperPaths::from(v)
}

/// Calculate the signed area of a polygon using the shoelace formula.
pub fn get_polygon_signed_area(polygon: &[Point]) -> f64 {
    if polygon.len() < 3 {
        return 0.0;
    }
    let n = polygon.len();
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += polygon[i].perp_dot(polygon[j]);
    }
    area / 2.0
}

/// Calculate the absolute area of a polygon.
#[prof]
pub fn get_polygon_area(polygon: &Polygon) -> f64 {
    get_polygon_signed_area(polygon).abs()
}

/// Calculate the perimeter of a polygon.
pub fn get_polygon_perimeter(polygon: &Polygon) -> f64 {
    if polygon.len() < 2 {
        return 0.0;
    }
    let n = polygon.len();
    let mut perimeter = 0.0;
    for i in 0..n {
        let p1 = polygon[i];
        let p2 = polygon[(i + 1) % n];
        perimeter += p1.distance(p2);
    }
    perimeter
}

/// Perpendicular distance from a point to a line segment.
pub fn point_line_distance(
    point: Point,
    line_start: Point,
    line_end: Point,
) -> f64 {
    let line_vec = line_end - line_start;
    let line_len = line_vec.length();
    if line_len < 1e-6 {
        return point.distance(line_start);
    }
    let line_unit = line_vec.normalize();
    let point_vec = point - line_start;
    let mut proj_len = point_vec.dot(line_unit);
    proj_len = proj_len.max(0.0).min(line_len);
    let closest = line_start + line_unit * proj_len;
    point.distance(closest)
}

/// Extract all edges from a polygon as (start, end) point pairs.
pub fn get_polygon_edges(polygon: &Polygon) -> Vec<Edge> {
    if polygon.len() < 2 {
        return vec![];
    }
    let n = polygon.len();
    let mut edges = Vec::with_capacity(n);
    for i in 0..n {
        edges.push((polygon[i], polygon[(i + 1) % n]));
    }
    edges
}

/// Get the bounding box of a polygon as (min_x, min_y, max_x, max_y).
#[prof]
pub fn get_polygon_bounds(polygon: &Polygon) -> Rect {
    if polygon.is_empty() {
        return Rect::default();
    }
    let mut min_x = polygon[0].x;
    let mut max_x = polygon[0].x;
    let mut min_y = polygon[0].y;
    let mut max_y = polygon[0].y;
    for p in polygon {
        let x = p.x;
        let y = p.y;
        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }
        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }
    }
    Rect::new(min_x, min_y, max_x, max_y)
}

/// Get the bounding box of multiple polygons.
#[prof]
pub fn get_polygon_group_bounds(polygons: &[Polygon]) -> Rect {
    if polygons.is_empty() {
        return Rect::default();
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut has_points = false;
    for poly in polygons {
        for p in poly {
            let x = p.x;
            let y = p.y;
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
            has_points = true;
        }
    }
    if !has_points {
        return Rect::default();
    }
    Rect::new(min_x, min_y, max_x, max_y)
}

/// Translate a bounding box by a given offset.
pub fn translate_bounds(bounds: Rect, dx: f64, dy: f64) -> Rect {
    Rect::new(
        bounds.min.x + dx,
        bounds.min.y + dy,
        bounds.max.x + dx,
        bounds.max.y + dy,
    )
}

/// Normalize polygons so their minimum corner is at the origin.
pub fn normalize_polygons(polygons: &[Polygon]) -> (Vec<Polygon>, f64, f64) {
    if polygons.is_empty() {
        return (polygons.to_vec(), 0.0, 0.0);
    }
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    for poly in polygons {
        for p in poly {
            let x = p.x;
            let y = p.y;
            if x < min_x {
                min_x = x;
            }
            if y < min_y {
                min_y = y;
            }
        }
    }
    if min_x == f64::MAX {
        return (polygons.to_vec(), 0.0, 0.0);
    }
    let normalized: Vec<Polygon> = polygons
        .iter()
        .map(|p| translate_polygon(p, -min_x, -min_y))
        .collect();
    (normalized, min_x, min_y)
}

/// Flip a polygon horizontally and/or vertically.
pub fn flip_polygon(polygon: &Polygon, flip_h: bool, flip_v: bool) -> Polygon {
    polygon
        .iter()
        .map(|p| {
            Point::new(
                if flip_h { -p.x } else { p.x },
                if flip_v { -p.y } else { p.y },
            )
        })
        .collect()
}

/// Flip multiple polygons horizontally and/or vertically.
pub fn flip_polygons(
    polygons: &[Polygon],
    flip_h: bool,
    flip_v: bool,
) -> Vec<Polygon> {
    polygons
        .iter()
        .map(|p| flip_polygon(p, flip_h, flip_v))
        .collect()
}

/// Approximate a circle as an n-gon polygon.
#[prof]
pub fn get_circle_polygon(center: Point, radius: f64, n: usize) -> Polygon {
    let mut poly = Vec::with_capacity(n);
    for i in 0..n {
        let a = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        poly.push(Point::new(
            center.x + radius * a.cos(),
            center.y + radius * a.sin(),
        ));
    }
    poly
}

/// Compute the swept-area polygons of a line segment with a given radius.
///
/// Returns a rectangle (the Minkowski sum of the segment with a disk of
/// *radius*) plus two disks at the endpoints.
#[prof]
pub fn get_segment_swept_polygon(
    a: Point,
    b: Point,
    radius: f64,
) -> Vec<Polygon> {
    let dir = b - a;
    let len = dir.length();
    if len < 1e-12 {
        return vec![get_circle_polygon(a, radius, 64)];
    }
    let dir = dir / len;
    let perp = Point::new(-dir.y, dir.x);
    let rp = perp * radius;
    vec![
        vec![a - rp, b - rp, b + rp, a + rp],
        get_circle_polygon(a, radius, 64),
        get_circle_polygon(b, radius, 64),
    ]
}

/// The number of linear segments used to approximate a full-circle arc when
/// constructing swept‑polygon vertex arcs.  The actual subdivision count for
/// a given arc is `max(4, ceil(N_ARC * |span| / π))`.
pub const SWEPT_N_ARC: usize = 32;

/// Push interior points of a circular arc (excluding endpoints) into `pts`.
///
/// The arc is centred at `center`, starts at angle `a0` (radians), and
/// sweeps `span` radians counter-clockwise (positive) or clockwise
/// (negative).  Points are densely sampled so the chord error is
/// acceptably small for tool‑radius values.
///
/// When `|span| < 1e-6` nothing is pushed (the arc is degenerate).
pub fn push_arc_interior(
    pts: &mut Vec<Point>,
    center: Point,
    a0: f64,
    span: f64,
    r: f64,
) {
    if span.abs() < 1e-6 {
        return;
    }
    let n = ((SWEPT_N_ARC as f64 * span.abs() / std::f64::consts::PI).ceil()
        as usize)
        .max(4);
    for i in 1..n {
        let a = a0 + span * i as f64 / n as f64;
        pts.push(center + Point::new(a.cos() * r, a.sin() * r));
    }
}

/// Miter intersection of two offset lines through vertex `v`.
///
/// Line A:  `v + off_a  + t * dir_a`
/// Line B:  `v + off_b  + s * dir_b`
///
/// Returns the intersection point.  When the lines are (nearly) parallel
/// falls back to `v + off_a`.
pub fn miter_offset_intersection(
    v: Point,
    off_a: Point,
    dir_a: Point,
    off_b: Point,
    dir_b: Point,
) -> Point {
    let p0 = v + off_a;
    let p1 = v + off_b;
    let d = p1 - p0;
    let denom = dir_a.x * (-dir_b.y) - dir_a.y * (-dir_b.x);
    if denom.abs() < 1e-9 {
        return p0; // parallel — fall back
    }
    let t = (d.x * (-dir_b.y) - d.y * (-dir_b.x)) / denom;
    p0 + dir_a * t
}

/// Minkowski sum of a polyline path with a disk of `radius`.
///
/// Produces a single polygon covering the swept area — the union of
/// segment-wide rectangular strips capped with half-circles at the
/// first (rear cap) and last (front cap) endpoints.
///
/// At each interior vertex the two offset lines on the **concave** side
/// diverge, exposing the disk → a circular arc fills the gap.  On the
/// **convex** side the offset lines converge and cross; the disk is
/// fully shadowed by the strips, so a sharp Miter intersection replaces
/// the arc.
#[prof]
pub fn get_polyline_swept_polygon(path: &[Point], radius: f64) -> Vec<Polygon> {
    let n = path.len();
    if n < 2 {
        return vec![];
    }

    let mut pts: Vec<Point> = Vec::new();

    // Pre-compute per-segment data.
    struct Seg {
        dir: Point, // unit direction
        rp: Point,  // right-perp * radius = -perp * radius
        lp: Point,  // left-perp  * radius = +perp * radius
    }
    let mut segs: Vec<Seg> = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let d = (path[i + 1] - path[i]).normalize();
        let p = Point::new(-d.y, d.x);
        segs.push(Seg {
            dir: d,
            rp: -p * radius,
            lp: p * radius,
        });
    }

    // Cross product of consecutive segment directions.
    //   cross > 0  →  LEFT turn  (CCW)
    //   cross < 0  →  RIGHT turn (CW)
    let cross_at = |i: usize| -> f64 {
        segs[i].dir.x * segs[i + 1].dir.y - segs[i].dir.y * segs[i + 1].dir.x
    };

    // 1. Outer side:  follow the right-perp offset, walking forward.
    //    LEFT  turn → concave side → circular arc (gap exposes disk).
    //    RIGHT turn → convex side → Miter intersection (disk shadowed).
    for i in 0..n - 1 {
        let rp_i = &segs[i].rp;
        if i == 0 {
            pts.push(path[0] + *rp_i);
        }

        if i + 1 < n - 1 {
            let rp_nxt = &segs[i + 1].rp;
            let cross = cross_at(i);
            if cross > 1e-6 {
                // Concave → arc.
                pts.push(path[i + 1] + *rp_i);
                let a0 = rp_i.y.atan2(rp_i.x);
                let a1 = rp_nxt.y.atan2(rp_nxt.x);
                let span = normalize_angle_signed(a1 - a0);
                if span.abs() > 0.02 {
                    push_arc_interior(&mut pts, path[i + 1], a0, span, radius);
                }
                // Push start of next segment's rp offset — unless the next
                // turn (at path[i+2]) is a Miter, which replaces this
                // endpoint (it sits at dist=radius on the vertex disk).
                let next_is_miter = i + 2 < n - 1 && cross_at(i + 1) <= 1e-6;
                if !next_is_miter {
                    pts.push(path[i + 1] + *rp_nxt);
                }
            } else {
                // Convex (or straight) → Miter.
                pts.push(miter_offset_intersection(
                    path[i + 1],
                    *rp_i,
                    segs[i].dir,
                    *rp_nxt,
                    segs[i + 1].dir,
                ));
            }
        } else {
            pts.push(path[i + 1] + *rp_i);
        }
    }

    // 2. End cap: front half-circle at path[n-1] (outer → inner, +π).
    let s_last = &segs[n - 2];
    let a_last = s_last.rp.y.atan2(s_last.rp.x);
    push_arc_interior(
        &mut pts,
        path[n - 1],
        a_last,
        std::f64::consts::PI,
        radius,
    );
    pts.push(path[n - 1] + s_last.lp);

    // 3. Inner side: follow the left-perp offset, walking backward.
    //    RIGHT turn → concave side → circular arc (gap exposes disk).
    //    LEFT  turn → convex side → Miter intersection (disk shadowed).
    for i in (0..n - 1).rev() {
        if i == n - 2 {
            // Already pushed by the end‑cap endpoint above.
        } else {
            let lp_nxt = &segs[i + 1].lp;
            let lp_cur = &segs[i].lp;
            let cross = cross_at(i);
            if cross < -1e-6 {
                // Concave → arc.  (Directions reversed for backward walk.)
                pts.push(path[i + 1] + *lp_nxt);
                let a0 = lp_nxt.y.atan2(lp_nxt.x);
                let a1 = lp_cur.y.atan2(lp_cur.x);
                let span = normalize_angle_signed(a1 - a0);
                if span.abs() > 0.02 {
                    push_arc_interior(&mut pts, path[i + 1], a0, span, radius);
                }
                pts.push(path[i + 1] + *lp_cur);
            } else {
                // Convex (or straight) → Miter.
                pts.push(miter_offset_intersection(
                    path[i + 1],
                    *lp_nxt,
                    -segs[i + 1].dir,
                    *lp_cur,
                    -segs[i].dir,
                ));
            }
        }
        // Push the start of this segment's lp offset — unless the next
        // iteration (i-1) will use a Miter at this vertex, in which case
        // the Miter replaces this endpoint (which sits at dist=radius on
        // the vertex disk).
        let next_is_miter = i > 0 && cross_at(i - 1) > -1e-6;
        if !next_is_miter {
            pts.push(path[i] + segs[i].lp);
        }
    }

    // 4. Start cap: rear half-circle at path[0] (inner → outer, +π).
    let s_first = &segs[0];
    let a_first = s_first.rp.y.atan2(s_first.rp.x);
    push_arc_interior(
        &mut pts,
        path[0],
        a_first + std::f64::consts::PI,
        std::f64::consts::PI,
        radius,
    );

    vec![pts]
}

/// Pre-compute bounding boxes for a slice of polygons.
///
/// Returns a `Vec<Rect>` in the same order as `polygons`.
/// Useful when calling [`does_path_sweep_intersect_polygon`]
/// many times with the same obstacles — precompute once
/// and pass the bounds.
#[prof]
pub fn compute_polygon_bounds(polygons: &[Polygon]) -> Vec<Rect> {
    polygons.iter().map(get_polygon_bounds).collect()
}

/// True if the sweep of a disk of `radius` moving along `path` intersects
/// any polygon in `obstacles`. The sweep is the union of capsules
/// (rectangles capped with half-disks) one per segment, plus a disk at
/// each vertex — exactly `get_segment_swept_polygon` per segment.
///
/// Uses pre-computed bounding boxes for the obstacles. When calling this
/// function many times with the same obstacle set, precompute bounds
/// once via [`compute_polygon_bounds`] and pass them here.
///
/// The obstacle list may contain both CCW (positive-area) outer
/// boundaries and CW (negative-area) holes — a polygon-with-holes
/// representation as produced by Clipper2.  Point-in-polygon tests use
/// the NonZero winding rule: a point is "inside" the solid material
/// only when the signed coverage (CCW outers count +1, CW holes count
/// −1) is positive.  This prevents holes from being
/// treated as solid obstacles.
#[prof]
pub fn does_path_sweep_intersect_polygon(
    path: &[Point],
    radius: f64,
    obstacles: &[Polygon],
    obstacle_bounds: &[Rect],
) -> bool {
    if path.len() < 2 {
        return false;
    }

    // Precompute winding sign for each obstacle (+1 CCW, −1 CW).
    let signs: Vec<i8> = obstacles
        .iter()
        .map(|obs| {
            if get_polygon_signed_area(obs) > 0.0 {
                1
            } else {
                -1
            }
        })
        .collect();

    // Winding-number point-in-region test using NonZero rule.
    let point_in_region = |p: Point| -> bool {
        let mut winding = 0i32;
        for ((obs, bounds), &sign) in
            obstacles.iter().zip(obstacle_bounds).zip(&signs)
        {
            if obs.len() < 3 {
                continue;
            }
            if p.x < bounds.min.x
                || p.x > bounds.max.x
                || p.y < bounds.min.y
                || p.y > bounds.max.y
            {
                continue;
            }
            if is_point_in_polygon(p, obs) {
                winding += sign as i32;
            }
        }
        winding > 0
    };

    for (obstacle, obs_bounds) in obstacles.iter().zip(obstacle_bounds) {
        if obstacle.len() < 3 {
            continue;
        }

        for i in 0..path.len() - 1 {
            let a = path[i];
            let b = path[i + 1];

            let seg_min_x = a.x.min(b.x) - radius;
            let seg_min_y = a.y.min(b.y) - radius;
            let seg_max_x = a.x.max(b.x) + radius;
            let seg_max_y = a.y.max(b.y) + radius;
            if seg_max_x < obs_bounds.min.x
                || seg_min_x > obs_bounds.max.x
                || seg_max_y < obs_bounds.min.y
                || seg_min_y > obs_bounds.max.y
            {
                continue;
            }

            if point_in_region(a) || point_in_region(b) {
                return true;
            }

            let n = obstacle.len();
            for j in 0..n {
                let c = obstacle[j];
                let d = obstacle[(j + 1) % n];
                if get_segment_segment_distance(a, b, c, d) < radius {
                    return true;
                }
            }
        }
    }

    false
}

/// Calculate the centroid of a polygon.
#[prof]
pub fn get_polygon_centroid(polygon: &Polygon) -> Point {
    if polygon.is_empty() {
        return Point::new(0.0, 0.0);
    }
    let n = polygon.len();
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut signed_area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let cross = polygon[i].perp_dot(polygon[j]);
        signed_area += cross;
        cx += (polygon[i].x + polygon[j].x) * cross;
        cy += (polygon[i].y + polygon[j].y) * cross;
    }
    signed_area /= 2.0;
    if signed_area.abs() < 1e-9 {
        let sum_x: f64 = polygon.iter().map(|p| p.x).sum();
        let sum_y: f64 = polygon.iter().map(|p| p.y).sum();
        return Point::new(sum_x / n as f64, sum_y / n as f64);
    }
    cx /= 6.0 * signed_area;
    cy /= 6.0 * signed_area;
    Point::new(cx, cy)
}

/// Find the closest point on a polygon's boundary to a given point.
///
/// Returns `(t, closest_point, distance_squared)` where `t` is the
/// parametric position along the edge (0–1), `closest_point` is the
/// nearest point on the boundary, and `distance_squared` is the
/// squared Euclidean distance from `(x, y)` to that point.
///
/// Returns `None` when the polygon has fewer than 2 vertices.
#[prof]
pub fn get_polygon_closest_point(
    polygon: &Polygon,
    x: f64,
    y: f64,
) -> Option<(f64, Point, f64)> {
    let n = polygon.len();
    if n < 2 {
        return None;
    }
    let mut best: Option<(f64, Point, f64)> = None;
    for i in 0..n {
        let j = (i + 1) % n;
        let (t, pt, d2) =
            get_line_segment_closest_point(polygon[i], polygon[j], x, y);
        if best.is_none() || d2 < best.unwrap().2 {
            best = Some((t, pt, d2));
        }
    }
    best
}

/// Find the closest point on any polygon in `polygons` to the given
/// `point`.  Returns `(polygon_index, t, closest_point, distance_squared)`
/// where `polygon_index` is the index into `polygons`, `t` is the
/// parametric position along the closest edge (0–1), `closest_point` is
/// the nearest point on the boundary, and `distance_squared` is the
/// squared Euclidean distance.
///
/// Returns `None` when all polygons have fewer than 2 vertices.
pub fn get_polygons_closest_point(
    polygons: &[Polygon],
    point: Point,
) -> Option<(usize, f64, Point, f64)> {
    let mut best: Option<(usize, f64, Point, f64)> = None;
    for (pi, poly) in polygons.iter().enumerate() {
        if let Some((t, pt, d2)) =
            get_polygon_closest_point(poly, point.x, point.y)
        {
            if best.is_none() || d2 < best.unwrap().3 {
                best = Some((pi, t, pt, d2));
            }
        }
    }
    best
}

/// Signed perpendicular distance from a point to the nearest polygon boundary.
///
/// Positive means the point is *outside* the polygon group, negative means *inside*
/// the polygon group, and zero means exactly on a boundary.
///
/// **Important:** The polygon list may contain holes (CW polygons with negative
/// signed area). A point is considered inside the group only when it is inside
/// at least one CCW (positive‑area) polygon **and** not inside any CW (negative‑area)
/// hole polygon.
pub fn get_signed_boundary_distance(point: Point, polygons: &[Polygon]) -> f64 {
    let mut inside_ccw = false;
    let mut inside_cw = false;

    for poly in polygons {
        if poly.len() < 3 {
            continue;
        }
        if is_point_in_polygon(point, poly) {
            if get_polygon_signed_area(poly) > 0.0 {
                inside_ccw = true;
            } else {
                inside_cw = true;
            }
        }
    }

    let inside = inside_ccw && !inside_cw;

    let d = get_polygons_closest_point(polygons, point)
        .map(|(_, _, _, d2)| d2.sqrt())
        .unwrap_or(f64::MAX);

    if inside {
        -d.abs()
    } else {
        d
    }
}

/// Rotate a polygon around the origin.
pub fn rotate_polygon(polygon: &Polygon, angle_degrees: f64) -> Polygon {
    if polygon.is_empty() {
        return polygon.clone();
    }
    let angle_rad = angle_degrees.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    polygon
        .iter()
        .map(|p| {
            Point::new(p.x * cos_a - p.y * sin_a, p.x * sin_a + p.y * cos_a)
        })
        .collect()
}

/// Rotate multiple polygons around the origin.
pub fn rotate_polygons(
    polygons: &[Polygon],
    angle_degrees: f64,
) -> Vec<Polygon> {
    polygons
        .iter()
        .map(|p| rotate_polygon(p, angle_degrees))
        .collect()
}

/// Translate a polygon by a given offset.
pub fn translate_polygon(polygon: &Polygon, dx: f64, dy: f64) -> Polygon {
    polygon
        .iter()
        .map(|p| Point::new(p.x + dx, p.y + dy))
        .collect()
}

/// Translate multiple polygons by a given offset.
pub fn translate_polygons(
    polygons: &[Polygon],
    dx: f64,
    dy: f64,
) -> Vec<Polygon> {
    polygons
        .iter()
        .map(|p| translate_polygon(p, dx, dy))
        .collect()
}

/// Scale a polygon.
pub fn scale_polygon(polygon: &Polygon, sx: f64, sy: Option<f64>) -> Polygon {
    let sy = sy.unwrap_or(sx);
    polygon
        .iter()
        .map(|p| Point::new(p.x * sx, p.y * sy))
        .collect()
}

fn cross(o: Point, a: Point, b: Point) -> f64 {
    (a - o).perp_dot(b - o)
}

/// Check if a polygon is convex.
pub fn is_polygon_convex(polygon: &Polygon) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    if n == 3 {
        return true;
    }
    let mut sign: Option<bool> = None;
    for i in 0..n {
        let c = cross(polygon[i], polygon[(i + 1) % n], polygon[(i + 2) % n]);
        if c.abs() < 1e-10 {
            continue;
        }
        match sign {
            None => sign = Some(c > 0.0),
            Some(s) if (c > 0.0) != s => return false,
            _ => {}
        }
    }
    true
}

/// Compute the convex hull of a polygon using Andrew's monotone chain.
pub fn get_polygon_convex_hull(polygon: &Polygon) -> Polygon {
    if polygon.len() < 3 {
        return polygon.clone();
    }
    let mut sorted = polygon.clone();
    sorted.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let mut lower: Vec<Point> = Vec::new();
    for &p in &sorted {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0
        {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<Point> = Vec::new();
    for &p in sorted.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0
        {
            upper.pop();
        }
        upper.push(p);
    }
    lower[..lower.len() - 1]
        .iter()
        .chain(&upper[..upper.len() - 1])
        .copied()
        .collect()
}

/// Clean a polygon by removing duplicate and near-collinear points.
pub fn clean_polygon(polygon: &Polygon, tolerance: f64) -> Option<Polygon> {
    if polygon.len() < 3 {
        return None;
    }
    let path = polygon_to_path(polygon);
    let paths = ClipperPaths::from(path);
    let simplified = clipper_simplify(paths, 0.0, false);
    if simplified.is_empty() {
        return None;
    }
    let mut biggest = simplified.first().unwrap().clone();
    let mut biggest_area = biggest.signed_area().abs();
    for path in simplified.iter().skip(1) {
        let area = path.signed_area().abs();
        if area > biggest_area {
            biggest = path.clone();
            biggest_area = area;
        }
    }
    let clean_tol = GeoScale::scale(tolerance);
    let cleaned_paths = clipper_simplify(
        ClipperPaths::from(biggest),
        clean_tol / GeoScale::MULTIPLIER,
        false,
    );
    let cleaned = match cleaned_paths.get(0) {
        Some(p) => p.clone(),
        None => return None,
    };
    let mut result = path_to_polygon(&cleaned);
    if result.len() > 1 {
        let first = result[0];
        let last = result[result.len() - 1];
        if (first.x - last.x).abs() < 1e-9 && (first.y - last.y).abs() < 1e-9 {
            result.pop();
        }
    }
    if result.len() < 3 {
        return None;
    }
    Some(result)
}

/// Offset (inflate/deflate) a polygon with a specific join style.
///
/// **Planar (XY-plane only).** Z is not modeled.
pub fn offset_polygon(
    polygon: &Polygon,
    offset: f64,
    join_style: JoinStyle,
) -> Vec<Polygon> {
    if polygon.len() < 3 {
        return vec![];
    }
    if offset.abs() < 1e-9 {
        return vec![polygon.clone()];
    }
    let clipper_join = match join_style {
        JoinStyle::Miter => JoinType::Miter,
        JoinStyle::Round => JoinType::Round,
        JoinStyle::Square => JoinType::Square,
    };
    let path = polygon_to_path(polygon);
    let result = path.inflate(offset, clipper_join, EndType::Polygon, 2.0);
    let mut output = Vec::new();
    for p in result.iter() {
        let poly = path_to_polygon(p);
        if poly.len() >= 3 {
            output.push(poly);
        }
    }
    output
}

/// Enforce a minimum internal curvature radius on a polygon.
///
/// Performs a morphological opening: offsets inward by `r_min` using Miter
/// joins, then outward by `r_min` using Round joins. This acts as a
/// high-pass curvature filter — tight internal corners are filleted to
/// exactly `r_min`, while the overall shape is preserved.
///
/// **Planar (XY-plane only).** Z is not modeled.
pub fn apply_minimum_curvature(polygon: &Polygon, r_min: f64) -> Vec<Polygon> {
    if polygon.len() < 3 || r_min <= 0.0 {
        return vec![polygon.clone()];
    }
    let inward = offset_polygon(polygon, -r_min, JoinStyle::Miter);
    let mut result = Vec::new();
    for p in inward {
        result.extend(offset_polygon(&p, r_min, JoinStyle::Round));
    }
    result
}

/// Compute the union of multiple polygons.
///
/// **Planar (XY-plane only).** Uses Clipper2. Z is not modeled.
pub fn get_polygons_union(polygons: &[Polygon]) -> Vec<Polygon> {
    if polygons.is_empty() {
        return vec![];
    }
    if polygons.len() == 1 && polygons[0].len() >= 3 {
        let mut poly = polygons[0].clone();
        if get_polygon_signed_area(&poly) < 0.0 {
            poly.reverse();
        }
        return vec![poly];
    }
    let clipper_paths = polygons_to_paths(polygons);
    if clipper_paths.is_empty() {
        return vec![];
    }
    let result =
        clipper_union(clipper_paths.clone(), clipper_paths, FillRule::NonZero)
            .unwrap_or_default();
    paths_to_polygons(&result)
        .into_iter()
        .filter(|p| p.len() >= 3)
        .collect()
}

/// Compute the intersection of two groups of polygons (subject vs clip).
/// Equivalent to clipper CT_INTERSECTION between two sets of paths.
///
/// **Planar (XY-plane only).** Uses Clipper2. Z is not modeled.
#[prof]
pub fn get_polygons_group_intersection(
    subject: &[Polygon],
    clip: &[Polygon],
) -> Vec<Polygon> {
    if subject.is_empty() || clip.is_empty() {
        return vec![];
    }
    let subject_paths = polygons_to_paths(subject);
    let clip_paths = polygons_to_paths(clip);
    if subject_paths.is_empty() || clip_paths.is_empty() {
        return vec![];
    }
    let result =
        clipper_intersect(subject_paths, clip_paths, FillRule::NonZero)
            .unwrap_or_default();
    paths_to_polygons(&result)
        .into_iter()
        .filter(|p| p.len() >= 3)
        .collect()
}

/// Compute the intersection of two polygons.
///
/// **Planar (XY-plane only).** Uses Clipper2. Z is not modeled.
pub fn get_polygons_intersection(
    poly1: &Polygon,
    poly2: &Polygon,
) -> Vec<Polygon> {
    if poly1.len() < 3 || poly2.len() < 3 {
        return vec![];
    }
    let path1 = polygons_to_paths(std::slice::from_ref(poly1));
    let path2 = polygons_to_paths(std::slice::from_ref(poly2));
    let result =
        clipper_intersect(path1, path2, FillRule::NonZero).unwrap_or_default();
    paths_to_polygons(&result)
        .into_iter()
        .filter(|p| p.len() >= 3)
        .collect()
}

/// Compute the difference of two groups of polygons (subject - clip).
/// Equivalent to clipper CT_DIFFERENCE between two sets of paths.
///
/// **Planar (XY-plane only).** Uses Clipper2. Z is not modeled.
#[prof]
pub fn get_polygons_group_difference(
    subject: &[Polygon],
    clip: &[Polygon],
) -> Vec<Polygon> {
    if subject.is_empty() {
        return vec![];
    }
    let subject_paths = polygons_to_paths(subject);
    let clip_paths = polygons_to_paths(clip);
    if subject_paths.is_empty() {
        return vec![];
    }
    let result =
        clipper_difference(subject_paths, clip_paths, FillRule::NonZero)
            .unwrap_or_default();
    paths_to_polygons(&result)
        .into_iter()
        .filter(|p| p.len() >= 3)
        .collect()
}

/// Compute the difference of two polygons (poly1 - poly2).
///
/// **Planar (XY-plane only).** Uses Clipper2. Z is not modeled.
pub fn get_polygons_difference(
    poly1: &Polygon,
    poly2: &Polygon,
) -> Vec<Polygon> {
    if poly1.len() < 3 || poly2.len() < 3 {
        if poly1.len() >= 3 {
            return vec![poly1.clone()];
        }
        return vec![];
    }
    let path1 = polygons_to_paths(std::slice::from_ref(poly1));
    let path2 = polygons_to_paths(std::slice::from_ref(poly2));
    let result =
        clipper_difference(path1, path2, FillRule::NonZero).unwrap_or_default();
    paths_to_polygons(&result)
        .into_iter()
        .filter(|p| p.len() >= 3)
        .collect()
}

/// Determines if a polygon is wound in clockwise order using the signed area (shoelace).
pub fn is_polygon_clockwise(points: &[Point]) -> bool {
    points.len() >= 3 && get_polygon_signed_area(points) < 0.0
}

/// Tests if a point is inside a polygon using the ray casting algorithm.
/// Uses a bounding box early-out for performance, and handles edge cases
/// where the point lies exactly on a polygon edge.
pub fn is_point_in_polygon(point: Point, polygon: &Polygon) -> bool {
    let x = point.x;
    let y = point.y;
    let n = polygon.len();
    if n < 3 {
        return false;
    }

    let mut min_x = polygon[0].x;
    let mut max_x = polygon[0].x;
    let mut min_y = polygon[0].y;
    let mut max_y = polygon[0].y;

    for p in polygon {
        let px = p.x;
        let py = p.y;
        if px < min_x {
            min_x = px;
        } else if px > max_x {
            max_x = px;
        }
        if py < min_y {
            min_y = py;
        } else if py > max_y {
            max_y = py;
        }
    }

    if x < min_x || x > max_x || y < min_y || y > max_y {
        return false;
    }

    for i in 0..n {
        let p1 = polygon[i];
        let p2 = polygon[(i + 1) % n];
        let p1x = p1.x;
        let p1y = p1.y;
        let p2x = p2.x;
        let p2y = p2.y;

        let cross_product = (Point::new(x, y) - p1).perp_dot(p2 - p1);
        if cross_product.abs() < 1e-9
            && p1x.min(p2x) <= x
            && x <= p1x.max(p2x)
            && p1y.min(p2y) <= y
            && y <= p1y.max(p2y)
        {
            return true;
        }
    }

    let mut inside = false;
    let mut p1x = polygon[0].x;
    let mut p1y = polygon[0].y;
    for i in 0..=n {
        let p2x = polygon[i % n].x;
        let p2y = polygon[i % n].y;
        if (p1y > y) != (p2y > y) {
            let x_intersect = (y - p1y) * (p2x - p1x) / (p2y - p1y) + p1x;
            if x_intersect > x {
                inside = !inside;
            }
        }
        p1x = p2x;
        p1y = p2y;
    }

    inside
}

/// Alias for is_point_in_polygon for compatibility.
pub fn is_point_inside_polygon(point: Point, polygon: &Polygon) -> bool {
    is_point_in_polygon(point, polygon)
}

/// Returns `true` if `polygon` fully encloses a circle of `radius` at `center`.
///
/// Three checks are applied in increasing cost:
/// 1. The polygon's AABB must contain the circle's bounding box.
/// 2. The circle centre must lie inside the polygon.
/// 3. Every edge of the polygon must be at least `radius` away from the
///    centre (handles concave shapes whose AABB and centroid satisfy
///    checks 1–2 but whose notch cuts through the disk).
pub fn does_polygon_enclose_circle(
    center: Point,
    radius: f64,
    polygon: &Polygon,
) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let circle_rect = Rect::new(
        center.x - radius,
        center.y - radius,
        center.x + radius,
        center.y + radius,
    );
    let bounds = get_polygon_bounds(polygon);
    if !crate::geo::shape::rect::does_rect_contain_rect(bounds, circle_rect) {
        return false;
    }
    if !is_point_in_polygon(center, polygon) {
        return false;
    }
    let n = polygon.len();
    for i in 0..n {
        let a = polygon[i];
        let b = polygon[(i + 1) % n];
        let dist = super::line::get_point_line_distance(center, a, b);
        if dist < radius {
            return false;
        }
    }
    true
}

/// Check if a point is inside a polygon using clipper2.
pub fn point_in_polygon_clipper(point: Point, polygon: &Polygon) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    // Check if point is on any edge (clipper2 does not count edge points as inside)
    for i in 0..polygon.len() {
        let j = (i + 1) % polygon.len();
        if super::line::is_point_on_segment(point, polygon[i], polygon[j]) {
            return true;
        }
    }
    let path = polygon_to_path(polygon);
    let geo_point = GeoPoint::<GeoScale>::new(point.x, point.y);
    path.is_point_inside(geo_point) == PointInPolygonResult::IsInside
}

/// Resample a closed polygon by inserting evenly-spaced points along each
/// edge so that no segment is longer than `spacing`.
///
/// The result is a closed polyline (last point connects back to first
/// conceptually, but is not duplicated).  Useful for preparing boundary
/// data for algorithms like the medial-axis transform that require dense,
/// uniform sampling.
pub fn resample_polygon(poly: &[Point], spacing: f64) -> Vec<Point> {
    if poly.is_empty() {
        return vec![];
    }
    let mut result = Vec::new();
    for i in 0..poly.len() {
        let j = (i + 1) % poly.len();
        let dx = poly[j].x - poly[i].x;
        let dy = poly[j].y - poly[i].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 {
            result.push(poly[i]);
            continue;
        }
        let n = (len / spacing).ceil() as usize;
        for k in 0..n {
            let t = k as f64 / n as f64;
            result.push(Point::new(poly[i].x + t * dx, poly[i].y + t * dy));
        }
    }
    result
}

/// Arithmetic mean of polygon vertices (simple vertex-average centroid).
///
/// This differs from [`get_polygon_centroid`] which uses the area-weighted
/// (shoelace) centroid.  The vertex average is useful when the spatial
/// *arrangement* of vertices matters (e.g. finding the middle of a
/// concave polygon whose area centroid would lie outside the boundary).
#[prof]
pub fn get_polygon_vertex_centroid(poly: &[Point]) -> Point {
    if poly.is_empty() {
        return Point::new(0.0, 0.0);
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    for p in poly {
        cx += p.x;
        cy += p.y;
    }
    Point::new(cx / poly.len() as f64, cy / poly.len() as f64)
}

/// Minimum midpoint-to-segment distance between the boundaries of two
/// polygons.
///
/// Uses segment **midpoints** rather than raw segment-segment distance
/// to avoid false positives from polygons that merely touch at a shared
/// vertex (where endpoint-to-endpoint distance is 0 but no boundary
/// edge is actually shared).
#[prof]
pub fn get_polygon_boundary_distance(a: &[Point], b: &[Point]) -> f64 {
    if a.len() < 2 || b.len() < 2 {
        return f64::MAX;
    }
    let na = a.len();
    let nb = b.len();
    let mut min_d = f64::MAX;

    for i in 0..na {
        let ai = (i + 1) % na;
        let mid =
            Point::new((a[i].x + a[ai].x) * 0.5, (a[i].y + a[ai].y) * 0.5);
        for j in 0..nb {
            let bj = (j + 1) % nb;
            let (_, _, d2) =
                get_line_segment_closest_point(b[j], b[bj], mid.x, mid.y);
            if d2 < min_d {
                min_d = d2;
            }
        }
    }

    for j in 0..nb {
        let bj = (j + 1) % nb;
        let mid =
            Point::new((b[j].x + b[bj].x) * 0.5, (b[j].y + b[bj].y) * 0.5);
        for i in 0..na {
            let ai = (i + 1) % na;
            let (_, _, d2) =
                get_line_segment_closest_point(a[i], a[ai], mid.x, mid.y);
            if d2 < min_d {
                min_d = d2;
            }
        }
    }

    min_d.sqrt()
}

pub fn polygons_intersect(
    poly1: &Polygon,
    poly2: &Polygon,
    min_area: f64,
) -> bool {
    if poly1.len() < 3 || poly2.len() < 3 {
        return false;
    }
    let intersection = get_polygons_intersection(poly1, poly2);
    if intersection.is_empty() {
        return false;
    }
    if min_area <= 0.0 {
        return true;
    }
    // min_area is specified in clipper integer coordinates (scale^2),
    // convert to float area for comparison
    let scale = GeoScale::MULTIPLIER;
    let min_area_float = min_area / (scale * scale);
    for poly in &intersection {
        if get_polygon_area(poly) > min_area_float {
            return true;
        }
    }
    false
}

/// Outward-facing heading angle (radians) at a point on a closed polygon.
///
/// Collects all polygon edges whose closest distance to `vertex` is within
/// epsilon and averages their outward normals.  When `vertex` lies at a
/// polygon corner this produces the **bisector** direction (the average of
/// both incident edge normals), giving a smooth outward direction rather
/// than snapping to a single edge.  At a collinear point the two normals
/// are identical, so the result is unchanged.
///
/// Winding is detected via [`get_polygon_signed_area`]:
///
/// | Winding  | Outward normal       |
/// |----------|----------------------|
/// | CCW (>0) | right normal  (dy, -dx) |
/// | CW  (<0) | left normal   (-dy, dx) |
///
/// For a CCW outer polygon this is the true outward (exterior-facing)
/// direction.  For a CW hole it is also outward (away from the hole
/// interior, i.e. into the surrounding material).
///
/// Returns `0.0` when the polygon has fewer than 3 vertices or the
/// averaged normal is the zero vector.
pub fn get_polygon_heading_at(polygon: &[Point], vertex: Point) -> f64 {
    let n = polygon.len();
    if n < 3 {
        return 0.0;
    }

    const EPS: f64 = 1e-12;
    let signed_area = get_polygon_signed_area(polygon);
    let compute_outward = |edge_dir: Point| -> Point {
        if signed_area >= 0.0 {
            Point::new(edge_dir.y, -edge_dir.x)
        } else {
            Point::new(-edge_dir.y, edge_dir.x)
        }
    };

    // Find all edges whose closest distance to vertex is within EPS
    // and average their unit outward normals so each edge contributes
    // equally regardless of length.
    let mut normal_sum = Point::new(0.0, 0.0);
    let mut count: usize = 0;

    for i in 0..n {
        let j = (i + 1) % n;
        let (_, _, d2) = get_line_segment_closest_point(
            polygon[i], polygon[j], vertex.x, vertex.y,
        );
        if d2 < EPS {
            let edge_dir = polygon[j] - polygon[i];
            let len_sq = edge_dir.length_squared();
            if len_sq < EPS {
                continue;
            }
            let outward = compute_outward(edge_dir);
            let inv_len = 1.0 / len_sq.sqrt();
            normal_sum += Point::new(outward.x * inv_len, outward.y * inv_len);
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let avg =
        Point::new(normal_sum.x / count as f64, normal_sum.y / count as f64);
    if avg.length_squared() < EPS {
        return 0.0;
    }
    avg.y.atan2(avg.x)
}

/// Walk polygon vertices forward (in storage order, wrapping around),
/// starting from the vertex closest to `start`.
///
/// For each vertex `visit(idx, &point)` is called.  The walk stops at the
/// first invocation that returns `Some(result)` and that result is returned.
/// If every call returns `None` the overall result is `None`.
///
/// # Direction
///
/// "Forward" means increasing vertex index with wrap-around:
/// `closest, closest+1, …, n-1, 0, 1, …, closest-1`.
/// This follows the polygon's natural storage order.
///
/// * For a **CCW outer polygon** (signed area > 0) forward walks
///   **counter-clockwise** — the interior stays on the left.
/// * For a **CW hole** (signed area < 0) forward walks **clockwise** —
///   the interior (hole) stays on the right.
///
/// The direction never reverses; the walk covers all `n` vertices exactly
/// once (unless the visit callback short-circuits).
pub fn walk_polygon_from_point<T>(
    polygon: &[Point],
    start: Point,
    mut visit: impl FnMut(usize, &Point) -> Option<T>,
) -> Option<T> {
    let n = polygon.len();
    if n < 3 {
        return None;
    }

    let start_idx = polygon
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(start)
                .partial_cmp(&b.distance_squared(start))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)?;

    for offset in 0..n {
        let idx = (start_idx + offset) % n;
        if let result @ Some(_) = visit(idx, &polygon[idx]) {
            return result;
        }
    }
    None
}
