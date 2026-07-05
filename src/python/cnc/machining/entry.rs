use crate::cnc::machining::entry;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;
use crate::types::Point;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

pub(crate) fn register(machining_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = machining_mod.py();
    let m = PyModule::new(py, "entry")?;
    register_functions!(
        m,
        adaptive_entry_py,
        detect_entry_method_py,
        generate_helix_spiral_py,
    );
    machining_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.machining.entry", &m)?;

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
        cut_power: float = 1.0,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Fast central clearing entry.

        Finds the optimal entry pole using ``find_largest_circle``, then
        generates either a helix->spiral (wide area) or zigzag ramp
        (tight slot).

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
        :returns: An :class:`AssemblyResult` with the entry toolpath.
        """
    "#,
    module = "raygeo.cnc.machining.entry"
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

    let opts = entry::AdaptiveEntryOptions {
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

    let result = entry::adaptive_entry(&opts, &cut_state)?;
    Ok(PyAssemblyResult::from_inner(result))
}

#[gen_stub_pyfunction(
    python = r#"
    def detect_entry_method(
        r_max: float,
        tool_radius: float,
        safe_margin: float = 0.0,
    ) -> str:
        """Classify a pocket by its largest inscribed circle radius.

        Returns ``"helix_spiral"``, ``"toroid"``, ``"ramp"``, or ``"none"``.

        :param r_max: Radius of the largest inscribed circle (mm).
        :param tool_radius: Tool radius (mm).
        :param safe_margin: Safety margin (mm, default 0).
        :returns: Entry method name.
        """
    "#,
    module = "raygeo.cnc.machining.entry"
)]
#[pyfunction(name = "detect_entry_method")]
fn detect_entry_method_py(
    r_max: f64,
    tool_radius: f64,
    safe_margin: f64,
) -> String {
    match entry::detect_entry_method(r_max, tool_radius, safe_margin) {
        entry::EntryMethod::HelixSpiral => "helix_spiral".to_string(),
        entry::EntryMethod::Toroid => "toroid".to_string(),
        entry::EntryMethod::Ramp => "ramp".to_string(),
        entry::EntryMethod::None => "none".to_string(),
    }
}

#[gen_stub_pyfunction(
    python = r#"
    import raygeo

    def generate_helix_spiral(
        entry_pt: tuple[float, float],
        r_max: float,
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        safe_z: float = 2.0,
        target_z: float = -5.0,
        plunge_pitch: float = 1.0,
        safe_margin: float = 1.0,
        angular_step: float = 0.1,
        state: raygeo.ops.state.State | None = None,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Build a helix→spiral entry sequence.

        Chains a helical plunge with a flat Archimedean spiral + smoothing
        circular pass.  Useful when you already know the entry point and
        max radius (e.g. from ``find_largest_circle``).

        :param entry_pt: ``(x, y)`` entry point (pocket center).
        :param r_max: Max inscribed circle radius (mm).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial step-over per spiral revolution (default 2.0).
        :param safe_z: Safe (retract) Z height (default 2.0).
        :param target_z: Target cutting depth (default -5.0).
        :param plunge_pitch: Vertical descent per helix revolution (default 1.0).
        :param safe_margin: Extra margin from tool edge to boundary (default 1.0).
        :param angular_step: Angular step in radians (default 0.1).
        :param state: Optional machine state to apply before the path.
        :returns: An :class:`AssemblyResult`.
        """
    "#,
    module = "raygeo.cnc.machining.entry"
)]
#[pyfunction(name = "generate_helix_spiral")]
#[pyo3(signature = (
    entry_pt,
    r_max,
    tool_radius = 3.0,
    step_over = 2.0,
    safe_z = 2.0,
    target_z = -5.0,
    plunge_pitch = 1.0,
    safe_margin = 1.0,
    angular_step = 0.1,
    state = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_helix_spiral_py(
    entry_pt: (f64, f64),
    r_max: f64,
    tool_radius: f64,
    step_over: f64,
    safe_z: f64,
    target_z: f64,
    plunge_pitch: f64,
    safe_margin: f64,
    angular_step: f64,
    state: Option<Bound<'_, PyState>>,
) -> PyResult<PyAssemblyResult> {
    let cut_state = match state {
        Some(ref s) => s.borrow().0.clone(),
        None => State::default(),
    };

    let opts = entry::AdaptiveEntryOptions {
        pocket_boundary: vec![],
        islands: vec![],
        tool_radius,
        step_over,
        safe_z,
        target_z,
        plunge_pitch,
        safe_margin,
        angular_step,
    };

    let result = entry::generate_helix_spiral(
        Point::new(entry_pt.0, entry_pt.1),
        r_max,
        &opts,
        &cut_state,
    )?;
    Ok(PyAssemblyResult::from_inner(result))
}
