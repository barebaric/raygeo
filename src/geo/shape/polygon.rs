//! Polygon shapes and boolean operations.
//!
//! # Planar-only (XY-plane) operations
//!
//! All Boolean functions in this module (`offset_polygon_with_style`,
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

use crate::geo::shape::line::get_line_segment_closest_point;
use crate::types::{Edge, Point, Polygon, Rect};

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

/// Calculate the centroid of a polygon.
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
pub fn offset_polygon_with_style(
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
    let inward = offset_polygon_with_style(polygon, -r_min, JoinStyle::Miter);
    let mut result = Vec::new();
    for p in inward {
        result.extend(offset_polygon_with_style(&p, r_min, JoinStyle::Round));
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

/// Check if two polygons intersect.
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
