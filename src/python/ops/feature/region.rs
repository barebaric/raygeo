use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::feature::region::find_regions;
use crate::types::Point;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_regions(
        boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        tool_radius: float = 3.0,
        tolerance: float = 0.5,
    ) -> list[tuple[list[tuple[float, float]], float, tuple[float, float], float]]:
        """Detect disconnected wide sub-regions of a pocket.

        Analyses the pocket boundary (with optional islands) for narrow
        passages that separate the pocket into disconnected wide sub-regions.
        Each sub-region is returned as a tuple of
        ``(polygon, area, entry_pt, r_max)``.

        * ``polygon`` — list of ``(x, y)`` vertices of the wide region.
        * ``area`` — area of the sub-region in mm².
        * ``entry_pt`` — ``(x, y)`` centre of the largest inscribed circle.
        * ``r_max`` — radius of the largest inscribed circle in mm.

        Results are sorted by area descending (largest region first).
        Returns an empty list when the pocket is entirely narrow/slot.

        :param boundary: Outer pocket boundary as ``[(x, y), ...]``.
        :param islands: List of island polygons, each as ``[(x, y), ...]``.
        :param tool_radius: Tool radius in mm.
        :param tolerance: Additional clearance tolerance in mm.
        :returns: List of ``(polygon, area, entry_pt, r_max)`` tuples.
        """
    "#,
    module = "raygeo.ops.feature.region"
)]
#[allow(clippy::type_complexity)]
#[pyfunction(name = "find_regions")]
#[pyo3(signature = (boundary, islands = None, tool_radius = 3.0, tolerance = 0.5))]
fn find_regions_py(
    boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    tolerance: f64,
) -> Vec<(Vec<(f64, f64)>, f64, (f64, f64), f64)> {
    let boundary_pts: Vec<Point> = boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let regions =
        find_regions(&boundary_pts, &islands_pts, tool_radius, tolerance);

    regions
        .into_iter()
        .map(|r| {
            let poly_py: Vec<(f64, f64)> =
                r.polygon.into_iter().map(|p| (p.x, p.y)).collect();
            (poly_py, r.area, (r.entry_pt.x, r.entry_pt.y), r.r_max)
        })
        .collect()
}

pub fn register(feature_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = feature_mod.py();
    let m = PyModule::new(py, "region")?;
    m.setattr("__doc__", "Wide-region detection for adaptive clearing.")?;

    m.add_function(pyo3::wrap_pyfunction!(find_regions_py, m.clone())?)?;

    feature_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.feature.region", &m)?;

    Ok(())
}
