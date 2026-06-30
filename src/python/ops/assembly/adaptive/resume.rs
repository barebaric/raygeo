//! Python wrappers for the adaptive-clearing resume / re-engagement helpers.
//!
//! Mirrors [`crate::ops::assembly::adaptive::resume`].  Exposes the
//! pure-geometry path shortener ([`smooth_travel_path`]), the
//! Medial-Axis helpers ([`mat_resume_target`]),
//! and the two resume drivers ([`emit_resume_travel`], [`try_resume`])
//! so they can be exercised directly from Python tests.

use crate::ops::assembly::adaptive::resume::{self, ResumeCtx};
use crate::ops::assembly::adaptive::AdaptiveClearingOptions;
use crate::ops::cut::CutDirection;
use crate::ops::cut::ToolPose;
use crate::python::geo::algo::medial_axis::PyMedialAxis;
use crate::python::ops::cut::cleared_area::PyClearedArea;
use crate::python::ops::cut::search::PyToolPose;
use crate::python::ops::PyOps;
use crate::types::{Point, Polygon};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) fn register(adaptive_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let resume_mod = PyModule::new(adaptive_mod.py(), "resume")?;
    register_functions!(
        resume_mod,
        smooth_travel_path_py,
        mat_resume_target_py,
        search_reengagement_py,
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
/// through ``build_smoothed_path`` against the *obstacles* so redundant
/// intermediate waypoints are shortcut away and sharp turns are rounded.
///
/// :param from_pt: Tool's current position ``(x, y)`` (preserved as the
///                 first point of the result).
/// :param raw: Waypoint list (e.g. from the MAT).
/// :param obstacles: Obstacle polygons (islands + remaining stock).
/// :param clearance: Minimum distance from obstacles (tool radius).
/// :returns: Shortened, smoothed path as a list of ``(x, y)`` points.
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def smooth_travel_path(
        from_pt: tuple[float, float],
        raw: collections.abc.Sequence[tuple[float, float]],
        obstacles: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        clearance: float = 1.0,
    ) -> list[tuple[float, float]]:
        """Smooth and shorten a cleared-territory travel path."""
    "#,
    module = "raygeo.ops.assembly.adaptive.resume"
)]
#[pyfunction(name = "smooth_travel_path")]
#[pyo3(signature = (from_pt, raw, obstacles = None, clearance = 1.0))]
fn smooth_travel_path_py(
    from_pt: (f64, f64),
    raw: Vec<(f64, f64)>,
    obstacles: Option<Vec<Vec<(f64, f64)>>>,
    clearance: f64,
) -> Vec<(f64, f64)> {
    let from = Point::new(from_pt.0, from_pt.1);
    let raw_pts: Vec<Point> =
        raw.into_iter().map(|(x, y)| Point::new(x, y)).collect();
    let obstacles_pts: Vec<Polygon> = obstacles
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    resume::smooth_travel_path(from, &raw_pts, &obstacles_pts, clearance)
        .into_iter()
        .map(|p| (p.x, p.y))
        .collect()
}

/// Pick a resume target via MAT-guided frontier walk.
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def mat_resume_target(
        axis: raygeo.geo.algo.medial_axis.MedialAxis,
        cleared: raygeo.ops.cut.cleared_area.ClearedArea,
        tool: raygeo.ops.assembly.adaptive.tool.Tool,
        cut_direction: str,
        step_length: float,
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
        valid_tool_area: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
    ) -> raygeo.ops.cut.search.ToolPose | None:
        """Pick a resume target via MAT-guided frontier walk.

        :param cut_direction: ``"cw"`` or ``"ccw"``.
        """
    "#,
    module = "raygeo.ops.assembly.adaptive.resume"
)]
#[pyfunction(name = "mat_resume_target")]
#[allow(clippy::too_many_arguments)]
fn mat_resume_target_py(
    axis: &PyMedialAxis,
    cleared: &PyClearedArea,
    tool: &crate::python::ops::assembly::adaptive::tool::PyTool,
    cut_direction: &str,
    step_length: f64,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Vec<Vec<(f64, f64)>>,
    valid_tool_area: Vec<Vec<(f64, f64)>>,
) -> Option<PyToolPose> {
    let pb: Polygon = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let isls: Vec<Polygon> = islands
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let valid: Vec<Polygon> = valid_tool_area
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let cd = match cut_direction.to_ascii_lowercase().as_str() {
        "cw" => CutDirection::Cw,
        _ => CutDirection::Ccw,
    };
    let r = resume::mat_resume_target(
        &axis.inner,
        &cleared.inner,
        &tool.inner,
        cd,
        step_length,
        &pb,
        &isls,
        &valid,
    )?;
    Some(PyToolPose {
        pos: (r.pos.x, r.pos.y),
        heading: r.heading,
    })
}

/// SegmentResume: walk forward from segment_start along cut_direction
/// until probing finds engagement.
#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def search_reengagement(
        cleared: raygeo.ops.cut.cleared_area.ClearedArea,
        segment_start: tuple[float, float],
        cut_direction: tuple[float, float],
        radius: float,
        step_length: float,
        advance: float,
        min_cut_area: float,
        valid_tool_area: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]],
    ) -> raygeo.ops.cut.search.ToolPose | None:
        """SegmentResume: walk forward from segment_start along cut_direction."""
    "#,
    module = "raygeo.ops.assembly.adaptive.resume"
)]
#[pyfunction(name = "search_reengagement")]
#[allow(clippy::too_many_arguments)]
fn search_reengagement_py(
    cleared: &PyClearedArea,
    segment_start: (f64, f64),
    cut_direction: (f64, f64),
    radius: f64,
    step_length: f64,
    advance: f64,
    min_cut_area: f64,
    valid_tool_area: Vec<Vec<(f64, f64)>>,
) -> Option<PyToolPose> {
    let vta: Vec<Polygon> = valid_tool_area
        .into_iter()
        .map(|p| p.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let r = resume::search_reengagement(
        &cleared.inner,
        Point::new(segment_start.0, segment_start.1),
        Point::new(cut_direction.0, cut_direction.1),
        radius,
        step_length,
        advance,
        min_cut_area,
        &vta,
    )?;
    Some(PyToolPose {
        pos: (r.pos.x, r.pos.y),
        heading: r.heading,
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
/// :param max_deflection_deg: Maximum steering deflection per step
///                             in degrees (default 30).
/// :param valid_tool_area: Valid tool-centre polygons.
/// :param axis: ``MedialAxis`` instance or ``None``.
/// :param last_resume_area: Cleared area at the last resume (mm²).
/// :param cut_direction: ``"cw"`` or ``"ccw"`` (default ``"ccw"``).
/// :param segment_start: ``(x, y)`` position where the current
///                        cutting segment began.
/// :param segment_heading: Tool heading (radians) at segment start.
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
        max_deflection_deg: float = 30.0,
        valid_tool_area: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        axis: raygeo.geo.algo.medial_axis.MedialAxis | None = None,
        last_resume_area: float = -1.0,
        cut_direction: str = "ccw",
        segment_start: tuple[float, float] = (0.0, 0.0),
        segment_heading: float = 0.0,
    ) -> bool:
        """Try to recover after the tool stalls or is detected as stuck.

        :param cut_direction: ``"cw"`` or ``"ccw"``.
        """
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
    max_deflection_deg = 30.0,
    valid_tool_area = None,
    axis = None,
    last_resume_area = -1.0,
    cut_direction = "ccw",
    segment_start = (0.0, 0.0),
    segment_heading = 0.0,
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
    max_deflection_deg: f64,
    valid_tool_area: Option<Vec<Vec<(f64, f64)>>>,
    axis: Option<&PyMedialAxis>,
    last_resume_area: f64,
    cut_direction: &str,
    segment_start: (f64, f64),
    segment_heading: f64,
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
    let cd = match cut_direction.to_ascii_lowercase().as_str() {
        "cw" => CutDirection::Cw,
        _ => CutDirection::Ccw,
    };
    let opts = AdaptiveClearingOptions {
        pocket_boundary: pb,
        islands: islands_pts,
        radius,
        step_length,
        advance,
        cut_z,
        max_deflection_deg,
        cut_direction: cd,
        ..Default::default()
    };
    let target_area_pd =
        crate::ops::assembly::adaptive::target_area_per_distance(
            radius,
            advance,
            step_length,
        );

    let ctx = ResumeCtx {
        cleared: &cleared.inner,
        opts: &opts,
        valid_tool_area: &vta,
        mat,
        target_area_pd,
        segment_start: ToolPose {
            pos: Point::new(segment_start.0, segment_start.1),
            heading: segment_heading,
        },
        last_resume_area,
        last_resume_pos: tool.inner.pos,
    };
    let result = resume::try_resume(&ctx, &tool.inner);
    if let Some((_source, rp)) = result {
        resume::emit_resume_travel(
            &mut ops.inner,
            &cleared.inner,
            mat,
            tool.inner.pos,
            rp.pos,
            &opts,
        );
        tool.inner.pos = rp.pos;
        tool.inner.heading = rp.heading;
        tool.inner.reset_gyro();
        true
    } else {
        false
    }
}
