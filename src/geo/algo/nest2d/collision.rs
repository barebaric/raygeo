use crate::geo::shape::polygon::{
    do_polygons_intersect, get_polygon_bounds, get_polygon_group_bounds,
    get_polygons_difference,
};
use crate::geo::types::{Polygon, Rect};

use crate::geo::algo::spatial_grid2d::SpatialGrid;

/// Check whether `inner` polygons are fully contained within `outer`.
///
/// Uses a fast bounding-box reject followed by Clipper difference.
pub fn is_contained(inner: &[Polygon], outer: &Polygon) -> bool {
    if inner.is_empty() || outer.len() < 3 {
        return false;
    }
    let inner_bounds = get_polygon_group_bounds(inner);
    let outer_bounds = get_polygon_bounds(outer);
    if inner_bounds.min.x < outer_bounds.min.x - 1e-6
        || inner_bounds.min.y < outer_bounds.min.y - 1e-6
        || inner_bounds.max.x > outer_bounds.max.x + 1e-6
        || inner_bounds.max.y > outer_bounds.max.y + 1e-6
    {
        return false;
    }
    for poly in inner {
        if poly.len() < 3 {
            continue;
        }
        let diff = get_polygons_difference(poly, outer);
        if !diff.is_empty() {
            return false;
        }
    }
    true
}

/// Check if `candidate` polygon overlaps any polygon in `placed`.
///
/// Uses bounding-box pre-filter before the full Clipper intersection test.
pub fn does_any_overlap(
    candidate: &Polygon,
    placed: &[Polygon],
    min_area: f64,
) -> bool {
    if candidate.len() < 3 {
        return false;
    }
    let c_bounds = get_polygon_bounds(candidate);
    for p in placed {
        if p.len() < 3 {
            continue;
        }
        let p_bounds = get_polygon_bounds(p);
        if c_bounds.min.x > p_bounds.max.x
            || c_bounds.max.x < p_bounds.min.x
            || c_bounds.min.y > p_bounds.max.y
            || c_bounds.max.y < p_bounds.min.y
        {
            continue;
        }
        if do_polygons_intersect(candidate, p, min_area) {
            return true;
        }
    }
    false
}

/// Overlap check accelerated by a [`SpatialGrid`] for large numbers of placed
/// parts.
pub fn does_any_overlap_with_grid(
    candidate: &Polygon,
    placed: &[Polygon],
    grid: &SpatialGrid,
    min_area: f64,
) -> bool {
    if candidate.len() < 3 {
        return false;
    }
    let c_bounds = get_polygon_bounds(candidate);
    let indices = grid.query(c_bounds);
    for &idx in &indices {
        if idx >= placed.len() {
            continue;
        }
        let p = &placed[idx];
        if p.len() < 3 {
            continue;
        }
        let p_bounds = get_polygon_bounds(p);
        if c_bounds.min.x > p_bounds.max.x
            || c_bounds.max.x < p_bounds.min.x
            || c_bounds.min.y > p_bounds.max.y
            || c_bounds.max.y < p_bounds.min.y
        {
            continue;
        }
        if do_polygons_intersect(candidate, p, min_area) {
            return true;
        }
    }
    false
}

/// Hierarchical overlap check: bounding box → convex hull → detailed polygon.
///
/// Uses a 3-tier filter:
/// 1. Bounding box reject (fastest)
/// 2. Convex hull intersection pre-check (medium)
/// 3. Detailed polygon intersection (slowest, but only when hulls overlap)
///
/// This avoids expensive concave-polygon intersection when convex hulls
/// don't touch.
pub fn does_any_overlap_hierarchical(
    candidate_polys: &[Polygon],
    candidate_hulls: &[Polygon],
    placed_polys_groups: &[Vec<Polygon>],
    placed_hulls_groups: &[Vec<Polygon>],
    min_area: f64,
) -> bool {
    if candidate_polys.is_empty() {
        return false;
    }
    let cand_bbox = get_polygon_group_bounds(candidate_polys);

    for (idx, placed_polys) in placed_polys_groups.iter().enumerate() {
        if placed_polys.is_empty() {
            continue;
        }
        let placed_bbox = get_polygon_group_bounds(placed_polys);

        // 1. Bounding box reject
        if cand_bbox.min.x > placed_bbox.max.x
            || cand_bbox.max.x < placed_bbox.min.x
            || cand_bbox.min.y > placed_bbox.max.y
            || cand_bbox.max.y < placed_bbox.min.y
        {
            continue;
        }

        // 2. Convex hull pre-check
        if !candidate_hulls.is_empty() && !placed_hulls_groups.is_empty() {
            let placed_hulls = &placed_hulls_groups[idx];
            let mut hulls_intersect = false;
            'hull: for cand_hull in candidate_hulls {
                for placed_hull in placed_hulls {
                    if do_polygons_intersect(cand_hull, placed_hull, 0.0) {
                        hulls_intersect = true;
                        break 'hull;
                    }
                }
            }
            if !hulls_intersect {
                continue;
            }
        }

        // 3. Detailed polygon check
        for cand_poly in candidate_polys {
            for placed_poly in placed_polys {
                if do_polygons_intersect(cand_poly, placed_poly, min_area) {
                    return true;
                }
            }
        }
    }
    false
}

/// Hierarchical overlap check accelerated by a [`SpatialGrid`].
///
/// Same 3-tier logic as [`does_any_overlap_hierarchical`] but limits candidate
/// pairs to those in nearby grid cells.
pub fn does_any_overlap_hierarchical_grid(
    candidate_polys: &[Polygon],
    candidate_hulls: &[Polygon],
    placed_polys_groups: &[Vec<Polygon>],
    placed_hulls_groups: &[Vec<Polygon>],
    grid: &SpatialGrid,
    candidate_bbox: Rect,
    min_area: f64,
) -> bool {
    if candidate_polys.is_empty() {
        return false;
    }
    let cand_bbox = if candidate_bbox.min.x.is_finite() {
        candidate_bbox
    } else {
        get_polygon_group_bounds(candidate_polys)
    };

    let indices = grid.query(cand_bbox);

    for &idx in &indices {
        if idx >= placed_polys_groups.len() {
            continue;
        }
        let placed_polys = &placed_polys_groups[idx];
        if placed_polys.is_empty() {
            continue;
        }
        let placed_bbox = get_polygon_group_bounds(placed_polys);

        // 1. Bounding box reject
        if cand_bbox.min.x > placed_bbox.max.x
            || cand_bbox.max.x < placed_bbox.min.x
            || cand_bbox.min.y > placed_bbox.max.y
            || cand_bbox.max.y < placed_bbox.min.y
        {
            continue;
        }

        // 2. Convex hull pre-check
        if !candidate_hulls.is_empty() && !placed_hulls_groups.is_empty() {
            let placed_hulls = &placed_hulls_groups[idx];
            let mut hulls_intersect = false;
            'hull: for cand_hull in candidate_hulls {
                for placed_hull in placed_hulls {
                    if do_polygons_intersect(cand_hull, placed_hull, 0.0) {
                        hulls_intersect = true;
                        break 'hull;
                    }
                }
            }
            if !hulls_intersect {
                continue;
            }
        }

        // 3. Detailed polygon check
        for cand_poly in candidate_polys {
            for placed_poly in placed_polys {
                if do_polygons_intersect(cand_poly, placed_poly, min_area) {
                    return true;
                }
            }
        }
    }
    false
}
