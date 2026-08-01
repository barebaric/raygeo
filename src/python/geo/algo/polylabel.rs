use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::polylabel;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "polylabel")?;
    m.setattr(
        "__doc__",
        "Pole-of-inaccessibility computation via Polylabel.",
    )?;

    register_functions!(m, find_largest_circle_py, polylabel_py,);

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.polylabel", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def get_polylabel(
        shell: collections.abc.Sequence[tuple[float, float]],
        holes: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        precision: float = 0.5,
    ) -> tuple[float, float] | None:
        """Find the pole of inaccessibility of a polygon (with optional holes).

        Uses the Polylabel algorithm (Mapbox): a priority-queue of
        grid cells repeatedly subdivided until the cell radius drops
        below *precision*.

        :param shell: Outer boundary polygon.
        :param holes: List of hole polygons to exclude (default []).
        :param precision: Desired precision (default 0.5).
        :returns: (x, y) of the most interior point, or None for
                  degenerate polygons.
        :complexity: O(n log n) where n is the number of cells explored.
        """
"#,
    module = "raygeo.geo.algo.polylabel"
)]
#[pyfunction(name = "get_polylabel")]
#[pyo3(signature = (shell, holes = None, precision = 0.5))]
fn polylabel_py(
    shell: Vec<(f64, f64)>,
    holes: Option<Vec<Vec<(f64, f64)>>>,
    precision: f64,
) -> Option<(f64, f64)> {
    let shell_pts: Vec<crate::types::Point> = shell
        .into_iter()
        .map(|(x, y)| crate::types::Point::new(x, y))
        .collect();
    let holes_pts: Vec<Vec<crate::types::Point>> = holes
        .unwrap_or_default()
        .into_iter()
        .map(|h| {
            h.into_iter()
                .map(|(x, y)| crate::types::Point::new(x, y))
                .collect()
        })
        .collect();
    polylabel::get_polylabel(&shell_pts, &holes_pts, precision)
        .map(|p| (p.x, p.y))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_largest_circle(
        shell: collections.abc.Sequence[tuple[float, float]],
        holes: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        precision: float = 0.5,
    ) -> tuple[tuple[float, float], float] | None:
        """Find the centre and radius of the largest inscribed circle.

        :param shell: Outer boundary polygon.
        :param holes: List of hole polygons to exclude (default []).
        :param precision: Desired precision (default 0.5).
        :returns: ((x, y), radius) or None for degenerate polygons.
        """
"#,
    module = "raygeo.geo.algo.polylabel"
)]
#[pyfunction(name = "find_largest_circle")]
#[pyo3(signature = (shell, holes = None, precision = 0.5))]
fn find_largest_circle_py(
    shell: Vec<(f64, f64)>,
    holes: Option<Vec<Vec<(f64, f64)>>>,
    precision: f64,
) -> Option<((f64, f64), f64)> {
    let shell_pts: Vec<crate::types::Point> = shell
        .into_iter()
        .map(|(x, y)| crate::types::Point::new(x, y))
        .collect();
    let holes_pts: Vec<Vec<crate::types::Point>> = holes
        .unwrap_or_default()
        .into_iter()
        .map(|h| {
            h.into_iter()
                .map(|(x, y)| crate::types::Point::new(x, y))
                .collect()
        })
        .collect();
    polylabel::find_largest_circle(&shell_pts, &holes_pts, precision)
        .map(|(p, r)| ((p.x, p.y), r))
}
