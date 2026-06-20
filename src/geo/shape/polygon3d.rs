//! 3D wrappers for polygon boolean and offset operations.
//!
//! Functions in this module accept [`Polygon3D`] input, extract the XY
//! coordinates for the geometric operation (performed in the XY plane via
//! Clipper2), and re-apply the source Z to the output.
//!
//! # Planarity
//!
//! Each input polygon **must** be planar — all vertices should share the
//! same Z coordinate.  The Z of the first vertex is used for the entire
//! output.  For non-planar data use the explicit projection helpers in
//! [`crate::geo::algo::project`] together with the 2D functions from
//! [`super::polygon`].
//!
//! # Usage
//!
//! ```rust
//! use raygeo::types::{Point3D, Polygon3D};
//! use raygeo::geo::shape::polygon3d::{get_polygons_union_3d, offset_polygon_3d};
//!
//! let poly: Polygon3D = vec![
//!     Point3D::new(0.0, 0.0, 5.0),
//!     Point3D::new(10.0, 0.0, 5.0),
//!     Point3D::new(10.0, 10.0, 5.0),
//!     Point3D::new(0.0, 10.0, 5.0),
//! ];
//! let inflated = offset_polygon_3d(&poly, 1.0);
//! assert!(inflated[0][0].z == 5.0);
//! ```
//!
//! [`Polygon3D`]: crate::types::Polygon3D

use crate::types::{Point, Point3D, Polygon, Polygon3D, Rect3D, Segment3D};

use super::polygon::{
    get_polygon_convex_hull as get_polygon_convex_hull_2d,
    get_polygons_difference, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_intersection,
    get_polygons_union as get_polygons_union_2d, offset_polygon_with_style,
    JoinStyle,
};

// ── internal helpers ──────────────────────────────────────────────────

fn first_z(poly: &[Point3D]) -> f64 {
    poly.first().map_or(0.0, |p| p.z)
}

/// Extract the XY projection and the shared Z from a 3D polygon.
fn project(poly: &Polygon3D) -> (f64, Polygon) {
    let z = first_z(poly);
    let poly_2d: Polygon = poly.iter().map(|p| Point::new(p.x, p.y)).collect();
    (z, poly_2d)
}

/// Re-apply a Z height to every vertex of every output polygon.
fn lift(polys: Vec<Polygon>, z: f64) -> Vec<Polygon3D> {
    polys
        .into_iter()
        .map(|p| {
            p.into_iter()
                .map(|pt| Point3D::new(pt.x, pt.y, z))
                .collect()
        })
        .collect()
}

/// Determine the Z for a group of 3D polygons.
/// Returns `(common_z, projected_2d_polygons)`.
/// If all polygons have the same Z (within 1e-9), that Z is returned.
/// Otherwise the Z of the first vertex of the first polygon is used.
fn project_group(polygons: &[Polygon3D]) -> (f64, Vec<Polygon>) {
    let z = polygons
        .first()
        .map(|p| first_z(p.as_slice()))
        .unwrap_or(0.0);
    let projected: Vec<Polygon> = polygons
        .iter()
        .map(|poly| poly.iter().map(|p| Point::new(p.x, p.y)).collect())
        .collect();
    (z, projected)
}

/// Project a subject and clip group, using the subject's Z for the output.
fn project_subject_clip(
    subject: &[Polygon3D],
    clip: &[Polygon3D],
) -> (f64, Vec<Polygon>, Vec<Polygon>) {
    let z = subject
        .first()
        .and_then(|p| p.first())
        .map(|p| p.z)
        .unwrap_or(0.0);
    let subj_2d: Vec<Polygon> = subject
        .iter()
        .map(|poly| poly.iter().map(|p| Point::new(p.x, p.y)).collect())
        .collect();
    let clip_2d: Vec<Polygon> = clip
        .iter()
        .map(|poly| poly.iter().map(|p| Point::new(p.x, p.y)).collect())
        .collect();
    (z, subj_2d, clip_2d)
}

// ── Public API ────────────────────────────────────────────────────────

/// Compute the union of multiple 3D polygons.
///
/// **Planar XY-plane operation with Z preservation.**  The Z of the first
/// vertex of the first polygon is applied to all output vertices.
pub fn get_polygons_union_3d(polygons: &[Polygon3D]) -> Vec<Polygon3D> {
    if polygons.is_empty() {
        return vec![];
    }
    let (z, projected) = project_group(polygons);
    let result = get_polygons_union_2d(&projected);
    lift(result, z)
}

/// Compute the intersection of two 3D polygons.
///
/// **Planar XY-plane operation with Z preservation.**  The Z of the first
/// vertex of the first input is applied to all output vertices.
pub fn get_polygons_intersection_3d(
    poly1: &Polygon3D,
    poly2: &Polygon3D,
) -> Vec<Polygon3D> {
    let z = poly1.first().map_or(0.0, |p| p.z);
    let p1_2d: Polygon = poly1.iter().map(|p| Point::new(p.x, p.y)).collect();
    let p2_2d: Polygon = poly2.iter().map(|p| Point::new(p.x, p.y)).collect();
    let result = get_polygons_intersection(&p1_2d, &p2_2d);
    lift(result, z)
}

/// Compute the difference of two 3D polygons (poly1 - poly2).
///
/// **Planar XY-plane operation with Z preservation.**  The Z of the first
/// vertex of the first input is applied to all output vertices.
pub fn get_polygons_difference_3d(
    poly1: &Polygon3D,
    poly2: &Polygon3D,
) -> Vec<Polygon3D> {
    let z = poly1.first().map_or(0.0, |p| p.z);
    let p1_2d: Polygon = poly1.iter().map(|p| Point::new(p.x, p.y)).collect();
    let p2_2d: Polygon = poly2.iter().map(|p| Point::new(p.x, p.y)).collect();
    let result = get_polygons_difference(&p1_2d, &p2_2d);
    lift(result, z)
}

/// Compute the intersection of two groups of 3D polygons (subject vs clip).
///
/// **Planar XY-plane operation with Z preservation.**  The Z of the first
/// vertex of the first subject polygon is applied to all output vertices.
pub fn get_polygons_group_intersection_3d(
    subject: &[Polygon3D],
    clip: &[Polygon3D],
) -> Vec<Polygon3D> {
    if subject.is_empty() || clip.is_empty() {
        return vec![];
    }
    let (z, subj_2d, clip_2d) = project_subject_clip(subject, clip);
    let result = get_polygons_group_intersection(&subj_2d, &clip_2d);
    lift(result, z)
}

/// Compute the difference of two groups of 3D polygons (subject - clip).
///
/// **Planar XY-plane operation with Z preservation.**  The Z of the first
/// vertex of the first subject polygon is applied to all output vertices.
pub fn get_polygons_group_difference_3d(
    subject: &[Polygon3D],
    clip: &[Polygon3D],
) -> Vec<Polygon3D> {
    if subject.is_empty() {
        return vec![];
    }
    let (z, subj_2d, clip_2d) = project_subject_clip(subject, clip);
    let result = get_polygons_group_difference(&subj_2d, &clip_2d);
    lift(result, z)
}

/// Offset (inflate/deflate) a closed 3D polygon.
///
/// **Planar XY-plane operation with Z preservation.**  The Z of the first
/// vertex of the input is applied to all output vertices.
///
/// The underlying offset uses Clipper2's `inflate` in the XY plane.
pub fn offset_polygon_3d(polygon: &Polygon3D, offset: f64) -> Vec<Polygon3D> {
    if polygon.len() < 3 {
        return vec![];
    }
    let (z, projected) = project(polygon);
    let result =
        offset_polygon_with_style(&projected, offset, JoinStyle::Miter);
    lift(result, z)
}

// ── 3D Analytical functions ──────────────────────────────────────────

/// Compute the perimeter of a 3D polygon using full 3D edge lengths.
pub fn get_polygon_perimeter_3d(polygon: &[Point3D]) -> f64 {
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

/// Signed XY-projected area of a 3D polygon (shoelace formula).
///
/// Positive for CCW winding, negative for CW.
pub fn get_polygon_signed_area_3d(polygon: &[Point3D]) -> f64 {
    if polygon.len() < 3 {
        return 0.0;
    }
    let n = polygon.len();
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += polygon[i].truncate().perp_dot(polygon[j].truncate());
    }
    area / 2.0
}

/// XY-projected area of a 3D polygon (absolute shoelace area).
pub fn get_polygon_area_3d(polygon: &[Point3D]) -> f64 {
    get_polygon_signed_area_3d(polygon).abs()
}

/// Get the 3D bounding box of a polygon (includes Z extents).
pub fn get_polygon_bounds_3d(polygon: &[Point3D]) -> Rect3D {
    if polygon.is_empty() {
        return Rect3D::default();
    }
    let mut min = polygon[0];
    let mut max = polygon[0];
    for p in polygon {
        if p.x < min.x {
            min.x = p.x;
        }
        if p.x > max.x {
            max.x = p.x;
        }
        if p.y < min.y {
            min.y = p.y;
        }
        if p.y > max.y {
            max.y = p.y;
        }
        if p.z < min.z {
            min.z = p.z;
        }
        if p.z > max.z {
            max.z = p.z;
        }
    }
    Rect3D::new(min, max)
}

/// Get the 3D bounding box of multiple polygons (includes Z extents).
pub fn get_polygon_group_bounds_3d(polygons: &[Polygon3D]) -> Rect3D {
    if polygons.is_empty() {
        return Rect3D::default();
    }
    let mut min = Point3D::splat(f64::MAX);
    let mut max = Point3D::splat(f64::MIN);
    let mut has_points = false;
    for poly in polygons {
        for p in poly {
            if p.x < min.x {
                min.x = p.x;
            }
            if p.x > max.x {
                max.x = p.x;
            }
            if p.y < min.y {
                min.y = p.y;
            }
            if p.y > max.y {
                max.y = p.y;
            }
            if p.z < min.z {
                min.z = p.z;
            }
            if p.z > max.z {
                max.z = p.z;
            }
            has_points = true;
        }
    }
    if !has_points {
        return Rect3D::default();
    }
    Rect3D::new(min, max)
}

/// Compute the centroid of a 3D polygon (XY via shoelace, Z as average).
pub fn get_polygon_centroid_3d(polygon: &[Point3D]) -> Point3D {
    if polygon.is_empty() {
        return Point3D::new(0.0, 0.0, 0.0);
    }
    let n = polygon.len();
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut signed_area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let cross = polygon[i].truncate().perp_dot(polygon[j].truncate());
        signed_area += cross;
        cx += (polygon[i].x + polygon[j].x) * cross;
        cy += (polygon[i].y + polygon[j].y) * cross;
    }
    signed_area /= 2.0;
    if signed_area.abs() < 1e-9 {
        let sum_x: f64 = polygon.iter().map(|p| p.x).sum();
        let sum_y: f64 = polygon.iter().map(|p| p.y).sum();
        let sum_z: f64 = polygon.iter().map(|p| p.z).sum();
        return Point3D::new(
            sum_x / n as f64,
            sum_y / n as f64,
            sum_z / n as f64,
        );
    }
    cx /= 6.0 * signed_area;
    cy /= 6.0 * signed_area;
    let avg_z: f64 = polygon.iter().map(|p| p.z).sum::<f64>() / n as f64;
    Point3D::new(cx, cy, avg_z)
}

/// Extract all edges from a 3D polygon as (start, end) point pairs.
pub fn get_polygon_edges_3d(polygon: &[Point3D]) -> Vec<Segment3D> {
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

/// Compute the convex hull of a 3D polygon in the XY plane, preserving Z
/// from the first vertex of the result.
pub fn get_polygon_convex_hull_3d(polygon: &[Point3D]) -> Vec<Point3D> {
    if polygon.len() < 3 {
        return polygon.to_vec();
    }
    let poly_2d: Vec<Point> =
        polygon.iter().map(|p| Point::new(p.x, p.y)).collect();
    let hull_2d = get_polygon_convex_hull_2d(&poly_2d);
    lift_single(hull_2d, first_z(polygon))
}

/// Lift a single 2D polygon to 3D at the given Z.
fn lift_single(poly: Polygon, z: f64) -> Vec<Point3D> {
    poly.into_iter()
        .map(|p| Point3D::new(p.x, p.y, z))
        .collect()
}

// ── 3D Transform functions ────────────────────────────────────────────

/// Translate a 3D polygon by dx, dy, dz.
pub fn translate_polygon_3d(
    polygon: &[Point3D],
    dx: f64,
    dy: f64,
    dz: f64,
) -> Vec<Point3D> {
    polygon
        .iter()
        .map(|p| Point3D::new(p.x + dx, p.y + dy, p.z + dz))
        .collect()
}

/// Translate multiple 3D polygons by dx, dy, dz.
pub fn translate_polygons_3d(
    polygons: &[Polygon3D],
    dx: f64,
    dy: f64,
    dz: f64,
) -> Vec<Polygon3D> {
    polygons
        .iter()
        .map(|p| translate_polygon_3d(p, dx, dy, dz))
        .collect()
}

/// Scale a 3D polygon.
pub fn scale_polygon_3d(
    polygon: &[Point3D],
    sx: f64,
    sy: Option<f64>,
    sz: Option<f64>,
) -> Vec<Point3D> {
    let sy = sy.unwrap_or(sx);
    let sz = sz.unwrap_or(sx);
    polygon
        .iter()
        .map(|p| Point3D::new(p.x * sx, p.y * sy, p.z * sz))
        .collect()
}

/// Flip a 3D polygon horizontally, vertically, and/or along Z.
pub fn flip_polygon_3d(
    polygon: &[Point3D],
    flip_h: bool,
    flip_v: bool,
    flip_z: bool,
) -> Vec<Point3D> {
    polygon
        .iter()
        .map(|p| {
            Point3D::new(
                if flip_h { -p.x } else { p.x },
                if flip_v { -p.y } else { p.y },
                if flip_z { -p.z } else { p.z },
            )
        })
        .collect()
}

/// Flip multiple 3D polygons.
pub fn flip_polygons_3d(
    polygons: &[Polygon3D],
    flip_h: bool,
    flip_v: bool,
    flip_z: bool,
) -> Vec<Polygon3D> {
    polygons
        .iter()
        .map(|p| flip_polygon_3d(p, flip_h, flip_v, flip_z))
        .collect()
}

/// Rotate a 3D polygon around the Z axis (XY rotation, Z preserved).
pub fn rotate_polygon_3d(
    polygon: &[Point3D],
    angle_degrees: f64,
) -> Vec<Point3D> {
    if polygon.is_empty() {
        return polygon.to_vec();
    }
    let angle_rad = angle_degrees.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    polygon
        .iter()
        .map(|p| {
            Point3D::new(
                p.x * cos_a - p.y * sin_a,
                p.x * sin_a + p.y * cos_a,
                p.z,
            )
        })
        .collect()
}

/// Rotate multiple 3D polygons around the Z axis.
pub fn rotate_polygons_3d(
    polygons: &[Polygon3D],
    angle_degrees: f64,
) -> Vec<Polygon3D> {
    polygons
        .iter()
        .map(|p| rotate_polygon_3d(p, angle_degrees))
        .collect()
}

// ── True 3D Offset (edge-plane miter) ────────────────────────────────

/// Return a unit vector perpendicular to `v`.
///
/// Prefers the Z axis as reference to produce an XY-plane perpendicular
/// for all non-Z-aligned vectors.  Falls back to the X axis when `v` is
/// parallel to Z.
fn perpendicular_3d(v: Point3D) -> Point3D {
    // Z × v is the "left" perpendicular in XY (v × Z would be "right").
    let perp = Point3D::Z.cross(v);
    if perp.length_squared() > 1e-12 {
        return perp.normalize();
    }
    // v is parallel to Z → use X as reference
    Point3D::X.cross(v).normalize()
}

/// Offset a 3D polyline by `distance`, maintaining true 3D distance
/// perpendicular to each edge within its local edge plane.
///
/// Unlike [`offset_polygon_3d`] (which projects to XY, offsets in 2D, and
/// lifts back), this function offsets each vertex in the plane defined by
/// its two adjacent edges, giving a *true 3D offset* that works for
/// non-planar polylines.
///
/// **Algorithm** – for each internal vertex the displacement uses the
/// standard 2D miter formula applied in the local edge plane:
///
/// ```text
/// offset = P + distance · (n_in + n_out) / (1 + u_in · u_out)
/// ```
///
/// where `u_in`/`u_out` are unit edge directions and `n_in`/`n_out` are
/// the perpendicular (left) normals within the edge plane.  Endpoints of
/// open polylines are offset perpendicular to their single edge.
///
/// # Parameters
/// - `polyline` – input 3D vertices (distinct — no duplicated end vertex).
/// - `distance` – offset distance (positive = left of traversal direction,
///   negative = right).
/// - `closed` – when `true` the last vertex is connected back to the first,
///   giving every vertex a miter join.  When `false` the first and last
///   vertices are offset perpendicular to their single edge.
///
/// # Returns
/// Offset polyline with the same number of vertices as the input.
///
/// # Notes
/// - For near-hairpin turns (edges almost opposite) the miter is clamped
///   to avoid extreme displacement.
pub fn offset_polyline_3d(
    polyline: &[Point3D],
    distance: f64,
    closed: bool,
) -> Vec<Point3D> {
    if distance == 0.0 {
        return polyline.to_vec();
    }
    let n = polyline.len();
    if n < 2 {
        return polyline.to_vec();
    }

    let mut result = Vec::with_capacity(n);

    for i in 0..n {
        let curr = polyline[i];

        let prev_idx = if closed {
            (i + n - 1) % n
        } else if i > 0 {
            i - 1
        } else {
            // endpoint with no prev → use edge-plane of first two edges
            if n >= 3 {
                let u0 = (polyline[1] - curr).normalize();
                let u1 = (polyline[2] - polyline[1]).normalize();
                let n_plane = u0.cross(u1);
                if n_plane.length_squared() > 1e-12 {
                    let n_hat = n_plane.normalize();
                    let perp = n_hat.cross(u0);
                    result.push(curr + perp * distance);
                    continue;
                }
            }
            let dir = (polyline[1] - curr).normalize();
            let perp = perpendicular_3d(dir);
            result.push(curr + perp * distance);
            continue;
        };

        let next_idx = if closed {
            (i + 1) % n
        } else if i < n - 1 {
            i + 1
        } else {
            // endpoint with no next → use edge-plane of last two edges
            if n >= 3 {
                let u1 = (curr - polyline[prev_idx]).normalize();
                let u0 = (polyline[prev_idx]
                    - polyline[prev_idx.saturating_sub(1)])
                .normalize();
                let n_plane = u0.cross(u1);
                if n_plane.length_squared() > 1e-12 {
                    let n_hat = n_plane.normalize();
                    let perp = n_hat.cross(u1);
                    result.push(curr + perp * distance);
                    continue;
                }
            }
            let dir = (curr - polyline[prev_idx]).normalize();
            let perp = perpendicular_3d(dir);
            result.push(curr + perp * distance);
            continue;
        };

        let e_in = curr - polyline[prev_idx];
        let e_out = polyline[next_idx] - curr;

        let len_in = e_in.length();
        let len_out = e_out.length();

        if len_in < 1e-12 || len_out < 1e-12 {
            let dir = if len_out >= 1e-12 {
                e_out.normalize()
            } else if len_in >= 1e-12 {
                (-e_in).normalize()
            } else {
                result.push(curr);
                continue;
            };
            let perp = perpendicular_3d(dir);
            result.push(curr + perp * distance);
            continue;
        }

        let u_in = e_in / len_in;
        let u_out = e_out / len_out;

        // Edge-plane normal (cross product of the two unit edge directions)
        let n_plane = u_in.cross(u_out);
        let plane_len = n_plane.length();

        if plane_len < 1e-10 {
            // Collinear edges → simple perpendicular offset
            let perp = perpendicular_3d(u_in);
            result.push(curr + perp * distance);
            continue;
        }

        let n_hat = n_plane / plane_len;

        // Left normals in the edge plane
        let n_in = n_hat.cross(u_in);
        let n_out = n_hat.cross(u_out);

        let dot = u_in.dot(u_out).clamp(-1.0, 1.0);
        let denom = 1.0 + dot;

        if denom < 1e-8 {
            // Near-hairpin turn (edges almost opposite).
            // Offset perpendicular to the bisector direction.
            let bisector = (u_in + u_out).normalize();
            let perp = n_hat.cross(bisector);
            result.push(curr + perp * distance);
            continue;
        }

        let offset = distance * (n_in + n_out) / denom;
        result.push(curr + offset);
    }

    result
}

/// Remove consecutive points in a 3D polyline that are within 1e-12 of each other.
pub fn deduplicate_polyline_3d(pts: &mut Vec<Point3D>) {
    pts.dedup_by(|a, b| a.distance_squared(*b) < 1e-12);
}

/// Fillet (round) corners of a 3D polyline with circular arcs of a given radius.
///
/// For each internal vertex, the corner is replaced with a circular arc of
/// `radius` that is tangent to both adjacent edges.  The arc lies in the
/// plane spanned by the two adjacent edges (the *edge plane* of the corner),
/// so this is a **true 3D fillet** that works correctly for non-planar
/// polylines — each corner is rounded in its own local plane.
///
/// If either adjacent segment is too short to accommodate the required tangent
/// offset, the corner is left sharp (the vertex is preserved unchanged).
///
/// # Parameters
/// - `points` — the input polyline (open; first and last points are kept).
/// - `radius` — the fillet radius (must be > 0).
///
/// # Returns
/// A new polyline with filleted corners.  The output has at least as many
/// points as the input (additional points are inserted for each fillet arc).
pub fn fillet_polyline_3d(points: &[Point3D], radius: f64) -> Vec<Point3D> {
    if points.len() < 3 || radius <= 0.0 {
        return points.to_vec();
    }

    let n = points.len();
    let mut result = Vec::with_capacity(n + 16);
    result.push(points[0]);

    for i in 1..n - 1 {
        let prev = points[i - 1];
        let curr = points[i];
        let next = points[i + 1];

        let e_in = prev - curr;
        let e_out = next - curr;

        let len_in = e_in.length();
        let len_out = e_out.length();

        if len_in < 1e-12 || len_out < 1e-12 {
            result.push(curr);
            continue;
        }

        let u_in = e_in / len_in;
        let u_out = e_out / len_out;

        let dot = u_in.dot(u_out).clamp(-1.0, 1.0);
        let angle = dot.acos();

        // Skip near-straight corners (no rounding needed) and near-hairpins
        // (would require a degenerate, near-full-circle arc).
        if (angle - std::f64::consts::PI).abs() < 1e-6 || angle < 1e-6 {
            result.push(curr);
            continue;
        }

        let half_theta = angle / 2.0;
        let tan_off = radius / half_theta.tan();

        if tan_off > len_in || tan_off > len_out {
            // Not enough room on at least one edge — leave the corner sharp.
            result.push(curr);
            continue;
        }

        // Tangent points on each edge (true 3D points along the edges).
        let t_in = curr + u_in * tan_off;
        let t_out = curr + u_out * tan_off;

        // Arc center: on the open-side bisector at distance r/sin(θ/2).
        // The bisector (u_in + u_out) points away from the elbow, into the
        // open side of the angle, which is where the fillet circle lives.
        let bisector = (u_in + u_out).normalize();
        let center_dist = radius / half_theta.sin();
        let center = curr + bisector * center_dist;

        // The arc lies in the edge plane and goes from t_in to t_out around
        // `center`.  Both radius vectors have length `radius` and enclose an
        // angle equal to the turn (π − θ).  We sweep between them with a
        // SLERP-like formula, which traces the short arc on the side of
        // `curr` (i.e. the inside of the elbow) — exactly the fillet we want.
        let v_in = t_in - center;
        let v_out = t_out - center;
        let sweep = std::f64::consts::PI - angle;
        let sin_sweep = sweep.sin();
        let n_arc = ((sweep * 4.0).ceil().max(4.0) as usize).min(64);

        result.push(t_in);

        if sin_sweep.abs() > 1e-10 {
            for j in 1..n_arc {
                let t = j as f64 / n_arc as f64;
                let w_in = ((1.0 - t) * sweep).sin() / sin_sweep;
                let w_out = (t * sweep).sin() / sin_sweep;
                result.push(center + v_in * w_in + v_out * w_out);
            }
        }

        result.push(t_out);
    }

    result.push(points[n - 1]);
    result
}

/// Normalised tangent direction at the last point of a 3D polyline.
pub fn get_polyline_end_tangent_3d(poly: &[Point3D]) -> Point {
    if poly.len() < 2 {
        return Point::new(1.0, 0.0);
    }
    let a = poly[poly.len() - 2];
    let b = poly[poly.len() - 1];
    let d = Point::new(b.x - a.x, b.y - a.y);
    let len = d.length();
    if len < 1e-12 {
        Point::new(1.0, 0.0)
    } else {
        d / len
    }
}
