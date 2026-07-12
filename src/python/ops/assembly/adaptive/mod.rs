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
use crate::python::ops::assembly::progress_event_to_py;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::types::Point3D;
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
        part: raygeo.ops.cut.Part,
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
        on_progress: collections.abc.Callable[[dict], None] | None = None,
        batch_size: int = 128,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Run forward-stepping adaptive clearing.

        Starting from the pre-populated cleared area inside *part*,
        uses a constant-engagement stepping solver to generate a
        continuous spiral toolpath from the seed clearing to the
        pocket wall.

        The caller is responsible for populating the part's cleared
        area with the entry polygons (e.g. via a workplan built by
        :func:`raygeo.cnc.machining.wavefront.build_wavefront_workplan`
        and executed by
        :class:`raygeo.cnc.machining.plan.Workplan`) and
        prepending the entry Ops to the result.

        :param part: The part whose ``cleared`` field tracks accumulated
                     workpiece state and whose geometry defines the
                     pocket boundary and islands.
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
        :param on_progress: Optional callback receiving progress dicts.
        :param batch_size: Ops batch size for on_progress (default 128).
        :returns: Ops with cutting commands (entry not included).
        """
    "#,
    module = "raygeo.ops.assembly.adaptive"
)]
#[pyfunction(name = "adaptive_clearing")]
#[pyo3(signature = (
    part,
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
    on_progress = None,
    batch_size = 128,
))]
#[allow(clippy::too_many_arguments)]
fn adaptive_clearing_py(
    part: &mut crate::python::ops::cut::part::PyPart,
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
    on_progress: Option<Py<PyAny>>,
    batch_size: usize,
) -> PyResult<PyAssemblyResult> {
    let cd = match cut_direction.to_ascii_lowercase().as_str() {
        "cw" => CutDirection::Cw,
        _ => CutDirection::Ccw,
    };

    let opts = adaptive::AdaptiveClearingOptions {
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

    use crate::ops::assembly::Tracelet;
    let mut trace = if let Some(cb) = on_progress {
        Tracelet::with_callback(
            Box::new(move |event| {
                Python::attach(|py| {
                    let py_event = progress_event_to_py(py, event);
                    let _ = cb.call1(py, (py_event,));
                });
            }),
            batch_size,
        )
    } else {
        Tracelet::new()
    };
    let meta = adaptive::adaptive_clearing(
        &mut part.inner,
        &mut trace,
        &opts,
        &cut_state,
    )?;
    if profile {
        prof_report();
    }
    let events = trace.drain();
    let attrs = trace.attrs().cloned();
    let ops = trace.into_ops();
    Ok(PyAssemblyResult::from_parts(ops, meta, attrs, events))
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
