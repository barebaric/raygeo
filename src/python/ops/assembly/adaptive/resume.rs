//! Python wrappers for the adaptive-clearing resume / re-engagement helpers.
//!
//! Mirrors [`crate::ops::assembly::adaptive::resume`].  Exposes the
//! pure-geometry path shortener ([`smooth_travel_path`]), the
//! Medial-Axis helpers ([`mat_resume_target`], [`nearest_uncleared_node`]),
//! and the two resume drivers ([`emit_resume_travel`], [`try_resume`])
//! so they can be exercised directly from Python tests.

use crate::ops::assembly::adaptive::resume;
use crate::ops::assembly::adaptive::AdaptiveClearingOptions;
use crate::python::geo::algo::medial_axis::PyMedialAxis;
use crate::python::ops::cut::cleared_area::PyClearedArea;
use crate::python::ops::PyOps;
use crate::types::{Point, Polygon};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) fn register(adaptive_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let resume_mod = PyModule::new(adaptive_mod.py(), "resume")?;
    register_functions!(
        resume_mod,
        smooth_travel_path_py,
        nearest_uncleared_node_py,
        mat_resume_target_py,
        emit_resume_travel_py,
        try_resume_py,
    );
    adaptive_mod.add_submodule(&resume_mod)?;

    let sys_modules = adaptive_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.adaptive.resume", &resume_mod)?;

    Ok(())
}

/// Smooth and shorten a cleared-territory travel path.
///
/// Feeds *raw* (a waypoint list known to stay inside cleared territory)
/// through ``build_smoothed_path`` against the uncleared obstacles
/// (*islands* ∪ *remaining*) so redundant intermediate waypoints are
/// shortcut away and sharp turns are rounded.
///
/// :param from_pt: Tool's current position ``(x, y)`` (preserved as the
///                 first point of the result).
/// :param raw: Waypoint list (e.g. from the MAT).
/// :param islands: Island (hole) polygons to avoid.
/// :param remaining: Remaining (uncut) stock polygons to avoid.
/// :param clearance: Minimum distance from obstacles (tool radius).
/// :returns: Shortened, smoothed path as a list of ``(x, y)`` points.
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def smooth_travel_path(
        from_pt: tuple[float, float],
        raw: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        remaining: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        clearance: float = 1.0,
    ) -> list[tuple[float, float]]:
        """Smooth and shorten a cleared-territory travel path."""
    "#,
    module = "raygeo.ops.assembly.adaptive.resume"
)]
#[pyfunction(name = "smooth_travel_path")]
#[pyo3(signature = (from_pt, raw, islands = None, remaining = None, clearance = 1.0))]
fn smooth_travel_path_py(
    from_pt: (f64, f64),
    raw: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    remaining: Option<Vec<Vec<(f64, f64)>>>,
    clearance: f64,
) -> Vec<(f64, f64)> {
    let from = Point::new(from_pt.0, from_pt.1);
    let raw_pts: Vec<Point> =
        raw.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let islands_pts: Vec<Polygon> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let remaining_pts: Vec<Polygon> = remaining
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    resume::smooth_travel_path(
        from,
        &raw_pts,
        &islands_pts,
        &remaining_pts,
        clearance,
    )
    .into_iter()
    .map(|p| (p.x, p.y))
    .collect()
}

/// BFS over the Medial Axis tree from a start node, returning the index
/// of the nearest (fewest hops) node that is **not** cleared.
///
/// :param axis: ``MedialAxis`` instance.
/// :param start: Starting node index.
/// :param is_cleared: Cleared/uncleared mask (one bool per node).
/// :returns: Index of the nearest uncleared node, or ``None``.
#[gen_stub_pyfunction(module = "raygeo.ops.assembly.adaptive.resume")]
#[pyfunction(name = "nearest_uncleared_node")]
fn nearest_uncleared_node_py(
    axis: &PyMedialAxis,
    start: usize,
    is_cleared: Vec<bool>,
) -> Option<usize> {
    resume::nearest_uncleared_node(&axis.inner, start, &is_cleared)
}

/// Pick a resume target by walking the Medial Axis Transform from the
/// tool's current position to the nearest uncleared MAT node.
///
/// :param axis: ``MedialAxis`` instance.
/// :param cleared: ``ClearedArea`` instance.
/// :param tool_pos: Tool position ``(x, y)``.
/// :param valid_tool_area: Valid tool-centre polygons.
/// :returns: ``(path, heading)`` where *path* is a list of ``(x, y)``
///           waypoints and *heading* is in radians, or ``None``.
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def mat_resume_target(
        axis: raygeo.geo.algo.medial_axis.MedialAxis,
        cleared: raygeo.ops.cut.cleared_area.ClearedArea,
        tool_pos: tuple[float, float],
        valid_tool_area: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
    ) -> tuple[list[tuple[float, float]], float] | None:
        """Pick a resume target by walking the MAT to the nearest uncleared node."""
    "#,
    module = "raygeo.ops.assembly.adaptive.resume"
)]
#[pyfunction(name = "mat_resume_target")]
fn mat_resume_target_py(
    axis: &PyMedialAxis,
    cleared: &PyClearedArea,
    tool_pos: (f64, f64),
    valid_tool_area: Vec<Vec<(f64, f64)>>,
) -> Option<(Vec<(f64, f64)>, f64)> {
    let valid: Vec<Polygon> = valid_tool_area
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    resume::mat_resume_target(
        &axis.inner,
        &cleared.inner,
        Point::new(tool_pos.0, tool_pos.1),
        &valid,
    )
    .map(|(path, heading)| {
        (path.into_iter().map(|p| (p.x, p.y)).collect(), heading)
    })
}

/// Emit a safe resume travel from *from_pt* to *to_pt* into *ops*.
///
/// When a Medial Axis is available, the travel is routed through cleared
/// territory (MAT tree walk, shortened via ``smooth_travel_path``).
/// Otherwise a single straight ``move_to`` is emitted.
///
/// :param ops: ``Ops`` instance to append travel moves to (mutated).
/// :param cleared: ``ClearedArea`` instance.
/// :param axis: ``MedialAxis`` instance or ``None``.
/// :param from_pt: Travel start ``(x, y)``.
/// :param to_pt: Travel destination ``(x, y)``.
/// :param pocket_boundary: Pocket boundary polygon.
/// :param islands: Island (hole) polygons.
/// :param radius: Tool radius (mm).
/// :param cut_z: Cutting Z height.
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def emit_resume_travel(
        ops: raygeo.ops.Ops,
        cleared: raygeo.ops.cut.cleared_area.ClearedArea,
        axis: raygeo.geo.algo.medial_axis.MedialAxis | None,
        from_pt: tuple[float, float],
        to_pt: tuple[float, float],
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        radius: float = 3.0,
        cut_z: float = -5.0,
    ) -> None:
        """Emit a safe resume travel from *from_pt* to *to_pt* into *ops*."""
    "#,
    module = "raygeo.ops.assembly.adaptive.resume"
)]
#[pyfunction(name = "emit_resume_travel")]
#[pyo3(signature = (
    ops,
    cleared,
    axis,
    from_pt,
    to_pt,
    pocket_boundary,
    islands = None,
    radius = 3.0,
    cut_z = -5.0,
))]
#[allow(clippy::too_many_arguments)]
fn emit_resume_travel_py(
    ops: &mut PyOps,
    cleared: &PyClearedArea,
    axis: Option<&PyMedialAxis>,
    from_pt: (f64, f64),
    to_pt: (f64, f64),
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    radius: f64,
    cut_z: f64,
) {
    let mat = axis.map(|a| &a.inner);
    let pb: Polygon = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Polygon> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let opts = AdaptiveClearingOptions {
        pocket_boundary: pb,
        islands: islands_pts,
        radius,
        cut_z,
        ..Default::default()
    };
    resume::emit_resume_travel(
        &mut ops.inner,
        &cleared.inner,
        mat,
        Point::new(from_pt.0, from_pt.1),
        Point::new(to_pt.0, to_pt.1),
        &opts,
    );
}

/// Try to recover after the tool stalls or is detected as stuck.
///
/// :param cleared: ``ClearedArea`` instance (mutated).
/// :param ops: ``Ops`` instance to append travel moves to (mutated).
/// :param tool: ``Tool`` instance (mutated).
/// :param pocket_boundary: Pocket boundary polygon.
/// :param islands: Island (hole) polygons.
/// :param radius: Tool radius (mm).
/// :param step_length: Forward step length (mm).
/// :param advance: Step-over distance (mm).
/// :param cut_z: Cutting Z height.
/// :param valid_tool_area: Valid tool-centre polygons.
/// :param axis: ``MedialAxis`` instance or ``None``.
/// :param last_resume_area: Cleared area at the last resume (mm²).
/// :returns: ``True`` if the tool was repositioned, ``False`` otherwise.
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def try_resume(
        cleared: raygeo.ops.cut.cleared_area.ClearedArea,
        ops: raygeo.ops.Ops,
        tool: raygeo.ops.assembly.adaptive.tool.Tool,
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        radius: float = 3.0,
        step_length: float = 0.6,
        advance: float = 1.5,
        cut_z: float = -5.0,
        valid_tool_area: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        axis: raygeo.geo.algo.medial_axis.MedialAxis | None = None,
        last_resume_area: float = -1.0,
    ) -> bool:
        """Try to recover after the tool stalls or is detected as stuck."""
    "#,
    module = "raygeo.ops.assembly.adaptive.resume"
)]
#[pyfunction(name = "try_resume")]
#[pyo3(signature = (
    cleared,
    ops,
    tool,
    pocket_boundary,
    islands = None,
    radius = 3.0,
    step_length = 0.6,
    advance = 1.5,
    cut_z = -5.0,
    valid_tool_area = None,
    axis = None,
    last_resume_area = -1.0,
))]
#[allow(clippy::too_many_arguments)]
fn try_resume_py(
    cleared: &mut PyClearedArea,
    ops: &mut PyOps,
    tool: &mut crate::python::ops::assembly::adaptive::tool::PyTool,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    radius: f64,
    step_length: f64,
    advance: f64,
    cut_z: f64,
    valid_tool_area: Option<Vec<Vec<(f64, f64)>>>,
    axis: Option<&PyMedialAxis>,
    last_resume_area: f64,
) -> bool {
    let mat = axis.map(|a| &a.inner);
    let pb: Polygon = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Polygon> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let vta: Vec<Polygon> = valid_tool_area
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let opts = AdaptiveClearingOptions {
        pocket_boundary: pb,
        islands: islands_pts,
        radius,
        step_length,
        advance,
        cut_z,
        ..Default::default()
    };
    let target_area_pd =
        crate::ops::assembly::adaptive::target_area_per_distance(
            radius,
            advance,
            step_length,
        );
    let max_def = opts.max_deflection_deg.to_radians();
    let target_eng =
        2.0 * std::f64::consts::PI - 2.0 * (advance / radius).acos();
    let min_cut_area = step_length * target_area_pd * 0.01;
    let segment_start = tool.inner.pos;
    resume::try_resume(
        &mut cleared.inner,
        &mut ops.inner,
        &mut tool.inner,
        &opts,
        &vta,
        target_area_pd,
        max_def,
        target_eng,
        min_cut_area,
        mat,
        last_resume_area,
        segment_start,
    )
}
