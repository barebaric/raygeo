//! Offset: Polygon offsetting (growing/shrinking) for geometry data.
//!
//! **Planar (XY-plane) with Z passthrough.** The core inflate uses Clipper2
//! (strictly 2D).  When offsetting a `Geometry` the source contour's Z
//! (taken from its first vertex) is preserved on the output — the operation
//! is a 2D inflate of the XY outline at the source Z plane.
//!
//! For off-axis offsetting (e.g. on an arbitrary work plane),
//! use [`grow_geometry_on_plane`] which rotates the geometry so the target
//! plane aligns with XY before offsetting.
//!
//! Handles containment hierarchies (holes within solids) correctly by
//! offsetting solids and holes independently and subtracting holes from solids.

use glam::{DMat4, DQuat, DVec3};

use crate::geo::algo::intersect::check_intersection_from_array;
use crate::geo::algo::polylabel::find_largest_circle;
use crate::geo::algo::topology::{
    build_hierarchy, get_valid_contours_data, group_solids_and_holes,
    split_into_contours,
};
use crate::geo::geometry::Geometry;
use crate::geo::shape::polygon::{
    get_polygon_area, get_polygon_centroid, get_polygons_group_difference,
    is_point_in_polygon, offset_polygon_with_style, JoinStyle,
};
use crate::types::{Point, Point3D, Polygon};

#[derive(Clone, Debug)]
struct ContourItem {
    path: Polygon,
    #[allow(dead_code)]
    area: f64,
    id: usize,
    /// The Z height of this contour (from its first vertex).
    /// Preserved from the source 3D geometry so offset output keeps Z.
    z: f64,
}

fn prepare_contour_items(
    contour_data: &[(&Geometry, Vec<Point>, bool)],
) -> Vec<ContourItem> {
    let mut items = Vec::new();
    for (i, (geo, vertices, _is_closed)) in contour_data.iter().enumerate() {
        if vertices.len() < 2 {
            continue;
        }
        let mut verts = vertices.clone();
        let first = verts[0];
        let last = verts[verts.len() - 1];
        if (first.x - last.x).abs() < 1e-9 && (first.y - last.y).abs() < 1e-9 {
            verts.pop();
        }
        if verts.len() < 3 {
            continue;
        }
        let mut area = 0.0;
        let n = verts.len();
        for j in 0..n {
            let k = (j + 1) % n;
            area += verts[j].perp_dot(verts[k]);
        }
        area = area.abs() / 2.0;
        // Preserve Z from the source geometry's first point (Move command).
        let z = geo.data.first().map(|cmd| cmd.end_point().z).unwrap_or(0.0);
        items.push(ContourItem {
            path: verts,
            area,
            id: i,
            z,
        });
    }
    items
}

pub fn offset_contour_group(
    solid_path: &Polygon,
    hole_paths: &[Polygon],
    offset: f64,
    join_style: JoinStyle,
) -> Vec<Polygon> {
    if solid_path.len() < 3 {
        return vec![];
    }
    if hole_paths.is_empty() {
        return offset_polygon_with_style(solid_path, offset, join_style);
    }
    // Offset solid and holes separately, then subtract all holes from the
    // solid in a single Clipper2 difference operation.
    //
    // For positive offset (grow): solid expands outward, hole contracts
    // (inward).  For negative offset (shrink): solid contracts, hole
    // expands.  The hole offset direction is always opposite to the solid
    // offset.
    //
    // Subtracting all holes at once (rather than one at a time) lets
    // Clipper2 assign consistent orientations to every resulting contour:
    // outer boundaries CCW, holes CW.  Sequential subtraction corrupts the
    // orientation of earlier holes when later holes are processed, because
    // each intermediate polygon is treated as a standalone subject.
    let offset_solids =
        offset_polygon_with_style(solid_path, offset, join_style);
    let offset_holes: Vec<Polygon> = hole_paths
        .iter()
        .flat_map(|hole| offset_polygon_with_style(hole, -offset, join_style))
        .collect();
    if offset_holes.is_empty() {
        return offset_solids;
    }
    get_polygons_group_difference(&offset_solids, &offset_holes)
}

/// Offsets the closed contours of a Geometry object by a given amount.
///
/// This function grows (positive offset) or shrinks (negative offset) the
/// area enclosed by closed paths.
///
/// This implementation processes logically distinct contours
/// independently. Holes are associated with their enclosing solids and
/// offset together. Adjacent or overlapping solids remain separate, preserving
/// distinct outlines.
pub fn grow_geometry(geometry: &Geometry, offset: f64) -> Geometry {
    let raw_contours = split_into_contours(geometry);
    if raw_contours.is_empty() {
        return Geometry::new();
    }
    let contour_data = get_valid_contours_data(&raw_contours);
    if contour_data.is_empty() {
        return Geometry::new();
    }
    let closed_items = prepare_contour_items(&contour_data);
    if closed_items.is_empty() {
        return Geometry::new();
    }

    let hierarchy_geoms: Vec<&Geometry> =
        contour_data.iter().map(|(g, _, _)| *g).collect();
    let mut hierarchy = build_hierarchy(&hierarchy_geoms);
    let hierarchy_info = hierarchy.info.clone();
    hierarchy.filter_parents(&hierarchy_info, |child, parent| {
        !check_intersection_from_array(
            &hierarchy_geoms[child].data,
            &hierarchy_geoms[parent].data,
            false,
        )
    });
    let solid_groups = group_solids_and_holes(&hierarchy);

    let contour_to_item: std::collections::HashMap<usize, usize> = closed_items
        .iter()
        .enumerate()
        .map(|(item_idx, item)| (item.id, item_idx))
        .collect();

    let mut new_geo = Geometry::new();
    for (solid_contour_idx, hole_contour_indices) in solid_groups.iter() {
        let solid_item_idx = match contour_to_item.get(solid_contour_idx) {
            Some(&idx) => idx,
            None => continue,
        };
        let solid_item = &closed_items[solid_item_idx];
        let hole_paths: Vec<Polygon> = hole_contour_indices
            .iter()
            .filter_map(|&h_idx| {
                contour_to_item
                    .get(&h_idx)
                    .map(|&idx| closed_items[idx].path.clone())
            })
            .collect();
        let offset_contours = offset_contour_group(
            &solid_item.path,
            &hole_paths,
            offset,
            JoinStyle::Miter,
        );
        for new_vertices in offset_contours {
            let z = solid_item.z;
            let points: Vec<Point3D> = new_vertices
                .iter()
                .map(|p| Point3D::new(p.x, p.y, z))
                .collect();
            let new_contour_geo = Geometry::from_points(&points, true);
            if !new_contour_geo.is_empty() {
                new_geo.extend(&new_contour_geo);
            }
        }
    }
    new_geo
}

/// Offsets geometry on an arbitrary plane defined by a normal vector.
///
/// The 3D geometry is rotated so that `plane_normal` aligns with the +Z
/// axis, a 2D offset is performed in that plane, and the result is rotated
/// back to the original coordinate frame.
///
/// This enables offsetting contours that lie on non-XY planes — useful
/// when features sit on angled faces.
///
/// # Panics
///
/// Panics if `plane_normal` is a zero vector.
/// Generate up to `max_passes` concentric inward offsets of `geom`,
/// spaced `step` apart (in current units). Stops early when an offset
/// collapses (enclosed area drops below `min_area`). Returns offsets
/// outermost-first. Each returned Geometry preserves the source Z.
///
/// The first offset shrinks the input boundary by `step`; each subsequent
/// offset shrinks the previous result by the same amount.
pub fn concentric_offsets(
    geom: &Geometry,
    step: f64,
    max_passes: usize,
    min_area: f64,
) -> Vec<Geometry> {
    if max_passes == 0 || step <= 0.0 {
        return vec![];
    }
    let mut result = Vec::new();
    let mut current = geom.copy();
    for _ in 0..max_passes {
        let next = grow_geometry(&current, -step);
        if next.is_empty() || next.area() < min_area {
            break;
        }
        result.push(next.clone());
        current = next;
    }
    result
}

pub fn grow_geometry_on_plane(
    geometry: &Geometry,
    offset: f64,
    plane_normal: Point3D,
) -> Geometry {
    let normal = plane_normal.normalize();
    assert!(
        normal.length_squared() > 0.0,
        "grow_geometry_on_plane: plane_normal must be non-zero"
    );

    // Rotation that maps plane_normal → (0,0,1).
    let quat = DQuat::from_rotation_arc(normal, DVec3::Z);
    let rot = DMat4::from_quat(quat);
    let inv = DMat4::from_quat(quat.inverse());

    let mut rotated = geometry.copy();
    rotated.transform(&rot);

    let offset_result = grow_geometry(&rotated, offset);

    let mut result = offset_result;
    result.transform(&inv);
    result
}

/// Find the single most interior point of a compound polygon region.
///
/// Returns the point whose minimum distance to any boundary (outer
/// contour *or* hole) is greatest — the *pole of inaccessibility*.
///
/// Holes are detected by **spatial containment**: any input polygon
/// whose first vertex lies inside another is treated as a hole of that
/// polygon.  This is more robust than winding-based detection and works
/// correctly even when polygon orientations are inconsistent.  When
/// holes are present the core is found via
/// [`find_largest_circle`], which is guaranteed to lie inside the
/// filled area — never inside a hole.
///
/// For inputs without holes (a flat list of disconnected regions) the
/// legacy binary-search path is used: it returns the centroid of the
/// last surviving inward-offset fragment of the largest region.
///
/// Returns an empty vec when the input is empty or `step_over` ≤ 0.
///
/// **Planar (XY-plane only).** Z is not modeled.
pub fn find_deepest_cores(
    valid_tool_area: &[Polygon],
    step_over: f64,
) -> Vec<Point> {
    if valid_tool_area.is_empty() || step_over <= 0.0 {
        return vec![];
    }

    // Classify input polygons as solids or holes using spatial
    // containment.  A polygon whose first vertex lies inside another
    // input polygon is a hole; the rest are solids.  We pick the
    // *smallest* containing polygon (by area) as the parent so that
    // nested contours associate with their immediate enclosing contour.
    let n = valid_tool_area.len();
    let mut parent: Vec<Option<usize>> = vec![None; n];
    for (i, poly_i) in valid_tool_area.iter().enumerate() {
        if poly_i.len() < 3 {
            continue;
        }
        let probe = poly_i[0];
        let mut best: Option<usize> = None;
        let mut best_area = f64::MAX;
        for (j, poly_j) in valid_tool_area.iter().enumerate() {
            if i == j || poly_j.len() < 3 {
                continue;
            }
            if is_point_in_polygon(probe, poly_j) {
                let area = get_polygon_area(poly_j);
                if area < best_area {
                    best_area = area;
                    best = Some(j);
                }
            }
        }
        parent[i] = best;
    }

    let solid_indices: Vec<usize> = (0..n)
        .filter(|&i| valid_tool_area[i].len() >= 3 && parent[i].is_none())
        .collect();
    let hole_indices: Vec<usize> = (0..n)
        .filter(|&i| valid_tool_area[i].len() >= 3 && parent[i].is_some())
        .collect();

    // No holes (or no recognised solids): use the legacy binary-search
    // path.  It returns the centroid of the last surviving inward-offset
    // fragment, which matches existing behaviour for simple regions and
    // for callers that pass a flat list of disconnected regions.
    if hole_indices.is_empty() || solid_indices.is_empty() {
        return find_deepest_cores_by_offset(valid_tool_area);
    }

    // Holes present: find the pole of inaccessibility per solid so the
    // returned core always lies in the filled area, even for annular
    // regions with central holes where a centroid-based approach would
    // land inside the hole.
    let mut best_point: Option<Point> = None;
    let mut best_radius = -1.0_f64;

    for &solid_idx in &solid_indices {
        let solid = &valid_tool_area[solid_idx];

        // Collect the holes that belong to this solid: any hole whose
        // first vertex lies inside the solid.
        let associated_holes: Vec<Polygon> = hole_indices
            .iter()
            .filter_map(|&h_idx| {
                let hole = &valid_tool_area[h_idx];
                hole.first()
                    .filter(|&&p| is_point_in_polygon(p, solid))
                    .map(|_| hole.clone())
            })
            .collect();

        if let Some((centre, radius)) =
            find_largest_circle(solid, &associated_holes, 0.5)
        {
            if radius > best_radius {
                best_radius = radius;
                best_point = Some(centre);
            }
        }
    }

    best_point.map(|p| vec![p]).unwrap_or_default()
}

/// Legacy core finder: binary-searches the largest inward offset that
/// keeps each polygon alive and returns the centroid of the single
/// largest surviving fragment across all inputs.
///
/// Used by [`find_deepest_cores`] for inputs without holes.
fn find_deepest_cores_by_offset(valid_tool_area: &[Polygon]) -> Vec<Point> {
    let mut best_fragment: Option<Polygon> = None;
    let mut best_area = 0.0_f64;

    for poly in valid_tool_area {
        if poly.len() < 3 {
            continue;
        }

        // Bounding-box of the starting polygon.
        let (mut x_min, mut x_max) = (f64::MAX, f64::MIN);
        let (mut y_min, mut y_max) = (f64::MAX, f64::MIN);
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
        }
        let w = x_max - x_min;
        let h = y_max - y_min;
        if w <= 0.0 || h <= 0.0 {
            continue;
        }

        let half_min = w.min(h) * 0.5;
        let mut low = 0.0;
        let mut high = half_min;
        let mut best: Vec<Polygon> = vec![poly.clone()];
        let tol = 1e-4;

        // Binary search: at the end, `low` is the largest offset that
        // still yields a non-empty polygon.
        while high - low > tol {
            let mid = (low + high) * 0.5;
            let result =
                offset_polygon_with_style(poly, -mid, JoinStyle::Miter);
            let valid: Vec<Polygon> = result
                .into_iter()
                .filter(|p| p.len() >= 3 && get_polygon_area(p) > 1e-9)
                .collect();

            if valid.is_empty() {
                high = mid; // mid was too large
            } else {
                low = mid;
                best = valid;
            }
        }

        // Keep the single largest fragment across all polygons.
        for p in best {
            let a = get_polygon_area(&p);
            if a > best_area {
                best_area = a;
                best_fragment = Some(p);
            }
        }
    }

    best_fragment
        .map(|p| vec![get_polygon_centroid(&p)])
        .unwrap_or_default()
}

/// Compute the region swept by a circle of given radius whose centre
/// moves inside a boundary while avoiding obstacles.
///
/// The boundary is inset (shrunk) by `radius`, and each obstacle is
/// expanded (grown) by `radius` and subtracted from the inset
/// boundary.  Returns `(region_polygons, total_area)`.
pub fn compute_inset_region(
    boundary: &Polygon,
    radius: f64,
    obstacles: &[Polygon],
) -> (Vec<Polygon>, f64) {
    let mut region =
        offset_polygon_with_style(boundary, -radius, JoinStyle::Miter);
    if !region.is_empty() && !obstacles.is_empty() {
        let obstacle_bufs: Vec<Polygon> = obstacles
            .iter()
            .flat_map(|obs| {
                offset_polygon_with_style(obs, radius, JoinStyle::Miter)
            })
            .collect();
        if !obstacle_bufs.is_empty() {
            region = get_polygons_group_difference(&region, &obstacle_bufs);
        }
    }
    let total_area: f64 = region.iter().map(get_polygon_area).sum();
    (region, total_area)
}
