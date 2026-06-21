use crate::ops::assembly::hsm;
use crate::ops::state::State;
use crate::python::geo::algo::cleared_area::ClearedArea as PyClearedArea;
use crate::python::ops::PyOps;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let hsm_mod = PyModule::new(assembly_mod.py(), "hsm")?;
    register_functions!(
        hsm_mod,
        adaptive_entry_py,
        adaptive_wavefronts_py,
        link_arcs_to_ops_py,
        adaptive_peeling_py,
    );
    assembly_mod.add_submodule(&hsm_mod)?;

    let sys_modules = assembly_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.hsm", &hsm_mod)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

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
        cut_feed_rate: int = 1200,
    ) -> tuple[raygeo.ops.Ops, list[list[tuple[float, float]]]]:
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
        :param cut_feed_rate: Feed rate for the entry path (default 1200).
        :returns: ``(ops, cleared_polygons)`` where *ops* is an ``Ops``
                  with the entry toolpath and *cleared_polygons* is a list
                  of polygons to add to the ``ClearedArea``.
        """
"#,
    module = "raygeo.ops.assembly.hsm"
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
    cut_feed_rate = 1200,
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
    cut_feed_rate: i32,
) -> (PyOps, Vec<Vec<(f64, f64)>>) {
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

    let cut_state = State {
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };

    let result = hsm::adaptive_entry(&opts, &cut_state);

    let cleared_polys: Vec<Vec<(f64, f64)>> = result
        .cleared_polygons
        .into_iter()
        .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
        .collect();

    (PyOps { inner: result.ops }, cleared_polys)
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
        cut_feed_rate: int = 1200,
    ) -> raygeo.ops.Ops:
        """Inside-out adaptive wavefronts.

        Starting from the *cleared* state, each iteration expands the
        cleared boundary outward by *step_over*, clips to the valid tool
        area (pocket boundary offset inward by *tool_radius*, with
        islands excluded), and adds the result back to *cleared*.
        The loop terminates when the newly added area drops below
        *area_tolerance*.

        Each ring fragment is emitted as ``MoveTo`` + ``LineTo`` at
        height *z* with *cut_feed_rate* applied.

        :param cleared: ``ClearedArea`` instance (mutated in place).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial expansion per iteration (default 2.0).
        :param z: Z height for generated commands (default 0.0).
        :param area_tolerance: Minimum area increase to continue (default 1.0).
        :param cut_feed_rate: Feed rate for cutting moves (default 1200).
        :returns: Ops with wavefront cutting commands.
        """
"#,
    module = "raygeo.ops.assembly.hsm"
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
    cut_feed_rate = 1200,
))]
#[allow(clippy::too_many_arguments)]
fn adaptive_wavefronts_py(
    cleared: &mut PyClearedArea,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    z: f64,
    area_tolerance: f64,
    cut_feed_rate: i32,
) -> PyOps {
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
    };

    let cut_state = State {
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };

    let ops = hsm::adaptive_wavefronts(&mut cleared.inner, &opts, &cut_state);

    PyOps { inner: ops }
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def link_arcs_to_ops(
        arcs: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        uncleared: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        cut_z: float = -1.0,
        safe_z: float = 5.0,
        mat: tuple[list[tuple[float, float]], list[tuple[int, int]]] | None = None,
        safe_margin: float = 0.0,
        smoothing_amount: int = 50,
        preserve_order: bool = False,
        cut_feed_rate: int = 1200,
        travel_rapid_rate: int = 8000,
    ) -> raygeo.ops.Ops:
        """Link filleted arcs into an Ops with MAT-routed travel.

        Consecutive arcs are joined by travel segments (MoveTo) at
        *safe_z*.  When the direct line would cross (or pass within
        *safe_margin* of) any polygon in *uncleared*, the connection
        uses the Medial Axis to route around obstacles, then smoothed.

        Cutting arcs are emitted as LineTo at *cut_z* with
        *cut_feed_rate*; travel links as MoveTo at *safe_z* with
        *travel_rapid_rate*.

        :param arcs: Sequence of arcs (each a list of (x, y) points).
        :param uncleared: Areas to avoid during travel (default []).
        :param cut_z: Cutting Z height (default -1.0).
        :param safe_z: Safe (rapid) Z height (default 5.0).
        :param mat: Optional ``(nodes, edges)`` tuple from
                    ``compute_medial_axis`` for obstacle-aware routing.
        :param safe_margin: Minimum distance from uncleared polygons for
                            a direct travel line to be considered safe
                            (default 0 = no margin check).
        :param smoothing_amount: Gaussian smoothing amount (0-200) applied
                                  to MAT-routed travel (default 50).
        :param preserve_order: Keep arc order as given instead of
                               nearest-neighbour reordering (default False).
        :param cut_feed_rate: Feed rate for cutting moves (default 1200).
        :param travel_rapid_rate: Rapid rate for travel moves (default 8000).
        :returns: Ops with cutting LineTo and travel MoveTo commands.
        """
"#,
    module = "raygeo.ops.assembly.hsm"
)]
#[pyfunction(name = "link_arcs_to_ops")]
#[pyo3(signature = (
    arcs,
    uncleared = None,
    cut_z = -1.0,
    safe_z = 5.0,
    mat = None,
    safe_margin = 0.0,
    smoothing_amount = 50,
    preserve_order = false,
    cut_feed_rate = 1200,
    travel_rapid_rate = 8000,
))]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn link_arcs_to_ops_py(
    arcs: Vec<Vec<(f64, f64)>>,
    uncleared: Option<Vec<Vec<(f64, f64)>>>,
    cut_z: f64,
    safe_z: f64,
    mat: Option<(Vec<(f64, f64)>, Vec<(usize, usize)>)>,
    safe_margin: f64,
    smoothing_amount: i32,
    preserve_order: bool,
    cut_feed_rate: i32,
    travel_rapid_rate: i32,
) -> PyOps {
    use crate::geo::algo::medial_axis::{MaNode, MedialAxis};

    let arcs_pts: Vec<Vec<Point>> = arcs
        .into_iter()
        .map(|a| a.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let uncleared_pts: Vec<Vec<Point>> = uncleared
        .unwrap_or_default()
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

    let cut_state = State {
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };
    let travel_state = State {
        rapid_rate: Some(travel_rapid_rate),
        ..Default::default()
    };

    let ops = hsm::link_arcs_to_ops(
        &arcs_pts,
        &uncleared_pts,
        mat_opt.as_ref(),
        cut_z,
        safe_z,
        safe_margin,
        smoothing_amount,
        preserve_order,
        &cut_state,
        &travel_state,
    );

    PyOps { inner: ops }
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
        cut_z: float = -5.0,
        safe_z: float = 2.0,
        wall_margin: float = 0.0,
        travel_smoothing: int = 50,
        area_tolerance: float = 1.0,
        cut_feed_rate: int = 1200,
        travel_rapid_rate: int = 8000,
    ) -> raygeo.ops.Ops:
        """Run the peeling (D-cut) clearing strategy and return an Ops.

        Starting from the *cleared* state (mutated in place), each
        iteration expands the cleared boundary outward by *step_over*,
        clips to the valid tool area, computes crescent-shaped "bites",
        and generates a D-cut for each bite.  The individual passes are
        linked into a single Ops: each cutting arc at *cut_z* (LineTo
        with *cut_feed_rate*) followed by a travel segment at *safe_z*
        (MoveTo with *travel_rapid_rate*).  The Medial Axis of the
        pocket is used to route travel around obstacles.

        :param cleared: ``ClearedArea`` instance (mutated in place).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial expansion per iteration (default 2.0).
        :param cut_z: Cutting Z height (default -5.0).
        :param safe_z: Retract Z height for travel segments (default 2.0).
        :param wall_margin: Extra clearance between tool sweep and walls
                             (default 0.0).
        :param travel_smoothing: Gaussian smoothing for MAT-routed travel
                                  (default 50).
        :param area_tolerance: Convergence tolerance in square mm
                                (default 1.0).
        :param cut_feed_rate: Feed rate for cutting moves (default 1200).
        :param travel_rapid_rate: Rapid rate for travel moves (default 8000).
        :returns: Ops with cutting and travel commands.
        """
"#,
    module = "raygeo.ops.assembly.hsm"
)]
#[pyfunction(name = "adaptive_peeling")]
#[pyo3(signature = (
    cleared,
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 2.0,
    cut_z = -5.0,
    safe_z = 2.0,
    wall_margin = 0.0,
    travel_smoothing = 50,
    area_tolerance = 1.0,
    cut_feed_rate = 1200,
    travel_rapid_rate = 8000,
))]
#[allow(clippy::too_many_arguments)]
fn adaptive_peeling_py(
    cleared: &mut PyClearedArea,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    cut_z: f64,
    safe_z: f64,
    wall_margin: f64,
    travel_smoothing: i32,
    area_tolerance: f64,
    cut_feed_rate: i32,
    travel_rapid_rate: i32,
) -> PyOps {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let cut_state = State {
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };
    let travel_state = State {
        rapid_rate: Some(travel_rapid_rate),
        ..Default::default()
    };

    let ops = hsm::adaptive_peeling(
        &mut cleared.inner,
        &boundary,
        &islands_pts,
        tool_radius,
        step_over,
        cut_z,
        safe_z,
        wall_margin,
        travel_smoothing,
        area_tolerance,
        &cut_state,
        &travel_state,
    );

    PyOps { inner: ops }
}
