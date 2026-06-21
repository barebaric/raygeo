pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.medial_axis",
    "{}",
    MODULE_DOC_MEDIAL_AXIS
);

pub(crate) const MODULE_DOC_MEDIAL_AXIS: &str = "\
Medial Axis Transform (MAT) computation.

The MAT is the skeleton of a 2D domain — the set of points equidistant
to two or more boundary features.  It is computed via Delaunay-circumcenter
extraction from a constrained triangulation of the pocket boundary.

* ``compute_medial_axis`` — compute the MAT of a pocket (with optional islands).
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::medial_axis as rust_ma;
use crate::types::Point;

type PyMaResult = (
    Vec<(f64, f64)>,     // nodes: (x, y)
    Vec<f64>,            // clearances
    Vec<(usize, usize)>, // edges
    usize,               // root
    Vec<Vec<usize>>,     // branches (node indices)
);

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "medial_axis")?;
    m.setattr("__doc__", MODULE_DOC_MEDIAL_AXIS)?;

    register_functions!(m, compute_medial_axis_py, mat_path_py);

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def compute_medial_axis(
        outer: collections.abc.Sequence[tuple[float, float]],
        holes: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 1.0,
        sampling_spacing: float = 1.0,
    ) -> tuple[
        list[tuple[float, float]],
        list[float],
        list[tuple[int, int]],
        int,
        list[list[int]],
    ]:
        """Compute the Medial Axis Transform of a planar domain.

        Returns ``(nodes, clearances, edges, root, branches)``.

        :param outer: Outer boundary polygon (CCW).
        :param holes: List of hole polygons (CW). Defaults to [].
        :param tool_radius: Minimum clearance; narrower branches are pruned.
        :param sampling_spacing: Sampling density for boundary + Steiner grid.
                                Smaller = finer MAT.

        :returns:
            ``(nodes, clearances, edges, root, branches)`` where

            * **nodes** — ``list[(x, y)]`` of medial axis vertex positions.
            * **clearances** — ``list[float]`` inscribed circle radius per node.
            * **edges** — ``list[(int, int)]`` tree edges (parent, child).
            * **root** — ``int`` index of the maximum-clearance node.
            * **branches** — ``list[list[int]]`` each branch is a node-index path
              from a junction to another junction or leaf.
        """  # noqa: E501
    "#,
    module = "raygeo.geo.algo.medial_axis"
)]
#[pyfunction(name = "compute_medial_axis")]
#[pyo3(signature = (
    outer,
    holes = None,
    tool_radius = 1.0,
    sampling_spacing = 1.0,
))]
fn compute_medial_axis_py(
    outer: Vec<(f64, f64)>,
    holes: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    sampling_spacing: f64,
) -> PyResult<PyMaResult> {
    let outer_pts: Vec<Point> =
        outer.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let holes_pts: Vec<Vec<Point>> = holes
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let ma = rust_ma::compute_medial_axis(
        &outer_pts,
        &holes_pts,
        tool_radius,
        sampling_spacing,
    )
    .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

    let nodes: Vec<(f64, f64)> =
        ma.nodes.iter().map(|n| (n.point.x, n.point.y)).collect();
    let clearances: Vec<f64> = ma.nodes.iter().map(|n| n.clearance).collect();
    let edges: Vec<(usize, usize)> =
        ma.edges.iter().map(|&(a, b)| (a, b)).collect();
    let branches: Vec<Vec<usize>> =
        ma.branches.iter().map(|b| b.nodes.clone()).collect();

    Ok((nodes, clearances, edges, ma.root, branches))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def mat_path(
        outer: collections.abc.Sequence[tuple[float, float]],
        from_pt: tuple[float, float],
        to_pt: tuple[float, float],
        holes: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 1.0,
        sampling_spacing: float = 1.0,
    ) -> list[tuple[float, float]] | None:
        """Find a path between two points using the Medial Axis.

        Computes the MAT of the pocket defined by *outer* and *holes*,
        then finds the shortest-topology path between *from_pt* and
        *to_pt* along the medial axis graph.

        :param outer: Outer boundary polygon (CCW).
        :param from_pt: Start point (x, y).
        :param to_pt: End point (x, y).
        :param holes: List of hole polygons (CW). Defaults to [].
        :param tool_radius: Minimum clearance for MAT pruning.
        :param sampling_spacing: Boundary sampling density.
        :returns: List of (x, y) waypoints along the path, or None.
        """  # noqa: E501
    "#,
    module = "raygeo.geo.algo.medial_axis"
)]
#[pyfunction(name = "mat_path")]
#[pyo3(signature = (
    outer,
    from_pt,
    to_pt,
    holes = None,
    tool_radius = 1.0,
    sampling_spacing = 1.0,
))]
fn mat_path_py(
    outer: Vec<(f64, f64)>,
    from_pt: (f64, f64),
    to_pt: (f64, f64),
    holes: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    sampling_spacing: f64,
) -> PyResult<Option<Vec<(f64, f64)>>> {
    let outer_pts: Vec<Point> =
        outer.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let holes_pts: Vec<Vec<Point>> = holes
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let ma = rust_ma::compute_medial_axis(
        &outer_pts,
        &holes_pts,
        tool_radius,
        sampling_spacing,
    )
    .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

    let path = rust_ma::mat_path(
        &ma,
        Point::new(from_pt.0, from_pt.1),
        Point::new(to_pt.0, to_pt.1),
    );

    Ok(path.map(|pts| pts.into_iter().map(|p| (p.x, p.y)).collect()))
}
