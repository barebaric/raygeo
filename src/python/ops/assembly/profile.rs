use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::assembly::profile::{
    self, ProfileInnerOptions, ProfileOuterOptions,
};
use crate::ops::assembly::Tracelet;
use crate::ops::cut::CutDirection;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::cut::cleared_area::PyClearedArea;
use crate::types::{Point, Point3D, Polygon};

fn check_cancel() -> bool {
    let rc = unsafe { pyo3::ffi::PyErr_CheckSignals() };
    if rc == -1 {
        unsafe { pyo3::ffi::PyErr_Clear() };
        true
    } else {
        false
    }
}

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = assembly_mod.py();
    let m = PyModule::new(py, "profile")?;
    m.add_function(pyo3::wrap_pyfunction!(profile_outer_py, m.clone())?)?;
    m.add_function(pyo3::wrap_pyfunction!(profile_inner_py, m.clone())?)?;
    assembly_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.profile", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def profile_outer(
        cleared: raygeo.ops.cut.cleared_area.ClearedArea,
        boundary: list[tuple[float, float]],
        tool_radius: float,
        step_over: float,
        step_length: float,
        target_z: float,
        safe_z: float,
        wall_margin: float,
        cut_feed_rate: int,
        cut_power: float,
        start_pos: tuple[float, float] | None = None,
        cut_direction: str = "ccw",
        stock_to_leave: float = 0.0,
        engagement_area_threshold: float = 0.0,
        engagement_angle_threshold: float = 3.141592653589793,
        trace_path: str | None = None,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Profile the outer boundary of a pocket.

        Walks a tool around the grown boundary (offset outward by tool
        radius).  The path stays approximately one tool radius outside
        the original boundary, removing any excess stock on the outer
        side.  Returns an :class:`AssemblyResult` with the profiling
        move sequence.

        :param cleared: Cleared area tracker.
        :param boundary: Outer boundary polygon as ``(x, y)`` pairs.
        :param tool_radius: Tool radius in mm.
        :param step_over: Radial step-over between passes (mm).
        :param step_length: Forward step length in mm.
        :param target_z: Cutting depth (Z).
        :param safe_z: Safe (rapid) Z height.
        :param wall_margin: Extra distance to keep from the wall (mm).
        :param cut_feed_rate: Feed rate in mm/min.
        :param cut_power: Spindle power (0.0–1.0).
        :param start_pos: Optional override start ``(x, y)`` (default: first boundary vertex).
        :param cut_direction: ``"cw"`` or ``"ccw"`` (default ``"ccw"``).
        :param stock_to_leave: Stock left on wall for rough pass (mm, default 0.0).
        :param engagement_area_threshold: Overengagement area threshold (mm², 0 = auto).
        :param engagement_angle_threshold: Overengagement angle threshold (rad, default π).
        :param trace_path: Optional path to write a binary trace file (default None).
        :returns: An :class:`AssemblyResult` with the profiling path.
        """
    "#,
    module = "raygeo.ops.assembly.profile"
)]
#[pyfunction(name = "profile_outer")]
#[pyo3(signature = (
    cleared,
    boundary,
    tool_radius,
    step_over,
    step_length,
    target_z,
    safe_z,
    wall_margin,
    cut_feed_rate,
    cut_power,
    start_pos = None,
    cut_direction = "ccw",
    stock_to_leave = 0.0,
    engagement_area_threshold = 0.0,
    engagement_angle_threshold = std::f64::consts::PI,
    trace_path = None,
))]
#[allow(clippy::too_many_arguments)]
fn profile_outer_py(
    cleared: &mut PyClearedArea,
    boundary: Vec<(f64, f64)>,
    tool_radius: f64,
    step_over: f64,
    step_length: f64,
    target_z: f64,
    safe_z: f64,
    wall_margin: f64,
    cut_feed_rate: i32,
    cut_power: f64,
    start_pos: Option<(f64, f64)>,
    cut_direction: &str,
    stock_to_leave: f64,
    engagement_area_threshold: f64,
    engagement_angle_threshold: f64,
    trace_path: Option<String>,
) -> PyResult<PyAssemblyResult> {
    use std::path::PathBuf;
    let boundary_pts: crate::types::Polygon = boundary
        .into_iter()
        .map(|(x, y)| crate::types::Point::new(x, y))
        .collect();

    let cd = match cut_direction.to_ascii_lowercase().as_str() {
        "cw" => CutDirection::Cw,
        _ => CutDirection::Ccw,
    };

    let opts = ProfileOuterOptions {
        boundary: boundary_pts,
        tool_radius,
        step_over,
        step_length,
        target_z,
        safe_z,
        wall_margin,
        stock_to_leave,
        cut_direction: cd,
        start_pos: start_pos.map(|(x, y)| Point3D::new(x, y, target_z)),
        tolerance: 0.1,
        expansion_batch_size: 20,
        cancel_check: Some(check_cancel),
        engagement_area_threshold,
        engagement_angle_threshold,
        trace_path: trace_path.map(PathBuf::from),
    };

    let cut_state = State {
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };

    let mut trace = Tracelet::new();
    let meta = profile::profile_outer(
        &mut trace,
        &mut cleared.inner,
        &opts,
        &cut_state,
    )?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def profile_inner(
        cleared: raygeo.ops.cut.cleared_area.ClearedArea,
        boundary: list[tuple[float, float]],
        islands: list[list[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        step_over: float = 1.5,
        step_length: float = 0.6,
        target_z: float = -5.0,
        safe_z: float = 2.0,
        wall_margin: float = 0.0,
        stock_to_leave: float = 0.0,
        cut_feed_rate: int = 1000,
        cut_power: float = 0.0,
        start_pos: tuple[float, float] | None = None,
        cut_direction: str = "ccw",
        engagement_area_threshold: float = 0.0,
        engagement_angle_threshold: float = 3.141592653589793,
        trace_path: str | None = None,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Profile the inner boundary of a pocket, around islands.

        Walks a tool around the inset boundary (offset inward by tool
        radius) and around accessible islands, so that the tool clears
        the material along the pocket walls and around each island.
        Returns an :class:`AssemblyResult` with the profiling move
        sequence.

        :param cleared: Cleared area tracker.
        :param boundary: Outer boundary polygon as ``(x, y)`` pairs.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm.
        :param step_over: Radial step-over between passes (mm).
        :param step_length: Forward step length in mm.
        :param target_z: Cutting depth (Z).
        :param safe_z: Safe (rapid) Z height.
        :param wall_margin: Extra distance to keep from the wall (mm).
        :param stock_to_leave: Stock left on wall for rough pass (mm, default 0.0).
        :param cut_feed_rate: Feed rate in mm/min.
        :param cut_power: Spindle power (0.0–1.0).
        :param start_pos: Optional override start ``(x, y)`` (default: first boundary vertex).
        :param cut_direction: ``"cw"`` or ``"ccw"`` (default ``"ccw"``).
        :param engagement_area_threshold: Overengagement area threshold (mm², 0 = auto).
        :param engagement_angle_threshold: Overengagement angle threshold (rad, default π).
        :param trace_path: Optional path to write a binary trace file (default None).
        :returns: An :class:`AssemblyResult` with the profiling path.
        """
    "#,
    module = "raygeo.ops.assembly.profile"
)]
#[pyfunction(name = "profile_inner")]
#[pyo3(signature = (
    cleared,
    boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 1.5,
    step_length = 0.6,
    target_z = -5.0,
    safe_z = 2.0,
    wall_margin = 0.0,
    stock_to_leave = 0.0,
    cut_feed_rate = 1000,
    cut_power = 0.0,
    start_pos = None,
    cut_direction = "ccw",
    engagement_area_threshold = 0.0,
    engagement_angle_threshold = std::f64::consts::PI,
    trace_path = None,
))]
#[allow(clippy::too_many_arguments)]
fn profile_inner_py(
    cleared: &mut PyClearedArea,
    boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    step_length: f64,
    target_z: f64,
    safe_z: f64,
    wall_margin: f64,
    stock_to_leave: f64,
    cut_feed_rate: i32,
    cut_power: f64,
    start_pos: Option<(f64, f64)>,
    cut_direction: &str,
    engagement_area_threshold: f64,
    engagement_angle_threshold: f64,
    trace_path: Option<String>,
) -> PyResult<PyAssemblyResult> {
    use std::path::PathBuf;
    let boundary_pts: Polygon = boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();

    let islands_pts: Vec<Polygon> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let cd = match cut_direction.to_ascii_lowercase().as_str() {
        "cw" => CutDirection::Cw,
        _ => CutDirection::Ccw,
    };

    let opts = ProfileInnerOptions {
        boundary: boundary_pts,
        islands: islands_pts,
        tool_radius,
        step_over,
        step_length,
        target_z,
        safe_z,
        wall_margin,
        stock_to_leave,
        cut_direction: cd,
        start_pos: start_pos.map(|(x, y)| Point3D::new(x, y, target_z)),
        tolerance: 0.1,
        expansion_batch_size: 20,
        cancel_check: Some(check_cancel),
        engagement_area_threshold,
        engagement_angle_threshold,
        trace_path: trace_path.map(PathBuf::from),
    };

    let cut_state = State {
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };

    let mut trace = Tracelet::new();
    let meta = profile::profile_inner(
        &mut trace,
        &mut cleared.inner,
        &opts,
        &cut_state,
    )?;
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
}
