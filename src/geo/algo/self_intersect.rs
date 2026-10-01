//! Self-intersection resolution for geometry.
//!
//! Glyph outlines from some fonts (and traced vector art) contain
//! self-intersecting or mutually overlapping contours.  Such input
//! renders correctly with fill rules, but breaks downstream winding
//! analysis, containment hierarchy construction, and polygon
//! offsetting — e.g. the contour assembler can emit incomplete tool
//! paths for them.
//!
//! [`resolve_self_intersections`] detects affected closed contours and
//! rebuilds them as the Clipper2 union of their linearised outlines.
//! Unaffected contours — including their arcs and Béziers — pass
//! through untouched, as do open contours.

use crate::geo::algo::intersect::{
    check_intersection_from_array, check_self_intersection_from_array,
};
use crate::geo::algo::topology::split_into_contours;
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::{paths_to_polygons, polygons_to_paths};
use crate::geo::types::{Point3D, Polygon};
use clipper2::{union as clipper_union, FillRule};

/// Union polygons with the NonZero fill rule.
///
/// Unlike [`crate::geo::shape::polygon::get_polygons_union`] this
/// always runs the clipper pass, so a lone self-intersecting polygon
/// is resolved as well.
fn union_polygons_nonzero(polygons: &[Polygon]) -> Vec<Polygon> {
    let paths = polygons_to_paths(polygons);
    if paths.is_empty() {
        return vec![];
    }
    let result = clipper_union(paths.clone(), paths, FillRule::NonZero)
        .unwrap_or_default();
    paths_to_polygons(&result)
        .into_iter()
        .filter(|p| p.len() >= 3)
        .collect()
}

/// Rebuild self-intersecting / overlapping closed contours via union.
///
/// Contours are considered affected when they intersect themselves or
/// another closed contour.  All affected contours are linearised at
/// `linear_tolerance` (mm chord error) and replaced by the polygons
/// resulting from their NonZero-rule union, which merges overlaps and
/// resolves crossings while preserving properly wound holes.  If the
/// union comes back empty the original contours are kept as a
/// fallback.
pub fn resolve_self_intersections(
    geometry: &Geometry,
    linear_tolerance: f64,
) -> Geometry {
    if geometry.is_empty() {
        return geometry.copy();
    }
    let contours = split_into_contours(geometry);
    if contours.is_empty() {
        return geometry.copy();
    }

    let closed: Vec<usize> = contours
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_closed(1e-6))
        .map(|(i, _)| i)
        .collect();

    let mut flagged = vec![false; contours.len()];
    for &i in &closed {
        if check_self_intersection_from_array(&contours[i].data, false) {
            flagged[i] = true;
        }
    }
    for (a, &i) in closed.iter().enumerate() {
        for &j in closed.iter().skip(a + 1) {
            if flagged[i] && flagged[j] {
                continue;
            }
            if check_intersection_from_array(
                &contours[i].data,
                &contours[j].data,
                false,
            ) {
                flagged[i] = true;
                flagged[j] = true;
            }
        }
    }
    if !flagged.iter().any(|&f| f) {
        return geometry.copy();
    }

    let mut polygons: Vec<Polygon> = Vec::new();
    let z = contours
        .iter()
        .zip(flagged.iter())
        .find(|(_, &f)| f)
        .and_then(|(c, _)| c.data.first())
        .map(|cmd| cmd.end_point().z)
        .unwrap_or(0.0);
    for (i, c) in contours.iter().enumerate() {
        if flagged[i] {
            polygons.extend(c.to_polygons(linear_tolerance));
        }
    }

    let mut resolved = Geometry::new();
    for poly in union_polygons_nonzero(&polygons) {
        let pts: Vec<Point3D> =
            poly.iter().map(|p| Point3D::new(p.x, p.y, z)).collect();
        let contour = Geometry::from_points(&pts, true);
        resolved.extend(&contour);
    }

    let mut result = Geometry::new();
    if resolved.is_empty() {
        for c in &contours {
            result.extend(c);
        }
        return result;
    }
    let mut inserted = false;
    for (i, c) in contours.iter().enumerate() {
        if flagged[i] {
            if !inserted {
                result.extend(&resolved);
                inserted = true;
            }
        } else {
            result.extend(c);
        }
    }
    result
}
