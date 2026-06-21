pyo3_stub_gen::module_doc!("raygeo.geo.algo.hsm", "{}", MODULE_DOC_HSM);

pub(crate) const MODULE_DOC_HSM: &str = "\
HSM cutting-arc geometry primitives.

Pure geometric helpers for adaptive clearing.

* ``find_cutting_arc`` — extract the outer (cutting) arc from a bite.
* ``fillet_arc_ends`` — round both ends of a cutting arc.
* ``find_safe_sweep_end`` — find the longest safe sub-arc.

For motion assembly (entry strategy, wavefront expansion, peeling,
arc linking) see ``raygeo.ops.assembly.hsm``.
";

use crate::geo::algo::hsm;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "hsm")?;
    m.setattr("__doc__", MODULE_DOC_HSM)?;

    register_functions!(
        m,
        find_cutting_arc_py,
        fillet_arc_ends_py,
        find_safe_sweep_end_py,
    );

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_cutting_arc(
        bite: collections.abc.Sequence[tuple[float, float]],
        cleared_fragments: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
    ) -> list[tuple[float, float]] | None:
        """Extract the cutting arc (outer) vertices from a bite polygon.

        The cutting arc is the longest contiguous run of bite vertices
        that lie *outside* all cleared fragments.

        :param bite: Bite polygon vertices.
        :param cleared_fragments: List of cleared-area polygons.
        :returns: The cutting arc polyline, or None if degenerate.
        """
"#,
    module = "raygeo.geo.algo.hsm"
)]
#[pyfunction(name = "find_cutting_arc")]
fn find_cutting_arc_py(
    bite: Vec<(f64, f64)>,
    cleared_fragments: Vec<Vec<(f64, f64)>>,
) -> Option<Vec<(f64, f64)>> {
    let bite_pts: Vec<Point> =
        bite.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let cleared: Vec<Vec<Point>> = cleared_fragments
        .into_iter()
        .map(|poly| poly.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    hsm::find_cutting_arc(&bite_pts, &cleared)
        .map(|(arc, _, _)| arc.into_iter().map(|p| (p.x, p.y)).collect())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def fillet_arc_ends(
        arc: collections.abc.Sequence[tuple[float, float]],
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        wall_margin: float = 0.0,
    ) -> list[tuple[float, float]]:
        """Round both ends of a cutting arc with quarter-circle fillets.

        The arc is trimmed to the longest sub-arc whose tool sweep
        (arc + end fillets of *tool_radius*) does not collide with
        *pocket_boundary* or *islands*.  A 90° fillet of *tool_radius*
        is then appended at each end.

        :param arc: Cutting arc vertices (open polyline).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool / fillet radius in mm (default 3.0).
        :param wall_margin: Extra clearance past tangency (default 0.0).
        :returns: Filleted arc as an open polyline.
        """
"#,
    module = "raygeo.geo.algo.hsm"
)]
#[pyfunction(name = "fillet_arc_ends")]
#[pyo3(signature = (arc, pocket_boundary, islands = None, tool_radius = 3.0, wall_margin = 0.0))]
fn fillet_arc_ends_py(
    arc: Vec<(f64, f64)>,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    wall_margin: f64,
) -> Vec<(f64, f64)> {
    let arc_pts: Vec<Point> =
        arc.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    hsm::fillet_arc_ends(
        &arc_pts,
        &boundary,
        &islands_pts,
        tool_radius,
        wall_margin,
    )
    .into_iter()
    .map(|p| (p.x, p.y))
    .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def find_safe_sweep_end(
        arc: collections.abc.Sequence[tuple[float, float]],
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        wall_margin: float = 0.0,
    ) -> tuple[tuple[float, float], tuple[float, float]] | None:
        """Find the longest safe sub-arc by iterative sweep shortening.

        Returns the two points ``(enter, exit)`` delimiting the longest
        sub-arc of *arc* whose tool sweep (arc + end fillets of
        *tool_radius*) does not collide with *pocket_boundary* or
        *islands*.  Shortens from each end until the sweep is clear.
        Returns ``None`` when no usable safe sub-arc remains.

        :param arc: Cutting arc vertices (open polyline).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param wall_margin: Extra clearance past tangency (default 0.0).
        """
"#,
    module = "raygeo.geo.algo.hsm"
)]
#[pyfunction(name = "find_safe_sweep_end")]
#[pyo3(signature = (arc, pocket_boundary, islands = None, tool_radius = 3.0, wall_margin = 0.0))]
fn find_safe_sweep_end_py(
    arc: Vec<(f64, f64)>,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    wall_margin: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let arc_pts: Vec<Point> =
        arc.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    hsm::find_safe_sweep_end(
        &arc_pts,
        &boundary,
        &islands_pts,
        tool_radius,
        wall_margin,
    )
    .map(|(a, b)| ((a.x, a.y), (b.x, b.y)))
}
