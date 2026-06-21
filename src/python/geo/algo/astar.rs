use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction};

use crate::geo::algo::astar as astar_core;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "astar")?;
    m.setattr("__doc__", "A* pathfinding on a rasterised grid.")?;

    m.add_function(wrap_pyfunction!(find_path_py, &m)?)?;
    m.add_class::<AStarPath>()?;

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_path(
        from_: tuple[float, float],
        to: tuple[float, float],
        free_space: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        obstacles: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        obstacle_margin: float = 0.0,
        cell_size: float = 1.0,
    ) -> AStarPath | None:
        """Find a path from *from_* to *to* inside *free_space*, avoiding *obstacles*.

        The walkable area is rasterised at *cell_size* resolution.
        Obstacles are dilated by *obstacle_margin* before pathfinding.

        :param from_: Start point (x, y).
        :param to: Goal point (x, y).
        :param free_space: Polygons defining the walkable region.
        :param obstacles: Polygons defining forbidden zones (default []).
        :param obstacle_margin: Radius by which obstacles are expanded (default 0).
        :param cell_size: Raster grid resolution (default 1.0).
        :returns: AStarPath with waypoints, visited count, and length, or None.
        """
    "#,
    module = "raygeo.geo.algo.astar"
)]
#[pyfunction(name = "find_path")]
#[pyo3(signature = (from_, to, free_space, obstacles = None, obstacle_margin = 0.0, cell_size = 1.0))]
fn find_path_py(
    from_: (f64, f64),
    to: (f64, f64),
    free_space: Vec<Vec<(f64, f64)>>,
    obstacles: Option<Vec<Vec<(f64, f64)>>>,
    obstacle_margin: f64,
    cell_size: f64,
) -> Option<AStarPath> {
    let from_pt = crate::types::Point::new(from_.0, from_.1);
    let to_pt = crate::types::Point::new(to.0, to.1);
    let free_pts: Vec<crate::types::Polygon> = free_space
        .into_iter()
        .map(|poly| {
            poly.into_iter()
                .map(|(x, y)| crate::types::Point::new(x, y))
                .collect()
        })
        .collect();
    let obs_pts: Vec<crate::types::Polygon> = obstacles
        .unwrap_or_default()
        .into_iter()
        .map(|poly| {
            poly.into_iter()
                .map(|(x, y)| crate::types::Point::new(x, y))
                .collect()
        })
        .collect();
    astar_core::find_path(
        from_pt,
        to_pt,
        &free_pts,
        &obs_pts,
        obstacle_margin,
        cell_size,
    )
    .map(|p| AStarPath {
        waypoints: p.waypoints.iter().map(|pt| (pt.x, pt.y)).collect(),
        visited: p.visited,
        length: p.length,
    })
}

#[gen_stub_pyclass(module = "raygeo.geo.algo.astar")]
#[pyclass(skip_from_py_object)]
#[derive(Debug, Clone)]
struct AStarPath {
    #[pyo3(get)]
    waypoints: Vec<(f64, f64)>,
    #[pyo3(get)]
    visited: usize,
    #[pyo3(get)]
    length: f64,
}
