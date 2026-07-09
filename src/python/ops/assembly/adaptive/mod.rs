//! Python wrappers for the adaptive-clearing orchestrator.
//!
//! Mirrors the Rust [`crate::ops::assembly::adaptive`] module split:
//! * `adaptive_clearing` / `target_area_per_distance` live here (the
//!   orchestrator entry points),
//! * [`super::tool`] exposes the [`Tool`] state as a Python class,
//! * [`super::resume`] exposes the resume / re-engagement helpers.

pub(crate) mod resume;
pub(crate) mod routing;
pub(crate) mod tool;

use crate::ops::assembly::adaptive;
use crate::ops::cut::CutDirection;
use crate::ops::state::State;
use crate::prof::prof_report;
use crate::python::errors::{ResumePointNotFoundError, RoutingError};
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::cut::cleared_area::PyClearedArea;
use crate::types::{Point, Point3D};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use std::path::PathBuf;

/// Callback passed to the core algorithm as `cancel_check`.
/// Called periodically from the main loop.  Returns `true` when the
/// user has pressed Ctrl+C.
///
/// Uses `PyErr_CheckSignals` so signal delivery works even though Rust
/// holds the GIL.  After detecting the signal we clear the pending
/// `KeyboardInterrupt` exception so the core algorithm can return
/// `RaygeoError::Cancelled`, which the `From` impl maps back to a clean
/// `KeyboardInterrupt`.
fn check_cancel() -> bool {
    // Safety: called from the Python thread while holding the GIL.
    let rc = unsafe { pyo3::ffi::PyErr_CheckSignals() };
    if rc == -1 {
        // A signal was delivered — clear the exception so the core
        // algorithm returns normally (with Cancelled error) rather than
        // leaving a dangling KeyboardInterrupt in the thread state.
        unsafe { pyo3::ffi::PyErr_Clear() };
        true
    } else {
        false
    }
}

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let adaptive_mod = PyModule::new(assembly_mod.py(), "adaptive")?;

    // Register exception types so they are importable from Python.
    let py = assembly_mod.py();
    adaptive_mod.add("RoutingError", py.get_type::<RoutingError>())?;
    adaptive_mod.add(
        "ResumePointNotFoundError",
        py.get_type::<ResumePointNotFoundError>(),
    )?;

    register_functions!(
        adaptive_mod,
        adaptive_clearing_py,
        target_area_per_distance_py,
    );

    tool::register(&adaptive_mod)?;
    resume::register(&adaptive_mod)?;
    routing::register(&adaptive_mod)?;

    assembly_mod.add_submodule(&adaptive_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly.adaptive", &adaptive_mod)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def adaptive_clearing(
        cleared: raygeo.ops.cut.cleared_area.ClearedArea,
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] = [],
        tool_radius: float = 3.0,
        step_over: float = 1.5,
        step_length: float = 0.6,
        target_z: float = -5.0,
        safe_z: float = 2.0,
        max_deflection_deg: float = 30.0,
        wall_margin: float = 0.0,
        area_tolerance: float = 1.0,
        cut_feed_rate: int = 1200,
        cut_power: float = 1.0,
        start_pos: tuple[float, float] | None = None,
        start_heading: float | None = None,
        expansion_batch_size: int = 20,
        profile: bool = False,
        cut_direction: str = "ccw",
        trace_path: str | None = None,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Run forward-stepping adaptive clearing.

        Starting from the pre-populated *cleared* area, uses a
        constant-engagement stepping solver to generate a continuous
        spiral toolpath from the seed clearing to the pocket wall.

        The caller is responsible for populating *cleared* with
        the entry polygons (e.g. via a workplan built by
        :func:`raygeo.cnc.machining.wavefront.build_wavefront_workplan`
        and executed by
        :func:`raygeo.cnc.machining.plan.execute_workplan`) and
        prepending the entry Ops to the result.

        :param cleared: ``ClearedArea`` instance (mutated in place).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island (hole) polygons (default []).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Step-over distance (default 1.5).
        :param step_length: Forward step length in mm (default 0.6).
        :param target_z: Cutting Z height (default -5.0).
        :param safe_z: Retract Z height for travel (default 2.0).
        :param max_deflection_deg: Maximum steering deflection per step
                                   in degrees (default 30).
        :param wall_margin: Extra clearance between tool and boundary (default 0.0).
        :param area_tolerance: Stop when remaining uncut area drops below
                               this threshold (default 1.0).
        :param cut_feed_rate: Feed rate for cutting moves (default 1200).
        :param cut_power: Laser power for cutting moves (0.0-1.0, default 1.0).
        :param start_pos: Initial tool position (x, y).  When None,
                          auto-detected from the cleared-area frontier.
        :param start_heading: Initial tool heading in radians.  When None,
                              auto-detected as the CCW tangent at start_pos.
        :param expansion_batch_size: Batch cleared-area expansions every
                                     N steps (default 20).  Larger values
                                     improve performance but may slightly
                                     reduce path quality.
        :param profile: Print a profiling report to stdout (default False).
        :param cut_direction: Rotational direction of all cutting moves.
                              ``"cw"`` or ``"ccw"`` (default ``"ccw"``).
        :param trace_path: When set, write a per-step binary trace file for
                           the Python inspector (debug builds only).
        :returns: Ops with cutting commands (entry not included).
        """
    "#,
    module = "raygeo.ops.assembly.adaptive"
)]
#[pyfunction(name = "adaptive_clearing")]
#[pyo3(signature = (
    cleared,
    pocket_boundary,
    islands = None,
    tool_radius = 3.0,
    step_over = 1.5,
    step_length = 0.6,
    target_z = -5.0,
    safe_z = 2.0,
    max_deflection_deg = 30.0,
    wall_margin = 0.0,
    area_tolerance = 1.0,
    cut_feed_rate = 1200,
    cut_power = 1.0,
    start_pos = None,
    start_heading = None,
    expansion_batch_size = 20,
    profile = false,
    trace_path = None,
    cut_direction = "ccw",
))]
#[allow(clippy::too_many_arguments)]
fn adaptive_clearing_py(
    cleared: &mut PyClearedArea,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    step_length: f64,
    target_z: f64,
    safe_z: f64,
    max_deflection_deg: f64,
    wall_margin: f64,
    area_tolerance: f64,
    cut_feed_rate: i32,
    cut_power: f64,
    start_pos: Option<(f64, f64)>,
    start_heading: Option<f64>,
    expansion_batch_size: usize,
    profile: bool,
    trace_path: Option<String>,
    cut_direction: &str,
) -> PyResult<PyAssemblyResult> {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_pts: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();
    let cd = match cut_direction.to_ascii_lowercase().as_str() {
        "cw" => CutDirection::Cw,
        _ => CutDirection::Ccw,
    };

    let opts = adaptive::AdaptiveClearingOptions {
        pocket_boundary: boundary,
        islands: islands_pts,
        tool_radius,
        step_over,
        step_length,
        target_z,
        safe_z,
        max_deflection_deg,
        wall_margin,
        area_tolerance,
        start_pos: start_pos.map(|(x, y)| Point3D::new(x, y, target_z)),
        start_heading,
        expansion_batch_size,
        trace_path: trace_path.map(PathBuf::from),
        cut_direction: cd,
        tolerance: 0.1,
        cancel_check: Some(check_cancel),
    };

    let cut_state = State {
        power: cut_power,
        feed_rate: Some(cut_feed_rate),
        ..Default::default()
    };

    let result =
        adaptive::adaptive_clearing(&mut cleared.inner, &opts, &cut_state)?;
    if profile {
        prof_report();
    }
    Ok(PyAssemblyResult::from_inner(result))
}

/// Target cut-area per unit distance for the engagement solver.
///
/// :param radius: Tool radius in mm.
/// :param advance: Step-over distance in mm.
/// :param step_length: Forward step length in mm.
/// :returns: Target area per distance (mm).
#[gen_stub_pyfunction(module = "raygeo.ops.assembly.adaptive")]
#[pyfunction(name = "target_area_per_distance")]
fn target_area_per_distance_py(
    radius: f64,
    advance: f64,
    step_length: f64,
) -> f64 {
    adaptive::target_area_per_distance(radius, advance, step_length)
}
