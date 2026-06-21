pyo3_stub_gen::module_doc!("raygeo.geo.algo.fillet", "{}", MODULE_DOC_FILLET);

pub(crate) const MODULE_DOC_FILLET: &str = "\
Pure-geometry fillet operations.

Domain-neutral utilities for creating circular fillet arcs,
appending them to polylines, and trimming to safe spans.

* ``create_fillet_polyline`` — circular arc tangent to a direction.
* ``append_end_fillets`` — fillet both ends of an open polyline.
* ``trim_to_safe_fillet_span`` — longest sub-span whose end fillets avoid obstacles.
";

use crate::geo::algo::fillet;
use crate::geo::shape::arc::get_polyline_turn_sign;
use crate::geo::shape::polygon::trim_polyline_at;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "fillet")?;
    m.setattr("__doc__", MODULE_DOC_FILLET)?;

    register_functions!(
        m,
        create_fillet_polyline_py,
        append_end_fillets_py,
        trim_to_safe_fillet_span_py,
        fillet_arc_ends_py,
        find_safe_sweep_end_py,
    );

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    def create_fillet_polyline(
        p: tuple[float, float],
        dir: tuple[float, float],
        radius: float,
        sweep_angle: float,
        side: float,
        reverse: bool,
    ) -> tuple[tuple[float, float], list[tuple[float, float]]]:
        """Create a circular fillet arc tangent to *dir* at *p*.

        ``side`` selects the offset side (+1 = left of *dir*, -1 = right).
        When ``reverse`` is ``True`` the arc curls back opposite to *dir*.

        :param p: Start point (x, y).
        :param dir: Tangent direction vector (dx, dy).
        :param radius: Fillet radius.
        :param sweep_angle: Arc sweep angle in radians.
        :param side: Offset side (+1 left, -1 right).
        :param reverse: Whether the arc is reversed.
        :returns: ``(center, polyline)`` — arc centre and fillet vertices.
        """
"#,
    module = "raygeo.geo.algo.fillet"
)]
#[pyfunction(name = "create_fillet_polyline")]
fn create_fillet_polyline_py(
    p: (f64, f64),
    dir: (f64, f64),
    radius: f64,
    sweep_angle: f64,
    side: f64,
    reverse: bool,
) -> ((f64, f64), Vec<(f64, f64)>) {
    let (c, polyline) = fillet::create_fillet_polyline(
        Point::new(p.0, p.1),
        Point::new(dir.0, dir.1),
        radius,
        sweep_angle,
        side,
        reverse,
    );
    let pts: Vec<(f64, f64)> =
        polyline.into_iter().map(|p| (p.x, p.y)).collect();
    ((c.x, c.y), pts)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def append_end_fillets(
        polyline: collections.abc.Sequence[tuple[float, float]],
        radius: float,
        sweep_angle: float,
        side: float,
    ) -> list[tuple[float, float]]:
        """Append fillet arcs to both ends of an open polyline.

        A reversed fillet is added at the start and a forward fillet at
        the end, producing a smooth rounded path.

        :param polyline: Input open polyline.
        :param radius: Fillet radius.
        :param sweep_angle: Arc sweep angle in radians.
        :param side: Offset side (+1 left, -1 right).
        :returns: Full polyline with fillets.
        """
"#,
    module = "raygeo.geo.algo.fillet"
)]
#[pyfunction(name = "append_end_fillets")]
fn append_end_fillets_py(
    polyline: Vec<(f64, f64)>,
    radius: f64,
    sweep_angle: f64,
    side: f64,
) -> Vec<(f64, f64)> {
    let pts: Vec<Point> = polyline
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    fillet::append_end_fillets(&pts, radius, sweep_angle, side)
        .into_iter()
        .map(|p| (p.x, p.y))
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def trim_to_safe_fillet_span(
        polyline: collections.abc.Sequence[tuple[float, float]],
        outer_boundary: collections.abc.Sequence[tuple[float, float]],
        inner_obstacles: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        radius: float = 3.0,
        margin: float = 0.0,
    ) -> tuple[tuple[float, float], tuple[float, float]] | None:
        """Find the longest sub-span whose end fillets avoid obstacles.

        Shortens from each end until the sweep is clear.
        Returns ``(enter, exit)`` or ``None``.

        :param polyline: Open polyline to trim.
        :param outer_boundary: Outer boundary polygon.
        :param inner_obstacles: List of obstacle polygons (default []).
        :param radius: Fillet radius (default 3.0).
        :param margin: Extra clearance past tangency (default 0.0).
        """
"#,
    module = "raygeo.geo.algo.fillet"
)]
#[pyfunction(name = "trim_to_safe_fillet_span")]
#[pyo3(signature = (polyline, outer_boundary, inner_obstacles = None, radius = 3.0, margin = 0.0))]
fn trim_to_safe_fillet_span_py(
    polyline: Vec<(f64, f64)>,
    outer_boundary: Vec<(f64, f64)>,
    inner_obstacles: Option<Vec<Vec<(f64, f64)>>>,
    radius: f64,
    margin: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let polyline_pts: Vec<Point> = polyline
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let boundary_pts: Vec<Point> = outer_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let obstacles: Vec<Vec<Point>> = inner_obstacles
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    fillet::trim_to_safe_fillet_span(
        &polyline_pts,
        &boundary_pts,
        &obstacles,
        radius,
        margin,
    )
    .map(|(a, b)| ((a.x, a.y), (b.x, b.y)))
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
    module = "raygeo.geo.algo.fillet"
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
        arc.iter().map(|&(x, y)| Point::new(x, y)).collect();
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let Some((enter, exit)) = fillet::trim_to_safe_fillet_span(
        &arc_pts,
        &boundary,
        &islands_pts,
        tool_radius,
        wall_margin,
    ) else {
        return arc;
    };
    let trimmed = trim_polyline_at(&arc_pts, enter, exit);
    if trimmed.len() < 3 {
        return arc;
    }
    let side = get_polyline_turn_sign(&arc_pts);
    fillet::append_end_fillets(
        &trimmed,
        tool_radius,
        std::f64::consts::FRAC_PI_2,
        side,
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
    module = "raygeo.geo.algo.fillet"
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
    fillet::trim_to_safe_fillet_span(
        &arc_pts,
        &boundary,
        &islands_pts,
        tool_radius,
        wall_margin,
    )
    .map(|(a, b)| ((a.x, a.y), (b.x, b.y)))
}
