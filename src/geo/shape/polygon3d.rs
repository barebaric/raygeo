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
    get_polygons_union as get_polygons_union_2d,
    offset_polygon as offset_polygon_2d,
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
    let result = offset_polygon_2d(&projected, offset);
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
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let dz = p2.z - p1.z;
        perimeter += (dx * dx + dy * dy + dz * dz).sqrt();
    }
    perimeter
}

/// Get the 3D bounding box of a polygon (includes Z extents).
pub fn get_polygon_bounds_3d(polygon: &[Point3D]) -> Rect3D {
    if polygon.is_empty() {
        return Rect3D {
            x_min: 0.0,
            x_max: 0.0,
            y_min: 0.0,
            y_max: 0.0,
            z_min: 0.0,
            z_max: 0.0,
        };
    }
    let mut x_min = polygon[0].x;
    let mut x_max = polygon[0].x;
    let mut y_min = polygon[0].y;
    let mut y_max = polygon[0].y;
    let mut z_min = polygon[0].z;
    let mut z_max = polygon[0].z;
    for p in polygon {
        if p.x < x_min {
            x_min = p.x;
        }
        if p.x > x_max {
            x_max = p.x;
        }
        if p.y < y_min {
            y_min = p.y;
        }
        if p.y > y_max {
            y_max = p.y;
        }
        if p.z < z_min {
            z_min = p.z;
        }
        if p.z > z_max {
            z_max = p.z;
        }
    }
    Rect3D {
        x_min,
        x_max,
        y_min,
        y_max,
        z_min,
        z_max,
    }
}

/// Get the 3D bounding box of multiple polygons (includes Z extents).
pub fn get_polygon_group_bounds_3d(polygons: &[Polygon3D]) -> Rect3D {
    if polygons.is_empty() {
        return Rect3D {
            x_min: 0.0,
            x_max: 0.0,
            y_min: 0.0,
            y_max: 0.0,
            z_min: 0.0,
            z_max: 0.0,
        };
    }
    let mut x_min = f64::MAX;
    let mut x_max = f64::MIN;
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    let mut z_min = f64::MAX;
    let mut z_max = f64::MIN;
    let mut has_points = false;
    for poly in polygons {
        for p in poly {
            if p.x < x_min {
                x_min = p.x;
            }
            if p.x > x_max {
                x_max = p.x;
            }
            if p.y < y_min {
                y_min = p.y;
            }
            if p.y > y_max {
                y_max = p.y;
            }
            if p.z < z_min {
                z_min = p.z;
            }
            if p.z > z_max {
                z_max = p.z;
            }
            has_points = true;
        }
    }
    if !has_points {
        return Rect3D {
            x_min: 0.0,
            x_max: 0.0,
            y_min: 0.0,
            y_max: 0.0,
            z_min: 0.0,
            z_max: 0.0,
        };
    }
    Rect3D {
        x_min,
        x_max,
        y_min,
        y_max,
        z_min,
        z_max,
    }
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
        let cross = polygon[i].x * polygon[j].y - polygon[j].x * polygon[i].y;
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
