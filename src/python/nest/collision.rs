use numpy::{PyArray2, PyArrayMethods};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use super::super::geo::flex_point::{poly_to_points, PyPoint2D};
use super::spatial_grid::SpatialGrid as PySpatialGrid;
use crate::geo::shape::polygon::{
    get_polygon_group_bounds, polygons_intersect,
};
use crate::nest::collision;
use crate::types::Polygon;

pyo3_stub_gen::module_doc!("raygeo.nest.collision", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Collision detection for nesting algorithms.

Provides overlap checks with bounding-box, convex hull, and detailed
polygon intersection, including hierarchical variants for performance.
";

/// Convert a 2D numpy array (N×2 float64) to a `Polygon` (Vec<(f64, f64)>)
/// using direct buffer access (avoids PyO3 element-by-element extraction).
fn polygon_from_numpy(arr: &Bound<'_, PyArray2<f64>>) -> Polygon {
    let readonly = arr.readonly();
    let view = readonly.as_array();
    view.rows()
        .into_iter()
        .map(|row| (row[0], row[1]))
        .collect()
}

/// Compute the bounding box of a numpy array (N×2 float64) WITHOUT
/// converting it to a Polygon first. Returns (xmin, ymin, xmax, ymax).
fn numpy_bbox(arr: &Bound<'_, PyArray2<f64>>) -> (f64, f64, f64, f64) {
    let readonly = arr.readonly();
    let view = readonly.as_array();
    let (mut xmin, mut ymin) = (f64::MAX, f64::MAX);
    let (mut xmax, mut ymax) = (f64::MIN, f64::MIN);
    for row in view.rows() {
        let x = row[0];
        let y = row[1];
        if x < xmin {
            xmin = x;
        }
        if y < ymin {
            ymin = y;
        }
        if x > xmax {
            xmax = x;
        }
        if y > ymax {
            ymax = y;
        }
    }
    (xmin, ymin, xmax, ymax)
}

/// Compute the group bounding box from an array of numpy polygons, also
/// without converting to Polygon.
fn numpy_group_bbox(
    polys: &[Bound<'_, PyArray2<f64>>],
) -> (f64, f64, f64, f64) {
    let (mut xmin, mut ymin) = (f64::MAX, f64::MAX);
    let (mut xmax, mut ymax) = (f64::MIN, f64::MIN);
    for p in polys {
        let (pxmin, pylow, pxmax, pyhigh) = numpy_bbox(p);
        if pxmin < xmin {
            xmin = pxmin;
        }
        if pylow < ymin {
            ymin = pylow;
        }
        if pxmax > xmax {
            xmax = pxmax;
        }
        if pyhigh > ymax {
            ymax = pyhigh;
        }
    }
    (xmin, ymin, xmax, ymax)
}

fn rects_intersect(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 <= b.2 && a.2 >= b.0 && a.1 <= b.3 && a.3 >= b.1
}

// ---------------------------------------------------------------------------
// is_contained
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def is_contained(
        inner: collections.abc.Sequence[types.Polygon],
        outer: types.Polygon,
    ) -> bool:
        """Check if inner polygons are fully contained within outer polygon.

        :param inner: List of polygons to check.
        :param outer: Outer polygon.
        :returns: True if all inner polygons are inside outer.
        """
"#,
    module = "raygeo.nest.collision"
)]
#[pyfunction(name = "is_contained")]
fn is_contained_py(inner: Vec<Vec<PyPoint2D>>, outer: Vec<PyPoint2D>) -> bool {
    let inner_polys: Vec<Polygon> =
        inner.into_iter().map(poly_to_points).collect();
    let outer_poly = poly_to_points(outer);
    collision::is_contained(&inner_polys, &outer_poly)
}

// ---------------------------------------------------------------------------
// any_overlap
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def any_overlap(
        candidate: types.Polygon,
        placed: collections.abc.Sequence[types.Polygon],
        min_area: float = 1.0,
    ) -> bool:
        """Check if a candidate polygon overlaps any placed polygon.

        :param candidate: Candidate polygon.
        :param placed: List of already-placed polygons.
        :param min_area: Minimum overlap area to consider (in clipper coords).
        :returns: True if any overlap detected.
        """
"#,
    module = "raygeo.nest.collision"
)]
#[pyfunction(name = "any_overlap")]
fn any_overlap_py(
    candidate: Vec<PyPoint2D>,
    placed: Vec<Vec<PyPoint2D>>,
    min_area: f64,
) -> bool {
    let cand_poly = poly_to_points(candidate);
    let placed_polys: Vec<Polygon> =
        placed.into_iter().map(poly_to_points).collect();
    collision::any_overlap(&cand_poly, &placed_polys, min_area)
}

// ---------------------------------------------------------------------------
// any_overlap_hierarchical
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import numpy

    def any_overlap_hierarchical(
        candidate_polys: collections.abc.Sequence[numpy.ndarray],
        candidate_hulls: collections.abc.Sequence[numpy.ndarray],
        placed_polys_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]],
        placed_hulls_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]],
        min_area: float = 1.0,
    ) -> bool:
        """Hierarchical overlap: bbox -> hull -> detailed polygon.

        :param candidate_polys: Candidate polygons to check.
        :param candidate_hulls: Convex hulls of candidate polygons.
        :param placed_polys_groups: Groups of already-placed polygons.
        :param placed_hulls_groups: Convex hulls of placed groups.
        :param min_area: Minimum overlap area (clipper coords).
        :returns: True if any overlap detected.
        """
"#,
    module = "raygeo.nest.collision"
)]
#[pyfunction(name = "any_overlap_hierarchical")]
fn any_overlap_hierarchical_py(
    candidate_polys: Vec<Bound<'_, PyArray2<f64>>>,
    candidate_hulls: Vec<Bound<'_, PyArray2<f64>>>,
    placed_polys_groups: Vec<Vec<Bound<'_, PyArray2<f64>>>>,
    placed_hulls_groups: Vec<Vec<Bound<'_, PyArray2<f64>>>>,
    min_area: f64,
) -> bool {
    // Convert candidates upfront (always small)
    let cand_polys: Vec<Polygon> = candidate_polys
        .into_iter()
        .map(|a| polygon_from_numpy(&a))
        .collect();
    let cand_hulls: Vec<Polygon> = candidate_hulls
        .into_iter()
        .map(|a| polygon_from_numpy(&a))
        .collect();

    if cand_polys.is_empty() {
        return false;
    }
    let cand_bbox = get_polygon_group_bounds(&cand_polys);

    for (idx, placed_group) in placed_polys_groups.iter().enumerate() {
        if placed_group.is_empty() {
            continue;
        }

        // 1. BBOX check using numpy (no allocation)
        let placed_bbox = numpy_group_bbox(placed_group);
        if !rects_intersect(cand_bbox, placed_bbox) {
            continue;
        }

        // 2. Hull pre-check (convert on-demand)
        if !cand_hulls.is_empty() && !placed_hulls_groups.is_empty() {
            let placed_hulls = &placed_hulls_groups[idx];
            let mut hulls_meet = false;
            'hull: for cand_hull in &cand_hulls {
                for placed_hull_arr in placed_hulls {
                    let placed_hull = polygon_from_numpy(placed_hull_arr);
                    if polygons_intersect(cand_hull, &placed_hull, 0.0) {
                        hulls_meet = true;
                        break 'hull;
                    }
                }
            }
            if !hulls_meet {
                continue;
            }
        }

        // 3. Detail check (convert placed polys on-demand)
        for cand_poly in &cand_polys {
            for placed_poly_arr in placed_group {
                let placed_poly = polygon_from_numpy(placed_poly_arr);
                if polygons_intersect(cand_poly, &placed_poly, min_area) {
                    return true;
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// any_overlap_hierarchical_grid
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import numpy
    import raygeo.nest.spatial_grid

    def any_overlap_hierarchical_grid(
        candidate_polys: collections.abc.Sequence[numpy.ndarray],
        candidate_hulls: collections.abc.Sequence[numpy.ndarray],
        placed_polys_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]],
        placed_hulls_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]],
        spatial_grid: spatial_grid.SpatialGrid,
        candidate_bbox: tuple[float, float, float, float],
        min_area: float = 1.0,
    ) -> bool:
        """:param spatial_grid: SpatialGrid for fast neighbor lookup.
        """
"#,
    module = "raygeo.nest.collision"
)]
#[pyfunction(name = "any_overlap_hierarchical_grid")]
fn any_overlap_hierarchical_grid_py(
    candidate_polys: Vec<Bound<'_, PyArray2<f64>>>,
    candidate_hulls: Vec<Bound<'_, PyArray2<f64>>>,
    placed_polys_groups: Vec<Vec<Bound<'_, PyArray2<f64>>>>,
    placed_hulls_groups: Vec<Vec<Bound<'_, PyArray2<f64>>>>,
    spatial_grid: &Bound<'_, PySpatialGrid>,
    candidate_bbox: (f64, f64, f64, f64),
    min_area: f64,
) -> bool {
    // Convert candidates upfront (always small: 1-4 polygons)
    let cand_polys: Vec<Polygon> = candidate_polys
        .into_iter()
        .map(|a| polygon_from_numpy(&a))
        .collect();
    let cand_hulls: Vec<Polygon> = candidate_hulls
        .into_iter()
        .map(|a| polygon_from_numpy(&a))
        .collect();

    if cand_polys.is_empty() {
        return false;
    }
    let cand_bbox = if candidate_bbox.0.is_finite() {
        candidate_bbox
    } else {
        get_polygon_group_bounds(&cand_polys)
    };

    let grid_ref = spatial_grid.borrow();
    let grid = &grid_ref.inner;
    let indices = grid.query(cand_bbox);

    for &idx in &indices {
        if idx >= placed_polys_groups.len() {
            continue;
        }
        let placed_polys = &placed_polys_groups[idx];
        if placed_polys.is_empty() {
            continue;
        }

        // 1. BBOX check using numpy buffer (no allocation)
        let placed_bbox = numpy_group_bbox(placed_polys);
        if !rects_intersect(cand_bbox, placed_bbox) {
            continue;
        }

        // 2. Hull pre-check (convert on-demand)
        if !cand_hulls.is_empty() && !placed_hulls_groups.is_empty() {
            let placed_hulls = &placed_hulls_groups[idx];
            let mut hulls_meet = false;
            'hull: for cand_hull in &cand_hulls {
                for placed_hull_arr in placed_hulls {
                    let placed_hull = polygon_from_numpy(placed_hull_arr);
                    if polygons_intersect(cand_hull, &placed_hull, 0.0) {
                        hulls_meet = true;
                        break 'hull;
                    }
                }
            }
            if !hulls_meet {
                continue;
            }
        }

        // 3. Detail check (convert placed polys on-demand)
        for cand_poly in &cand_polys {
            for placed_poly_arr in placed_polys {
                let placed_poly = polygon_from_numpy(placed_poly_arr);
                if polygons_intersect(cand_poly, &placed_poly, min_area) {
                    return true;
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(is_contained_py, m)?)?;
    m.add_function(wrap_pyfunction!(any_overlap_py, m)?)?;
    m.add_function(wrap_pyfunction!(any_overlap_hierarchical_py, m)?)?;
    m.add_function(wrap_pyfunction!(any_overlap_hierarchical_grid_py, m)?)?;
    Ok(())
}
