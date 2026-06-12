use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use super::super::geo::flex_point::{poly_to_points, PyPoint2D};
use crate::nest::placement;
use crate::types::Polygon;

pyo3_stub_gen::module_doc!("raygeo.nest.placement", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Placement search for nesting algorithms.

Provides candidate generation strategies (bottom-left, grid, perimeter),
position search, and high-level nesting orchestration.
";

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
        """
"#,
    module = "raygeo.nest.placement"
)]
#[pyfunction(name = "generate_bottom_left_candidates")]
fn generate_bottom_left_candidates_py(
    ifp_bounds: (f64, f64, f64, f64),
    part_bounds: (f64, f64, f64, f64),
    spacing: f64,
) -> Vec<(f64, f64)> {
    placement::generate_bottom_left_candidates(ifp_bounds, part_bounds, spacing)
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
        """
"#,
    module = "raygeo.nest.placement"
)]
#[pyfunction(name = "generate_grid_candidates")]
fn generate_grid_candidates_py(
    ifp_bounds: (f64, f64, f64, f64),
    part_bounds: (f64, f64, f64, f64),
    spacing: f64,
) -> Vec<(f64, f64)> {
    placement::generate_grid_candidates(ifp_bounds, part_bounds, spacing)
}

// ---------------------------------------------------------------------------
// generate_perimeter_candidates
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def generate_perimeter_candidates(
        placed_groups: collections.abc.Sequence[collections.abc.Sequence[types.Polygon]],
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
        """
"#,
    module = "raygeo.nest.placement"
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
    placement::generate_perimeter_candidates(&groups, part_bounds, spacing)
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
        """
"#,
    module = "raygeo.nest.placement"
)]
#[pyfunction(name = "filter_candidates_multi_resolution")]
fn filter_candidates_multi_resolution_py(
    candidates: Vec<(f64, f64)>,
    ifp_bounds: (f64, f64, f64, f64),
    min_dist: f64,
) -> Vec<(f64, f64)> {
    placement::filter_candidates_multi_resolution(
        &candidates,
        ifp_bounds,
        min_dist,
    )
}

// ---------------------------------------------------------------------------
// find_valid_position
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def find_valid_position(
        ifp_polygons: collections.abc.Sequence[types.Polygon],
        part_polygons: collections.abc.Sequence[types.Polygon],
        placed_polygons: collections.abc.Sequence[types.Polygon],
        spacing: float,
        scale: int,
        min_area: float = 1.0,
    ) -> tuple[float, float] | None:
        """Find the first valid placement position for a part.

        :param ifp_polygons: IFP polygons (valid placement region).
        :param part_polygons: Part polygons to place.
        :param placed_polygons: Already-placed polygons.
        :param spacing: Minimum spacing between parts.
        :param scale: Clipper scale factor.
        :param min_area: Minimum overlap area (clipper coords).
        :returns: (x, y) position or None.
        """
"#,
    module = "raygeo.nest.placement"
)]
#[pyfunction(name = "find_valid_position")]
fn find_valid_position_py(
    ifp_polygons: Vec<Vec<PyPoint2D>>,
    part_polygons: Vec<Vec<PyPoint2D>>,
    placed_polygons: Vec<Vec<PyPoint2D>>,
    spacing: f64,
    scale: i64,
    min_area: f64,
) -> Option<(f64, f64)> {
    let ifp: Vec<Polygon> =
        ifp_polygons.into_iter().map(poly_to_points).collect();
    let part: Vec<Polygon> =
        part_polygons.into_iter().map(poly_to_points).collect();
    let placed: Vec<Polygon> =
        placed_polygons.into_iter().map(poly_to_points).collect();
    let config = placement::PlacementConfig {
        spacing,
        scale,
        min_area,
    };
    placement::find_valid_position(&ifp, &part, &placed, &config, spacing)
}

// ---------------------------------------------------------------------------
// place_parts (high-level)
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def place_parts(
        parts: collections.abc.Sequence[collections.abc.Sequence[types.Polygon]],
        sheets: collections.abc.Sequence[types.Polygon],
        rotations: collections.abc.Sequence[float],
        spacing: float,
        scale: int,
        min_area: float = 1.0,
    ) -> list[dict]:
        """Place as many parts as possible onto sheets.

        Each part is a list of polygons (holes are separate polygons).
        Each sheet is a single polygon.

        :param parts: List of parts (each part is a list of polygons).
        :param sheets: List of sheet polygons.
        :param rotations: List of rotation angles in degrees.
        :param spacing: Minimum spacing between parts.
        :param scale: Clipper scale factor.
        :param min_area: Minimum overlap area (clipper coords).
        :returns: List of dicts, one per sheet, with keys:
                  ``placements``, ``sheet_index``, ``unused_part_indices``.
        """
"#,
    module = "raygeo.nest.placement"
)]
#[pyfunction(name = "place_parts")]
fn place_parts_py<'py>(
    py: Python<'py>,
    parts: Vec<Vec<Vec<PyPoint2D>>>,
    sheets: Vec<Vec<PyPoint2D>>,
    rotations: Vec<f64>,
    spacing: f64,
    scale: i64,
    min_area: f64,
) -> Vec<Bound<'py, PyDict>> {
    let parts_rust: Vec<Vec<Polygon>> = parts
        .into_iter()
        .map(|part| part.into_iter().map(poly_to_points).collect())
        .collect();
    let sheets_rust: Vec<Polygon> =
        sheets.into_iter().map(poly_to_points).collect();
    let config = placement::PlacementConfig {
        spacing,
        scale,
        min_area,
    };
    let results =
        placement::place_parts(&parts_rust, &sheets_rust, &rotations, &config);
    let mut py_results: Vec<Bound<'py, PyDict>> = Vec::new();
    for res in results {
        let mut placements_py: Vec<Bound<'py, PyDict>> = Vec::new();
        for pl in res.placements {
            let pl_dict = PyDict::new(py);
            pl_dict.set_item("part_index", pl.part_index).unwrap();
            pl_dict
                .set_item("rotation_index", pl.rotation_index)
                .unwrap();
            pl_dict.set_item("position", pl.position).unwrap();
            let py_polys: Vec<Vec<(f64, f64)>> = pl.polygons;
            pl_dict.set_item("polygons", py_polys).unwrap();
            placements_py.push(pl_dict);
        }
        let res_dict = PyDict::new(py);
        res_dict.set_item("placements", placements_py).unwrap();
        res_dict.set_item("sheet_index", res.sheet_index).unwrap();
        res_dict
            .set_item("unused_part_indices", res.unused_part_indices)
            .unwrap();
        py_results.push(res_dict);
    }
    py_results
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
    m.add_function(wrap_pyfunction!(place_parts_py, m)?)?;
    Ok(())
}
