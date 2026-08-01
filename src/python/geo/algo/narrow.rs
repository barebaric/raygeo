use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::narrow;
use crate::geo::types::Point;

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_narrow_passages(
        polygon: collections.abc.Sequence[tuple[float, float]],
        holes: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        max_width: float = 10.0,
    ) -> list[collections.abc.Sequence[tuple[float, float]]]:
        """Detect narrow passages in a polygon.

        A passage is narrow when it is narrower than *max_width*. The
        detection evaluates every boundary edge against every other
        boundary edge via an R-tree spatial index. Edges whose midpoints
        are within *max_width* of a non-adjacent edge (on a different
        polygon or a distinct part of the same polygon) produce a
        quadrilateral via convex hull of their four endpoints.
        All quadrilaterals are unioned and clipped to the pocket.

        :param polygon: Outer boundary polygon.
        :param holes: List of hole (island) polygons.
        :param max_width: Passage-width threshold in mm.
        :returns: List of polygons (each a list of ``(x, y)`` vertices).
        :raises RuntimeError: If the polygon cannot be analyzed.
        """
"#,
    module = "raygeo.geo.algo.narrow"
)]
#[pyfunction(name = "find_narrow_passages")]
#[pyo3(signature = (polygon, holes = None, max_width = 10.0))]
fn find_narrow_passages_py(
    polygon: Vec<(f64, f64)>,
    holes: Option<Vec<Vec<(f64, f64)>>>,
    max_width: f64,
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let poly: Vec<Point> =
        polygon.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let holes_pts: Vec<Vec<Point>> = holes
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let regions = narrow::find_narrow_passages(&poly, &holes_pts, max_width)
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

    Ok(regions
        .into_iter()
        .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
        .collect())
}

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "narrow")?;
    m.setattr(
        "__doc__",
        "Narrow-passage detection via per-edge distance evaluation.",
    )?;

    m.add_function(pyo3::wrap_pyfunction!(
        find_narrow_passages_py,
        m.clone()
    )?)?;

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.narrow", &m)?;
    Ok(())
}
