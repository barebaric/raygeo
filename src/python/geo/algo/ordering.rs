pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.ordering",
    "{}",
    MODULE_DOC_ORDERING
);

pub(crate) const MODULE_DOC_ORDERING: &str = "\
Path ordering algorithms.

Provides pure-geometry combinatorial optimizations for sequencing
paths (polylines, arcs) with no machining or CNC concepts.
";

use crate::geo::algo::ordering::order_nearest_neighbor;
use crate::geo::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "ordering")?;
    m.setattr("__doc__", MODULE_DOC_ORDERING)?;

    register_functions!(m, order_nearest_neighbor_py,);

    algo_mod.add_submodule(&m)?;
    let sys_modules = algo_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.ordering", &m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def order_nearest_neighbor(
        paths: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
    ) -> list[int]:
        """Order paths by greedy nearest-neighbor starting from the longest path.

        Starts with the longest input path (most vertices), then repeatedly
        selects the next unvisited path whose first endpoint is closest to
        the current path's last endpoint.

        :param paths: List of paths, each a list of (x, y) points.
        :returns: Indices into *paths* in visit order.
        """
"#,
    module = "raygeo.geo.algo.ordering"
)]
#[pyfunction(name = "order_nearest_neighbor")]
fn order_nearest_neighbor_py(paths: Vec<Vec<(f64, f64)>>) -> Vec<usize> {
    let pts: Vec<Vec<Point>> = paths
        .into_iter()
        .map(|path| path.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    order_nearest_neighbor(&pts)
}
