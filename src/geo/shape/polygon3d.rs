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

use crate::types::{Point, Point3D, Polygon, Polygon3D};

use super::polygon::{
    get_polygons_difference, get_polygons_group_difference,
    get_polygons_group_intersection, get_polygons_intersection,
    get_polygons_union as get_polygons_union_2d,
    offset_polygon as offset_polygon_2d,
};

// ── internal helpers ──────────────────────────────────────────────────

fn first_z(poly: &Polygon3D) -> f64 {
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
    let z = polygons.first().map(first_z).unwrap_or(0.0);
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
