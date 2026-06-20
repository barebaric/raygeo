use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::nest2d::gravity;
use crate::python::geo::flex_point::{poly_to_points, PyPoint2D};
use crate::types::{Polygon, Rect};

pyo3_stub_gen::module_doc!("raygeo.geo.algo.nest2d.gravity", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Gravity optimization for nesting layouts.

Provides binary-search gravity sliding to tighten packing by sliding
parts down and left as far as possible without overlapping.
";

// ---------------------------------------------------------------------------
// find_max_slide
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def find_max_slide(
        polys: collections.abc.Sequence[types.Polygon],
        other_polys_list: collections.abc.Sequence[
            collections.abc.Sequence[types.Polygon]
        ],
        sheet_bounds: tuple[float, float, float, float],
        sheet_poly: types.Polygon,
        axis: str,
        spacing: float,
    ) -> float:
        """Find the maximum distance a part can slide in the negative axis direction.

        Uses binary search with polygon overlap and containment checks.

        :param polys: Polygons of the part to slide.
        :param other_polys_list: Polygons of all other placed parts (grouped).
        :param sheet_bounds: Sheet bounding box (min_x, min_y, max_x, max_y).
        :param sheet_poly: Sheet polygon.
        :param axis: ``"x"`` or ``"y"`` — axis to slide along.
        :param spacing: Minimum spacing between parts.
        :returns: Maximum slide distance.
        :complexity: O(log range * n * m) for binary search with overlap checks.
        """
"#,
    module = "raygeo.geo.algo.nest2d.gravity"
)]
#[pyfunction(name = "find_max_slide")]
fn find_max_slide_py(
    polys: Vec<Vec<PyPoint2D>>,
    other_polys_list: Vec<Vec<Vec<PyPoint2D>>>,
    sheet_bounds: (f64, f64, f64, f64),
    sheet_poly: Vec<PyPoint2D>,
    axis: String,
    spacing: f64,
) -> f64 {
    let parts: Vec<Polygon> = polys.into_iter().map(poly_to_points).collect();
    let others: Vec<Vec<Polygon>> = other_polys_list
        .into_iter()
        .map(|g| g.into_iter().map(poly_to_points).collect())
        .collect();
    let sheet = poly_to_points(sheet_poly);
    gravity::find_max_slide(
        &parts,
        &others,
        Rect::new(
            sheet_bounds.0,
            sheet_bounds.1,
            sheet_bounds.2,
            sheet_bounds.3,
        ),
        &sheet,
        &axis,
        spacing,
    )
}

// ---------------------------------------------------------------------------
// apply_gravity
// ---------------------------------------------------------------------------

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo.geo.types

    def apply_gravity(
        placement_groups: collections.abc.Sequence[
            collections.abc.Sequence[types.Polygon]
        ],
        sheet_poly: types.Polygon,
        spacing: float,
    ) -> list[tuple[float, float]]:
        """Apply gravity sliding to tighten a nesting layout.

        Iterates Y and X passes until no movement occurs (max 10 passes).
        Returns a ``(dx, dy)`` adjustment for each input group in order.

        :param placement_groups: List of placed parts (each a list of polygons).
        :param sheet_poly: Sheet polygon.
        :param spacing: Minimum spacing between parts.
        :returns: List of ``(dx, dy)`` adjustments, one per group.
        :complexity: O(passes * n * m) where passes ≤ 10.
        """
"#,
    module = "raygeo.geo.algo.nest2d.gravity"
)]
#[pyfunction(name = "apply_gravity")]
fn apply_gravity_py(
    placement_groups: Vec<Vec<Vec<PyPoint2D>>>,
    sheet_poly: Vec<PyPoint2D>,
    spacing: f64,
) -> Vec<(f64, f64)> {
    let groups: Vec<Vec<Polygon>> = placement_groups
        .into_iter()
        .map(|g| g.into_iter().map(poly_to_points).collect())
        .collect();
    let sheet = poly_to_points(sheet_poly);
    gravity::apply_gravity(&groups, &sheet, spacing)
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(find_max_slide_py, m)?)?;
    m.add_function(wrap_pyfunction!(apply_gravity_py, m)?)?;
    Ok(())
}
