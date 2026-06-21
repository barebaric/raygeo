pyo3_stub_gen::module_doc!("raygeo.geo.algo.hsm", "{}", MODULE_DOC_HSM);

pub(crate) const MODULE_DOC_HSM: &str = "\
HSM (High-Speed Machining) adaptive clearing.

* ``adaptive_entry`` — find the optimal entry pole, then helix + spiral
  (wide area) or zigzag ramp (tight slot).
* ``adaptive_wavefronts`` — inside-out expansion loop: each iteration
  expands the cleared boundary outward by ``step_over``, clips to the
  valid tool area, applies a minimum-curvature filter, and updates the
  cleared state until convergence.
* ``adaptive_peeling`` — inside-out D-biting (peeling): each iteration
  expands the cleared boundary, clips to the valid tool area, computes
  crescent-shaped ``bites``, and traces the full perimeter of each bite
  before incorporating it into the cleared state.
";

use crate::geo::algo::hsm;
use crate::python::geo::algo::cleared_area::ClearedArea as PyClearedArea;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

type PyAdaptiveEntryResult = (Vec<(f64, f64, f64)>, Vec<Vec<(f64, f64)>>);

pub fn register(algo_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = algo_mod.py();
    let m = PyModule::new(py, "hsm")?;
    m.setattr("__doc__", MODULE_DOC_HSM)?;

    register_functions!(
        m,
        adaptive_entry_py,
        adaptive_wavefronts_py,
        adaptive_peeling_py,
        find_cutting_arc_py,
        fillet_arc_ends_py,
        find_safe_sweep_end_py,
        link_filleted_arcs_py,
    );

    algo_mod.add_submodule(&m)?;
    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def adaptive_entry(
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        safe_z: float = 2.0,
        target_z: float = -5.0,
        plunge_pitch: float = 1.0,
        safe_margin: float = 1.0,
        angular_step: float = 0.1,
    ) -> tuple[list[tuple[float, float, float]], list[list[tuple[float, float]]]]:
        """Fast central clearing entry.

        Finds the optimal entry pole using ``find_largest_circle``, then
        generates either a helix->spiral (wide area) or zigzag ramp
        (tight slot).

        The returned *cleared_polygons* should be inserted into a
        ``ClearedArea`` via ``add_cleared_polygons``.

        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial step-over per spiral revolution (default 2.0).
        :param safe_z: Safe (retract) Z height (default 2.0).
        :param target_z: Target cutting depth (default -5.0).
        :param plunge_pitch: Vertical descent per helix revolution (default 1.0).
        :param safe_margin: Extra margin from tool edge to boundary (default 1.0).
        :param angular_step: Angular step in radians for path vertices (default 0.1).
        :returns: ``(toolpath, cleared_polygons)`` where *toolpath* is a list
                  of (x, y, z) points and *cleared_polygons* is a list of
                  polygons (each a list of (x, y) points) to add to the
                  ``ClearedArea``.
        :complexity: O(n) for the spiral/helix generation, O(m log m) for
                     ``find_largest_circle`` where m is the polygon vertex count.
        """
"#,
    module = "raygeo.geo.algo.hsm"
)]
#[pyfunction(name = "adaptive_entry")]
#[pyo3(signature = (
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    safe_z = 2.0,
    target_z = -5.0,
    plunge_pitch = 1.0,
    safe_margin = 1.0,
    angular_step = 0.1,
))]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn adaptive_entry_py(
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    safe_z: f64,
    target_z: f64,
    plunge_pitch: f64,
    safe_margin: f64,
    angular_step: f64,
) -> PyAdaptiveEntryResult {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = hsm::AdaptiveEntryOptions {
        pocket_boundary: boundary,
        islands: islands_pts,
        tool_radius,
        step_over,
        safe_z,
        target_z,
        plunge_pitch,
        safe_margin,
        angular_step,
    };

    let result = hsm::adaptive_entry(&opts);

    let toolpath: Vec<(f64, f64, f64)> = result
        .toolpath
        .into_iter()
        .map(|p| (p.x, p.y, p.z))
        .collect();
    let cleared_polys: Vec<Vec<(f64, f64)>> = result
        .cleared_polygons
        .into_iter()
        .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
        .collect();

    (toolpath, cleared_polys)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def adaptive_wavefronts(
        cleared: raygeo.geo.algo.cleared_area.ClearedArea,
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        z: float = 0.0,
        area_tolerance: float = 1.0,
    ) -> list[list[tuple[float, float, float]]]:
        """Inside-out adaptive wavefronts.

        Starting from the *cleared* state, each iteration expands the
        cleared boundary outward by *step_over*, clips to the valid tool
        area (pocket boundary offset inward by *tool_radius*, with
        islands excluded), and adds the result back to *cleared*.
        The loop terminates when the newly added area drops below
        *area_tolerance*.

        :param cleared: ``ClearedArea`` instance (mutated in place).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial expansion per iteration (default 2.0).
        :param z: Z height for generated toolpath points (default 0.0).
        :param area_tolerance: Minimum area increase to continue (default 1.0).
        :returns: List of toolpaths — one ``list[(x, y, z)]`` per iteration.
        :complexity: O(i * (n * m + p log p)) where i = iterations, n = boundary
            vertices, m = cleared fragments, p = polygon vertices
        """
"#,
    module = "raygeo.geo.algo.hsm"
)]
#[pyfunction(name = "adaptive_wavefronts")]
#[pyo3(signature = (
    cleared,
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    z = 0.0,
    area_tolerance = 1.0,
))]
fn adaptive_wavefronts_py(
    cleared: &mut PyClearedArea,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    z: f64,
    area_tolerance: f64,
) -> Vec<Vec<(f64, f64, f64)>> {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = hsm::AdaptiveWavefrontOptions {
        pocket_boundary: boundary,
        islands: islands_pts,
        tool_radius,
        step_over,
        z,
        area_tolerance,
        safe_z: z,
        wall_margin: 0.0,
        mat: None,
    };

    let result = hsm::adaptive_wavefronts(&mut cleared.inner, &opts);

    result
        .toolpaths
        .into_iter()
        .map(|path| path.into_iter().map(|p| (p.x, p.y, p.z)).collect())
        .collect()
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def adaptive_peeling(
        cleared: raygeo.geo.algo.cleared_area.ClearedArea,
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        z: float = 0.0,
        safe_z: float | None = None,
        area_tolerance: float = 1.0,
        wall_margin: float = 0.0,
    ) -> list[tuple[float, float, float]]:
        """Inside-out adaptive peeling (D-biting).

        Starting from the *cleared* state, each iteration expands the
        cleared boundary outward by *step_over*, clips to the valid tool
        area, computes crescent-shaped "bites", and generates a D-cut
        for each bite.  The individual passes are linked into a single
        continuous toolpath: each cutting arc at *z* followed by a travel
        segment at *safe_z* to the next cut.  The Medial Axis of the
        pocket is used to route travel around obstacles.

        :param cleared: ``ClearedArea`` instance (mutated in place).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial expansion per iteration (default 2.0).
        :param z: Cutting Z height (default 0.0).
        :param safe_z: Retract Z height for travel segments (defaults to *z*,
                       meaning no lift).
        :param area_tolerance: Minimum area increase to continue (default 1.0).
        :param wall_margin: Extra clearance (mm) kept between the tool sweep
                            and the pocket wall / islands when trimming
                            cutting arcs.  ``0.0`` allows tangency
                            (default 0.0).
        :returns: Single continuous toolpath ``list[(x, y, z)]`` with
                  cutting arcs at *z* and travel at *safe_z*.
        """
"#,
    module = "raygeo.geo.algo.hsm"
)]
#[pyfunction(name = "adaptive_peeling")]
#[pyo3(signature = (
    cleared,
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    z = 0.0,
    safe_z = None,
    area_tolerance = 1.0,
    wall_margin = 0.0,
))]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn adaptive_peeling_py(
    cleared: &mut PyClearedArea,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    z: f64,
    safe_z: Option<f64>,
    area_tolerance: f64,
    wall_margin: f64,
) -> Vec<(f64, f64, f64)> {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let mut opts = hsm::AdaptiveWavefrontOptions {
        pocket_boundary: boundary,
        islands: islands_pts,
        tool_radius,
        step_over,
        z,
        safe_z: safe_z.unwrap_or(z),
        area_tolerance,
        wall_margin,
        mat: None,
    };

    let path = hsm::adaptive_peeling(&mut cleared.inner, &mut opts);
    path.into_iter().map(|p| (p.x, p.y, p.z)).collect()
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

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def link_filleted_arcs(
        arcs: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        uncleared: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        z: float = 0.0,
        safe_z: float = 2.0,
        mat: tuple[list[tuple[float, float]], list[tuple[int, int]]] | None = None,
        safe_margin: float = 0.0,
    ) -> list[tuple[float, float, float]]:
        """Link filleted arcs into a continuous 3-D polyline.

        Consecutive arcs are joined by a straight segment at *safe_z*.
        When the direct line would cross (or pass within *safe_margin*
        of) any polygon in *uncleared*, the connection uses the Medial
        Axis to route around obstacles.

        :param arcs: Sequence of filleted arcs (each a list of (x, y) points).
        :param uncleared: Areas to avoid during travel.
        :param z: Cutting height (default 0).
        :param safe_z: Safe (rapid) height (default 2).
        :param mat: Optional ``(nodes, edges)`` tuple from
                    ``compute_medial_axis``.  When provided, blocked
                    travel segments are routed through the MAT graph.
        :param safe_margin: Minimum distance from uncleared polygons for
                            a direct travel line to be considered safe
                            (default 0 = no check).  Set to *tool_radius*
                            to prevent near-misses.
        :returns: Single continuous 3-D polyline.
        """
"#,
    module = "raygeo.geo.algo.hsm"
)]
#[pyfunction(name = "link_filleted_arcs")]
#[pyo3(signature = (arcs, uncleared, z = 0.0, safe_z = 2.0, mat = None, safe_margin = 0.0))]
#[allow(clippy::type_complexity)]
fn link_filleted_arcs_py(
    arcs: Vec<Vec<(f64, f64)>>,
    uncleared: Vec<Vec<(f64, f64)>>,
    z: f64,
    safe_z: f64,
    mat: Option<(Vec<(f64, f64)>, Vec<(usize, usize)>)>,
    safe_margin: f64,
) -> Vec<(f64, f64, f64)> {
    use crate::geo::algo::medial_axis::{MaNode, MedialAxis};

    let arcs_pts: Vec<Vec<Point>> = arcs
        .into_iter()
        .map(|a| a.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let uncleared_pts: Vec<Vec<Point>> = uncleared
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let mat_opt: Option<MedialAxis> = mat.map(|(nodes, edges)| {
        let nodes_vec: Vec<MaNode> = nodes
            .into_iter()
            .map(|(x, y)| MaNode {
                point: Point::new(x, y),
                clearance: 0.0,
            })
            .collect();
        MedialAxis {
            nodes: nodes_vec,
            edges,
            root: 0,
            branches: Vec::new(),
        }
    });

    hsm::link_filleted_arcs(
        &arcs_pts,
        &uncleared_pts,
        z,
        safe_z,
        mat_opt.as_ref(),
        false,
        safe_margin,
    )
    .into_iter()
    .map(|p| (p.x, p.y, p.z))
    .collect()
}
