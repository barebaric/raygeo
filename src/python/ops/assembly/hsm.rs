use crate::ops::assembly::hsm;
use crate::ops::state::State;
use crate::python::geo::algo::cleared_area::ClearedArea as PyClearedArea;
use crate::python::ops::PyOps;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction};

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let hsm_mod = PyModule::new(assembly_mod.py(), "hsm")?;
    register_functions!(
        hsm_mod,
        adaptive_entry_py,
        adaptive_wavefronts_py,
        split_ordered_wavefronts_py,
        link_arcs_to_ops_py,
        adaptive_peeling_py,
        find_cutting_arc_py,
    );
    hsm_mod.add_class::<PyWavefrontGraph>()?;
    assembly_mod.add_submodule(&hsm_mod)?;

    let sys_modules = assembly_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.hsm", &hsm_mod)?;

    Ok(())
}

/// Parent tree returned by [`split_ordered_wavefronts`].
///
/// Nodes are individual bite polygons, identified by a *global index*
/// computed from `bite_offsets`:
///
/// ```text
/// global = bite_offsets[pass] + local_index_within_pass
/// ```
///
/// Each bite has exactly one parent (the nearest previous-pass bite
/// sharing boundary), forming a tree.  `visit_order` lists global bite
/// indices in DFS traversal order.
#[gen_stub_pyclass(module = "raygeo.ops.assembly.hsm")]
#[pyclass(skip_from_py_object)]
#[derive(Debug, Clone)]
struct PyWavefrontGraph {
    /// Cutting arcs in DFS visit order.
    #[pyo3(get)]
    arcs: Vec<Vec<(f64, f64)>>,
    /// Pass index for each arc in `arcs` (same length, same order).
    #[pyo3(get)]
    arc_passes: Vec<usize>,
    /// Per-pass bite polygons: `bite_polys[pass][local]`.
    #[pyo3(get)]
    bite_polys: Vec<Vec<Vec<(f64, f64)>>>,
    /// Per-bite arc indices into `arcs` (DFS order):
    /// `bite_arcs[global_bite] = [arc_idx, ...]`.
    #[pyo3(get)]
    bite_arcs: Vec<Vec<usize>>,
    /// `parent[global]` = parent bite index, or `None` for roots.
    #[pyo3(get)]
    parent: Vec<Option<usize>>,
    /// Pass start offsets for global↔local conversion.
    #[pyo3(get)]
    bite_offsets: Vec<usize>,
    /// Global bite indices in the order visited by DFS.
    #[pyo3(get)]
    visit_order: Vec<usize>,
    /// V-junction-split sub-segments from each arc, flattened in arc order.
    #[pyo3(get)]
    segments: Vec<Vec<(f64, f64)>>,
    /// Outward normal (unit vector) for each segment in `segments`.
    #[pyo3(get)]
    segment_directions: Vec<(f64, f64)>,
    /// For each arc in `arcs`, indices into `segments`.
    #[pyo3(get)]
    arc_segments: Vec<Vec<usize>>,
}

impl From<hsm::WavefrontGraph> for PyWavefrontGraph {
    fn from(g: hsm::WavefrontGraph) -> Self {
        let arcs = g
            .arcs
            .into_iter()
            .map(|arc| arc.into_iter().map(|p| (p.x, p.y)).collect())
            .collect();
        let bite_polys = g
            .bite_polys
            .into_iter()
            .map(|pass| {
                pass.into_iter()
                    .map(|poly| poly.into_iter().map(|p| (p.x, p.y)).collect())
                    .collect()
            })
            .collect();
        let segments = g
            .segments
            .into_iter()
            .map(|seg| seg.into_iter().map(|p| (p.x, p.y)).collect())
            .collect();
        let segment_directions = g
            .segment_directions
            .into_iter()
            .map(|v| (v.x, v.y))
            .collect();
        PyWavefrontGraph {
            arcs,
            arc_passes: g.arc_passes,
            bite_polys,
            bite_arcs: g.bite_arcs,
            parent: g.parent,
            bite_offsets: g.bite_offsets,
            visit_order: g.visit_order,
            segments,
            segment_directions,
            arc_segments: g.arc_segments,
        }
    }
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
        cut_power: float = 1.0,
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
        :param cut_power: Laser power for the entry path (0.0-1.0, default 1.0).
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
    cut_power = 1.0,
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
    cut_power: f64,
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
        power: cut_power,
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
        cut_power: float = 1.0,
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
        :param cut_power: Laser power for cutting moves (0.0-1.0, default 1.0).
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
    cut_power = 1.0,
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
    cut_power: f64,
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
        power: cut_power,
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
        cut_power: float = 1.0,
        cleared: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
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
        :param cut_power: Laser power for cutting moves (0.0-1.0, default 1.0).
        :param cleared: Cleared-area polygons.  When provided the MAT is
                        trimmed to these polygons before routing, ensuring
                        travel only goes through already-machined territory
                        (default None = no trimming).
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
    cut_power = 1.0,
    cleared = None,
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
    cut_power: f64,
    cleared: Option<Vec<Vec<(f64, f64)>>>,
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
    let cleared_pts: Vec<Vec<Point>> = cleared
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
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };
    let travel_state = State {
        rapid_rate: Some(travel_rapid_rate),
        ..Default::default()
    };

    let cleared_ref: Option<&[Vec<Point>]> = if cleared_pts.is_empty() {
        None
    } else {
        Some(&cleared_pts)
    };

    let ops = hsm::link_arcs_to_ops(
        &arcs_pts,
        &uncleared_pts,
        mat_opt.as_ref(),
        cleared_ref,
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

    def split_ordered_wavefronts(
        cleared: raygeo.geo.algo.cleared_area.ClearedArea,
        step_over: float,
        valid_area: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        simplify_tol: float,
        entry: tuple[float, float],
    ) -> raygeo.ops.assembly.hsm.PyWavefrontGraph:
        """Generate, split, and order cutting arcs in one pass.

        Builds a directed bite graph during the clearing loop: each
        bite from pass N+1 that shares boundary with a pass-N bite
        becomes its child.  DFS with merge constraints produces the
        processing order.

        :param cleared: ``ClearedArea`` instance (mutated in place).
        :param step_over: Lateral step-over in mm.
        :param valid_area: Valid tool-centre region polygons.
        :param simplify_tol: Tolerance for frontier simplification.
        :param entry: Entry point (cleared centroid).
        :returns: ``PyWavefrontGraph`` carrying the ordered arcs and
                  the underlying directed bite graph.
        """
    "#,
    module = "raygeo.ops.assembly.hsm"
)]
#[pyfunction(name = "split_ordered_wavefronts")]
fn split_ordered_wavefronts_py(
    cleared: &mut PyClearedArea,
    step_over: f64,
    valid_area: Vec<Vec<(f64, f64)>>,
    simplify_tol: f64,
    entry: (f64, f64),
) -> PyWavefrontGraph {
    let valid: Vec<Vec<Point>> = valid_area
        .into_iter()
        .map(|v| v.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let graph = hsm::split_ordered_wavefronts(
        &mut cleared.inner,
        step_over,
        &valid,
        simplify_tol,
        Point::new(entry.0, entry.1),
    );
    graph.into()
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
        cut_feed_rate: int = 1200,
        travel_rapid_rate: int = 8000,
        cut_power: float = 1.0,
    ) -> raygeo.ops.Ops:
        """Run the peeling clearing strategy and return an Ops.

        Generates, splits, and orders cutting arcs via a directed bite
        graph, then fillets and links them into Ops with MAT-routed
        travel segments.

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
        :param cut_feed_rate: Feed rate for cutting moves (default 1200).
        :param travel_rapid_rate: Rapid rate for travel moves (default 8000).
        :param cut_power: Laser power for cutting moves (0.0-1.0, default 1.0).
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
    cut_feed_rate = 1200,
    travel_rapid_rate = 8000,
    cut_power = 1.0,
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
    cut_feed_rate: i32,
    travel_rapid_rate: i32,
    cut_power: f64,
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
        power: cut_power,
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
        &cut_state,
        &travel_state,
    );

    PyOps { inner: ops }
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
    module = "raygeo.ops.assembly.hsm"
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
