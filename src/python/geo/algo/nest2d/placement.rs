use numpy::{PyArray2, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::nest2d::placement;
use crate::geo::types::{Point, Polygon, Rect};
use crate::python::geo::algo::spatial_grid2d::SpatialGrid as PySpatialGrid;
use crate::python::geo::flex_point::{
    option_point_to_tuple, points_to_tuples, poly_to_points,
    polygons_to_tuples, tuples_to_points, PyPoint2D,
};

pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.nest2d.placement",
    "{}",
    MODULE_DOC
);

pub(crate) const MODULE_DOC: &str = "\
Placement search for nesting algorithms.

Provides candidate generation strategies (bottom-left, grid, perimeter),
position search, and high-level nesting orchestration.
";

fn polys_from_py(polys: Vec<Vec<PyPoint2D>>) -> Vec<Polygon> {
    polys.into_iter().map(poly_to_points).collect()
}

fn polys_list_from_py(list: Vec<Vec<Vec<PyPoint2D>>>) -> Vec<Vec<Polygon>> {
    list.into_iter()
        .map(|g| g.into_iter().map(poly_to_points).collect())
        .collect()
}

// ---------------------------------------------------------------------------
// generate_bottom_left_candidates
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    def generate_bottom_left_candidates(
        ifp_bounds: tuple[float, float, float, float],
        part_bounds: tuple[float, float, float, float],
        spacing: float,
    ) -> list[tuple[float, float]]:
        """Generate candidate positions scanning from bottom-left.

        :param ifp_bounds: IFP bounding box (min_x, min_y, max_x, max_y).
        :param part_bounds: Part bounding box (min_x, min_y, max_x, max_y).
        :param spacing: Minimum spacing between parts.
        :returns: List of (x, y) candidate positions.
        :complexity: O(n) where n = number of grid cells scanned.
        """
"#,
    module = "raygeo.geo.algo.nest2d.placement"
)]
#[pyfunction(name = "generate_bottom_left_candidates")]
fn generate_bottom_left_candidates_py(
    ifp_bounds: (f64, f64, f64, f64),
    part_bounds: (f64, f64, f64, f64),
    spacing: f64,
) -> Vec<(f64, f64)> {
    points_to_tuples(placement::generate_bottom_left_candidates(
        Rect::new(ifp_bounds.0, ifp_bounds.1, ifp_bounds.2, ifp_bounds.3),
        Rect::new(part_bounds.0, part_bounds.1, part_bounds.2, part_bounds.3),
        spacing,
    ))
}

// ---------------------------------------------------------------------------
// generate_grid_candidates
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    def generate_grid_candidates(
        ifp_bounds: tuple[float, float, float, float],
        part_bounds: tuple[float, float, float, float],
        spacing: float,
    ) -> list[tuple[float, float]]:
        """Generate candidate positions in a grid pattern.

        :param ifp_bounds: IFP bounding box (min_x, min_y, max_x, max_y).
        :param part_bounds: Part bounding box (min_x, min_y, max_x, max_y).
        :param spacing: Grid spacing.
        :returns: List of (x, y) candidate positions.
        :complexity: O(n) where n = number of grid cells.
        """
"#,
    module = "raygeo.geo.algo.nest2d.placement"
)]
#[pyfunction(name = "generate_grid_candidates")]
fn generate_grid_candidates_py(
    ifp_bounds: (f64, f64, f64, f64),
    part_bounds: (f64, f64, f64, f64),
    spacing: f64,
) -> Vec<(f64, f64)> {
    points_to_tuples(placement::generate_grid_candidates(
        Rect::new(ifp_bounds.0, ifp_bounds.1, ifp_bounds.2, ifp_bounds.3),
        Rect::new(part_bounds.0, part_bounds.1, part_bounds.2, part_bounds.3),
        spacing,
    ))
}

// ---------------------------------------------------------------------------
// generate_perimeter_candidates
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def generate_perimeter_candidates(
        placed_groups: collections.abc.Sequence[collections.abc.Sequence[list[tuple[float, float]]]],
        part_bounds: tuple[float, float, float, float],
        spacing: float,
    ) -> list[tuple[float, float]]:
        """Generate edge-aligned candidates around placed parts.

        For each placed part (group of polygons), produces 8 positions:
        right-bottom, right-top, left-bottom, left-top, above-left,
        above-right, below-left, below-right.

        :param placed_groups: List of placed parts, each a list of polygons.
        :param part_bounds: Part bounding box (min_x, min_y, max_x, max_y).
        :param spacing: Minimum spacing between parts.
        :returns: List of (x, y) candidate positions.
        :complexity: O(n) where n = number of placed parts.
        """
"#,
    module = "raygeo.geo.algo.nest2d.placement"
)]
#[pyfunction(name = "generate_perimeter_candidates")]
fn generate_perimeter_candidates_py(
    placed_groups: Vec<Vec<Vec<PyPoint2D>>>,
    part_bounds: (f64, f64, f64, f64),
    spacing: f64,
) -> Vec<(f64, f64)> {
    let groups: Vec<Vec<Polygon>> = placed_groups
        .into_iter()
        .map(|group| group.into_iter().map(poly_to_points).collect())
        .collect();
    points_to_tuples(placement::generate_perimeter_candidates(
        &groups,
        Rect::new(part_bounds.0, part_bounds.1, part_bounds.2, part_bounds.3),
        spacing,
    ))
}

// ---------------------------------------------------------------------------
// filter_candidates_multi_resolution
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    def filter_candidates_multi_resolution(
        candidates: list[tuple[float, float]],
        ifp_bounds: tuple[float, float, float, float],
        min_dist: float,
    ) -> list[tuple[float, float]]:
        """Remove candidates that are too close together.

        :param candidates: List of (x, y) candidate positions.
        :param ifp_bounds: IFP bounding box (min_x, min_y, max_x, max_y).
        :param min_dist: Minimum allowed distance between candidates.
        :returns: Filtered list of (x, y) positions.
        :complexity: O(n log n) for spatial distance filtering.
        """
"#,
    module = "raygeo.geo.algo.nest2d.placement"
)]
#[pyfunction(name = "filter_candidates_multi_resolution")]
fn filter_candidates_multi_resolution_py(
    candidates: Vec<(f64, f64)>,
    ifp_bounds: (f64, f64, f64, f64),
    min_dist: f64,
) -> Vec<(f64, f64)> {
    let candidates = tuples_to_points(candidates);
    points_to_tuples(placement::filter_candidates_multi_resolution(
        &candidates,
        Rect::new(ifp_bounds.0, ifp_bounds.1, ifp_bounds.2, ifp_bounds.3),
        min_dist,
    ))
}

fn make_config(
    spacing: f64,
    min_area: f64,
    curve_tolerance: f64,
) -> placement::PlacementConfig {
    placement::PlacementConfig {
        spacing,
        min_area,
        curve_tolerance,
    }
}

macro_rules! with_grid {
    ($grid:expr, |$inner:ident| $($body:tt)+) => {{
        let __g = $grid.borrow();
        let $inner = &__g.inner;
        $($body)+
    }};
}

// ---------------------------------------------------------------------------
// find_valid_position
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.algo.spatial_grid2d

    def find_valid_position(
        ifp_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        part_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        part_hulls: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        placed_polys_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]],
        placed_hulls_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]],
        grid: spatial_grid2d.SpatialGrid,
        sheet_world_offset: tuple[float, float],
        spacing: float = 1.0,
        min_area: float = 1.0,
        curve_tolerance: float = 0.5,
    ) -> tuple[float, float] | None:
        """Find a valid position: heuristic search first, NFP fallback second.

        Supports hull-based collision detection and sheet world offsets.

        :param ifp_polygons: IFP polygons (valid placement region).
        :param part_polygons: Part polygons to place.
        :param part_hulls: Convex hulls for collision (may be empty).
        :param placed_polys_list: Already-placed parts, each a list of polygons.
        :param placed_hulls_list: Hulls of already-placed parts, each a list.
        :param grid: SpatialGrid for fast neighbor lookup.
        :param sheet_world_offset: (offset_x, offset_y) for this sheet.
        :param spacing: Minimum spacing between parts.
        :param min_area: Minimum overlap area (clipper coords).
        :param curve_tolerance: Curve tolerance for distance filtering.
        :returns: (x, y) position or None.
        :complexity: O(n * m) for candidate search with overlap checks.
        """
"#,
    module = "raygeo.geo.algo.nest2d.placement"
)]
#[pyfunction(name = "find_valid_position")]
#[pyo3(signature = (ifp_polygons, part_polygons, part_hulls, placed_polys_list, placed_hulls_list, grid, sheet_world_offset, spacing = 1.0, min_area = 1.0, curve_tolerance = 0.5))]
#[allow(clippy::too_many_arguments)]
fn find_valid_position_py(
    ifp_polygons: Vec<Vec<PyPoint2D>>,
    part_polygons: Vec<Vec<PyPoint2D>>,
    part_hulls: Vec<Vec<PyPoint2D>>,
    placed_polys_list: Vec<Vec<Vec<PyPoint2D>>>,
    placed_hulls_list: Vec<Vec<Vec<PyPoint2D>>>,
    grid: &Bound<'_, PySpatialGrid>,
    sheet_world_offset: (f64, f64),
    spacing: f64,
    min_area: f64,
    curve_tolerance: f64,
) -> Option<(f64, f64)> {
    let ifp = polys_from_py(ifp_polygons);
    let part = polys_from_py(part_polygons);
    let hulls = polys_from_py(part_hulls);
    let placed = polys_list_from_py(placed_polys_list);
    let placed_hulls = polys_list_from_py(placed_hulls_list);
    let config = make_config(spacing, min_area, curve_tolerance);
    option_point_to_tuple(with_grid!(grid, |grid_ref| {
        placement::find_valid_position(
            &ifp,
            &part,
            &hulls,
            &placed,
            &placed_hulls,
            grid_ref,
            sheet_world_offset,
            &config,
            spacing,
        )
    }))
}

// ---------------------------------------------------------------------------
// find_valid_position_scored
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.algo.spatial_grid2d

    def find_valid_position_scored(
        ifp_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        part_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        part_hulls: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        placed_polys_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]],
        placed_hulls_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]],
        grid: spatial_grid2d.SpatialGrid,
        sheet_world_offset: tuple[float, float],
        spacing: float = 1.0,
        min_area: float = 1.0,
        curve_tolerance: float = 0.5,
    ) -> tuple[float, float] | None:
        """Find a valid position using heuristic candidate search.

        Uses IFP vertices, bottom-left sweep, grid, placed polygon vertices,
        and perimeter candidates. Falls back to NFP-region candidates.
        Scores candidates and picks the best valid one.

        :param ifp_polygons: IFP polygons (valid placement region).
        :param part_polygons: Part polygons to place.
        :param part_hulls: Convex hulls for collision (may be empty).
        :param placed_polys_list: Already-placed parts, each a list of polygons.
        :param placed_hulls_list: Hulls of already-placed parts, each a list.
        :param grid: SpatialGrid for fast neighbor lookup.
        :param sheet_world_offset: (offset_x, offset_y) for this sheet.
        :param spacing: Minimum spacing between parts.
        :param min_area: Minimum overlap area (clipper coords).
        :param curve_tolerance: Curve tolerance for distance filtering.
        :returns: (x, y) position or None.
        :complexity: O(n * m) for scored candidate search.
        """
"#,
    module = "raygeo.geo.algo.nest2d.placement"
)]
#[pyfunction(name = "find_valid_position_scored")]
#[pyo3(signature = (ifp_polygons, part_polygons, part_hulls, placed_polys_list, placed_hulls_list, grid, sheet_world_offset, spacing = 1.0, min_area = 1.0, curve_tolerance = 0.5))]
#[allow(clippy::too_many_arguments)]
fn find_valid_position_scored_py(
    ifp_polygons: Vec<Vec<PyPoint2D>>,
    part_polygons: Vec<Vec<PyPoint2D>>,
    part_hulls: Vec<Vec<PyPoint2D>>,
    placed_polys_list: Vec<Vec<Vec<PyPoint2D>>>,
    placed_hulls_list: Vec<Vec<Vec<PyPoint2D>>>,
    grid: &Bound<'_, PySpatialGrid>,
    sheet_world_offset: (f64, f64),
    spacing: f64,
    min_area: f64,
    curve_tolerance: f64,
) -> Option<(f64, f64)> {
    let ifp = polys_from_py(ifp_polygons);
    let part = polys_from_py(part_polygons);
    let hulls = polys_from_py(part_hulls);
    let placed = polys_list_from_py(placed_polys_list);
    let placed_hulls = polys_list_from_py(placed_hulls_list);
    let config = make_config(spacing, min_area, curve_tolerance);
    option_point_to_tuple(with_grid!(grid, |grid_ref| {
        placement::find_valid_position_scored(
            &ifp,
            &part,
            &hulls,
            &placed,
            &placed_hulls,
            grid_ref,
            sheet_world_offset,
            &config,
            spacing,
        )
    }))
}

// ---------------------------------------------------------------------------
// find_valid_position_nfp
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.algo.spatial_grid2d

    def find_valid_position_nfp(
        ifp_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        part_polygons: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        part_hulls: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        placed_polys_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]],
        placed_hulls_list: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]],
        grid: spatial_grid2d.SpatialGrid,
        sheet_world_offset: tuple[float, float],
        spacing: float = 1.0,
        min_area: float = 1.0,
        curve_tolerance: float = 0.5,
    ) -> tuple[float, float] | None:
        """Find a valid position using NFP-based region subtraction.

        Computes No-Fit Polygons for nearby placed parts and subtracts
        them from the IFP to identify viable placement regions.

        :param ifp_polygons: IFP polygons (valid placement region).
        :param part_polygons: Part polygons to place.
        :param part_hulls: Convex hulls for collision (may be empty).
        :param placed_polys_list: Already-placed parts, each a list of polygons.
        :param placed_hulls_list: Hulls of already-placed parts, each a list.
        :param grid: SpatialGrid for fast neighbor lookup.
        :param sheet_world_offset: (offset_x, offset_y) for this sheet.
        :param spacing: Minimum spacing between parts.
        :param min_area: Minimum overlap area (clipper coords).
        :param curve_tolerance: Curve tolerance for distance filtering.
        :returns: (x, y) position or None.
        :complexity: O(n * m) for NFP construction per nearby placed part.
        """
"#,
    module = "raygeo.geo.algo.nest2d.placement"
)]
#[pyfunction(name = "find_valid_position_nfp")]
#[pyo3(signature = (ifp_polygons, part_polygons, part_hulls, placed_polys_list, placed_hulls_list, grid, sheet_world_offset, spacing = 1.0, min_area = 1.0, curve_tolerance = 0.5))]
#[allow(clippy::too_many_arguments)]
fn find_valid_position_nfp_py(
    ifp_polygons: Vec<Vec<PyPoint2D>>,
    part_polygons: Vec<Vec<PyPoint2D>>,
    part_hulls: Vec<Vec<PyPoint2D>>,
    placed_polys_list: Vec<Vec<Vec<PyPoint2D>>>,
    placed_hulls_list: Vec<Vec<Vec<PyPoint2D>>>,
    grid: &Bound<'_, PySpatialGrid>,
    sheet_world_offset: (f64, f64),
    spacing: f64,
    min_area: f64,
    curve_tolerance: f64,
) -> Option<(f64, f64)> {
    let ifp = polys_from_py(ifp_polygons);
    let part = polys_from_py(part_polygons);
    let hulls = polys_from_py(part_hulls);
    let placed = polys_list_from_py(placed_polys_list);
    let placed_hulls = polys_list_from_py(placed_hulls_list);
    let config = make_config(spacing, min_area, curve_tolerance);
    option_point_to_tuple(with_grid!(grid, |grid_ref| {
        placement::find_valid_position_nfp(
            &ifp,
            &part,
            &hulls,
            &placed,
            &placed_hulls,
            grid_ref,
            sheet_world_offset,
            &config,
            spacing,
        )
    }))
}

// ---------------------------------------------------------------------------
// place_parts (high-level)
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def place_parts(
        part_polys: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]],
        part_hulls: collections.abc.Sequence[collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]]],
        sheet_polys: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        sheet_offsets: collections.abc.Sequence[tuple[float, float]],
        rotations: collections.abc.Sequence[float],
        flips_h: collections.abc.Sequence[bool],
        flips_v: collections.abc.Sequence[bool],
        spacing: float = 1.0,
        min_area: float = 1.0,
        curve_tolerance: float = 0.5,
    ) -> list[dict]:
        """Place as many parts as possible onto sheets.

        Supports combined IFP for multi-polygon parts, hull-based
        collision detection, world-space offsets per sheet, gravity
        post-processing, and fitness calculation.

        Parts are sorted by area (largest first).  For each part, the
        best sheet and position are selected.  After all parts are
        placed, gravity is applied per sheet.

        :param part_polys: List of parts, each a list of polygon point lists.
        :param part_hulls: List of hull groups per part (may be empty).
        :param sheet_polys: List of sheet polygons.
        :param sheet_offsets: World-space offset (x, y) for each sheet.
        :param rotations: Rotation angle (degrees) for each part.
        :param flips_h: Horizontal flip flag per part.
        :param flips_v: Vertical flip flag per part.
        :param spacing: Minimum spacing between parts.
        :param min_area: Minimum overlap area (clipper coords).
        :param curve_tolerance: Curve tolerance for distance filtering.
        :returns: List of dicts, one per sheet, with keys: placements,
                  sheet_index, unused_part_indices, fitness.
        :complexity: O(p * s * n * m) where p = parts, s = sheets.
        """
"#,
    module = "raygeo.geo.algo.nest2d.placement"
)]
#[pyfunction(name = "place_parts")]
#[pyo3(signature = (part_polys, part_hulls, sheet_polys, sheet_offsets, rotations, flips_h, flips_v, spacing = 1.0, min_area = 1.0, curve_tolerance = 0.5))]
#[allow(clippy::too_many_arguments)]
fn place_parts_py<'py>(
    py: Python<'py>,
    part_polys: Vec<Vec<Vec<PyPoint2D>>>,
    part_hulls: Vec<Vec<Vec<PyPoint2D>>>,
    sheet_polys: Vec<Vec<PyPoint2D>>,
    sheet_offsets: Vec<(f64, f64)>,
    rotations: Vec<f64>,
    flips_h: Vec<bool>,
    flips_v: Vec<bool>,
    spacing: f64,
    min_area: f64,
    curve_tolerance: f64,
) -> Vec<Bound<'py, PyDict>> {
    let parts: Vec<placement::PartDesc> = part_polys
        .into_iter()
        .zip(part_hulls)
        .map(|(polys, hulls)| placement::PartDesc {
            polygons: polys.into_iter().map(poly_to_points).collect(),
            hulls: hulls.into_iter().map(poly_to_points).collect(),
        })
        .collect();

    let sheets: Vec<placement::SheetDesc> = sheet_polys
        .into_iter()
        .zip(sheet_offsets)
        .map(|(poly, offset)| placement::SheetDesc {
            polygon: poly_to_points(poly),
            world_offset: offset,
        })
        .collect();

    let config = placement::PlacementConfig {
        spacing,
        min_area,
        curve_tolerance,
    };

    let results = placement::place_parts(
        &parts, &sheets, &rotations, &config, &flips_h, &flips_v,
    );

    let mut py_results: Vec<Bound<'py, PyDict>> = Vec::new();
    for res in results {
        let mut placements_py: Vec<Bound<'py, PyDict>> = Vec::new();
        for pl in res.placements {
            let pl_dict = PyDict::new(py);
            pl_dict.set_item("part_index", pl.part_index).unwrap();
            pl_dict
                .set_item("rotation_index", pl.rotation_index)
                .unwrap();
            pl_dict
                .set_item("position", (pl.position.x, pl.position.y))
                .unwrap();
            pl_dict
                .set_item("polygons", polygons_to_tuples(pl.polygons))
                .unwrap();
            pl_dict
                .set_item("hulls", polygons_to_tuples(pl.hulls))
                .unwrap();
            placements_py.push(pl_dict);
        }
        let res_dict = PyDict::new(py);
        res_dict.set_item("placements", placements_py).unwrap();
        res_dict.set_item("sheet_index", res.sheet_index).unwrap();
        res_dict
            .set_item("unused_part_indices", res.unused_part_indices)
            .unwrap();
        res_dict.set_item("fitness", res.fitness).unwrap();
        py_results.push(res_dict);
    }
    py_results
}

// ---------------------------------------------------------------------------
// calculate_fitness
// ---------------------------------------------------------------------------

fn polygon_group_from_numpy_arrs(
    arrs: Vec<Bound<'_, PyArray2<f64>>>,
) -> Vec<Polygon> {
    arrs.iter()
        .map(|arr| {
            let readonly = arr.readonly();
            let view = readonly.as_array();
            view.rows()
                .into_iter()
                .map(|row| Point::new(row[0], row[1]))
                .collect()
        })
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import numpy

    def calculate_fitness(
        polygon_groups: collections.abc.Sequence[collections.abc.Sequence[numpy.ndarray]],
        rotations: collections.abc.Sequence[float],
        sheet_indices: collections.abc.Sequence[int],
        *,
        num_parts: int = 0,
    ) -> float:
        """Calculate fitness score for a set of placements.

        Lower is better. Returns infinity if no placements or zero area.

        :param polygon_groups: Polygons for each placement.
        :param rotations: Rotation angle in degrees for each placement.
        :param sheet_indices: 0-based sheet index for each placement.
        :param num_parts: Total number of parts (some may be unplaced).
        :returns: Fitness score (lower is better).
        :complexity: O(n) where n = number of placements.
        """
"#,
    module = "raygeo.geo.algo.nest2d.placement"
)]
#[pyfunction(name = "calculate_fitness")]
#[pyo3(signature = (polygon_groups, rotations, sheet_indices, num_parts = 0))]
fn calculate_fitness_py(
    polygon_groups: Vec<Vec<Bound<'_, PyArray2<f64>>>>,
    rotations: Vec<f64>,
    sheet_indices: Vec<usize>,
    num_parts: usize,
) -> f64 {
    let groups: Vec<Vec<Polygon>> = polygon_groups
        .into_iter()
        .map(polygon_group_from_numpy_arrs)
        .collect();
    placement::calculate_fitness(&groups, &rotations, &sheet_indices, num_parts)
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(generate_bottom_left_candidates_py, m)?)?;
    m.add_function(wrap_pyfunction!(generate_grid_candidates_py, m)?)?;
    m.add_function(wrap_pyfunction!(generate_perimeter_candidates_py, m)?)?;
    m.add_function(wrap_pyfunction!(
        filter_candidates_multi_resolution_py,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(find_valid_position_py, m)?)?;
    m.add_function(wrap_pyfunction!(find_valid_position_scored_py, m)?)?;
    m.add_function(wrap_pyfunction!(find_valid_position_nfp_py, m)?)?;
    m.add_function(wrap_pyfunction!(place_parts_py, m)?)?;
    m.add_function(wrap_pyfunction!(calculate_fitness_py, m)?)?;
    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.nest2d.placement", m)?;
    Ok(())
}
