pyo3_stub_gen::module_doc!(
    "raygeo.geo.algo.morph_spiral",
    "{}",
    MODULE_DOC_MORPH_SPIRAL
);

pub(crate) const MODULE_DOC_MORPH_SPIRAL: &str = "\
MAT-driven morphing spiral generation.

* ``morph_spiral`` — full pipeline: offset, MAT, per-branch spiral, linking.
* ``morph_spiral_from_branch`` — generate a boustrophedon spiral for a
  single MAT branch (centerline + clearance profile).
";

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::geo::algo::morph_spiral as rust_ms;
use crate::types::Point;

type PyMorphResult = (Vec<(f64, f64, f64)>, Vec<Vec<(f64, f64, f64)>>);

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "morph_spiral")?;
    m.setattr("__doc__", MODULE_DOC_MORPH_SPIRAL)?;

    register_functions!(m, morph_spiral_py, morph_spiral_from_branch_py);

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def morph_spiral(
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        z: float = 0.0,
        sampling_spacing: float | None = None,
    ) -> tuple[list[tuple[float, float, float]], list[list[tuple[float, float, float]]]]:
        """Full morphing-spiral pipeline.

        Offsets the boundary by *tool_radius*, computes the medial axis
        transform, generates a boustrophedon spiral per branch, and links
        all branches into a single continuous toolpath.

        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial step-over between passes in mm (default 2.0).
        :param z: Z height for generated toolpath points (default 0.0).
        :param sampling_spacing: MAT sampling density (mm).
            Defaults to ``step_over × 0.5``.
        :returns: ``(toolpath, branch_paths)`` where *toolpath* is a single
            ``list[(x, y, z)]`` and *branch_paths* is per-branch paths.
        :raises RuntimeError: If MAT computation or spiral generation fails.
        """  # noqa: E501
    "#,
    module = "raygeo.geo.algo.morph_spiral"
)]
#[pyfunction(name = "morph_spiral")]
#[pyo3(signature = (
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    z = 0.0,
    sampling_spacing = None,
))]
fn morph_spiral_py(
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    z: f64,
    sampling_spacing: Option<f64>,
) -> PyResult<PyMorphResult> {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = rust_ms::MorphSpiralOptions {
        pocket_boundary: &boundary,
        islands: &islands_pts,
        tool_radius,
        step_over,
        z,
        sampling_spacing,
    };

    let result = rust_ms::morph_spiral(&opts)
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;

    let toolpath: Vec<(f64, f64, f64)> = result
        .toolpath
        .into_iter()
        .map(|p| (p.x, p.y, p.z))
        .collect();
    let branch_paths: Vec<Vec<(f64, f64, f64)>> = result
        .branches
        .into_iter()
        .map(|bp| bp.into_iter().map(|p| (p.x, p.y, p.z)).collect())
        .collect();

    Ok((toolpath, branch_paths))
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def morph_spiral_from_branch(
        points: collections.abc.Sequence[tuple[float, float]],
        clearances: collections.abc.Sequence[float],
        step_over: float,
        z: float = 0.0,
    ) -> list[tuple[float, float, float]]:
        """Generate a boustrophedon spiral for a single MAT branch.

        *points* is the centerline polyline (root→leaf).
        *clearances[i]* is the channel half-width at *points[i]*.

        :param points: Centerline polyline, root (high clearance) → leaf
                       (low clearance).
        :param clearances: Channel half-width at each point.
        :param step_over: Radial step-over between passes.
        :param z: Z height for generated points.
        :returns: ``list[(x, y, z)]`` — the continuous boustrophedon path.
        """  # noqa: E501
    "#,
    module = "raygeo.geo.algo.morph_spiral"
)]
#[pyfunction(name = "morph_spiral_from_branch")]
#[pyo3(signature = (
    points,
    clearances,
    step_over,
    z = 0.0,
))]
fn morph_spiral_from_branch_py(
    points: Vec<(f64, f64)>,
    clearances: Vec<f64>,
    step_over: f64,
    z: f64,
) -> Vec<(f64, f64, f64)> {
    let pts: Vec<Point> =
        points.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let path =
        rust_ms::morph_spiral_from_branch(&pts, &clearances, step_over, z);
    path.into_iter().map(|p| (p.x, p.y, p.z)).collect()
}
