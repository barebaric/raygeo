use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::machining::entry::{
    self, build_entry_workplan, AdaptiveEntryOptions, EntryMethod,
    EntryWorkplanOptions,
};
use crate::cnc::machining::plan::WorkplanStep;
use crate::geo::algo::helix::HelixDirection;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::python::ops::state::PyState;
use crate::types::Point;

pub(crate) fn register(machining_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = machining_mod.py();
    let m = PyModule::new(py, "entry")?;
    register_functions!(
        m,
        adaptive_entry_py,
        detect_entry_method_py,
        generate_helix_spiral_py,
        build_entry_workplan_py,
    );
    machining_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.machining.entry", &m)?;

    Ok(())
}

fn step_to_dict<'a>(
    py: Python<'a>,
    step: &WorkplanStep,
) -> PyResult<Bound<'a, PyDict>> {
    let d = PyDict::new(py);
    match step {
        WorkplanStep::HelixPlunge {
            center,
            helix_r,
            z_start,
            z_end,
            pitch,
            direction,
            angular_step,
        } => {
            d.set_item("kind", "HelixPlunge")?;
            d.set_item("center", (center.x, center.y))?;
            d.set_item("helix_r", *helix_r)?;
            d.set_item("z_start", *z_start)?;
            d.set_item("z_end", *z_end)?;
            d.set_item("pitch", *pitch)?;
            d.set_item(
                "direction",
                match direction {
                    HelixDirection::Cw => "CW",
                    HelixDirection::Ccw => "CCW",
                },
            )?;
            d.set_item("angular_step", *angular_step)?;
        }
        WorkplanStep::FlatSpiral {
            center,
            z,
            start_radius,
            end_radius,
            revolutions,
            direction,
            angular_step,
            start_angle,
        } => {
            d.set_item("kind", "FlatSpiral")?;
            d.set_item("center", (center.x, center.y))?;
            d.set_item("z", *z)?;
            d.set_item("start_radius", *start_radius)?;
            d.set_item("end_radius", *end_radius)?;
            d.set_item("revolutions", *revolutions)?;
            d.set_item(
                "direction",
                match direction {
                    HelixDirection::Cw => "CW",
                    HelixDirection::Ccw => "CCW",
                },
            )?;
            d.set_item("angular_step", *angular_step)?;
            d.set_item("start_angle", *start_angle)?;
        }
        WorkplanStep::RampEntry {
            start,
            end,
            z_start,
            z_end,
            max_ramp_angle_deg,
            lateral_amplitude,
        } => {
            d.set_item("kind", "RampEntry")?;
            d.set_item("start", (start.x, start.y))?;
            d.set_item("end", (end.x, end.y))?;
            d.set_item("z_start", *z_start)?;
            d.set_item("z_end", *z_end)?;
            d.set_item("max_ramp_angle_deg", *max_ramp_angle_deg)?;
            d.set_item("lateral_amplitude", *lateral_amplitude)?;
        }
        WorkplanStep::ToroidalClear {
            carrier,
            start,
            target_z,
            tool_radius,
            step_over,
            max_ramp_angle_deg,
            direction,
            angular_step,
        } => {
            d.set_item("kind", "ToroidalClear")?;
            let carrier_py: Vec<(f64, f64)> =
                carrier.iter().map(|p| (p.x, p.y)).collect();
            d.set_item("carrier", carrier_py)?;
            d.set_item("start", (start.x, start.y, start.z))?;
            d.set_item("target_z", *target_z)?;
            d.set_item("tool_radius", *tool_radius)?;
            d.set_item("step_over", *step_over)?;
            d.set_item("max_ramp_angle_deg", *max_ramp_angle_deg)?;
            d.set_item(
                "direction",
                match direction {
                    HelixDirection::Cw => "CW",
                    HelixDirection::Ccw => "CCW",
                },
            )?;
            d.set_item("angular_step", *angular_step)?;
        }
        WorkplanStep::Slot {
            carrier,
            tool_radius,
            target_z,
        } => {
            d.set_item("kind", "Slot")?;
            let carrier_py: Vec<(f64, f64)> =
                carrier.iter().map(|p| (p.x, p.y)).collect();
            d.set_item("carrier", carrier_py)?;
            d.set_item("tool_radius", *tool_radius)?;
            d.set_item("target_z", *target_z)?;
        }
        WorkplanStep::AdaptiveClear {
            pocket_boundary,
            islands,
            tool_radius,
            step_over,
            step_length,
            target_z,
            safe_z,
            max_deflection_deg,
            wall_margin,
            area_tolerance,
            angular_step,
        } => {
            d.set_item("kind", "AdaptiveClear")?;
            let boundary_py: Vec<(f64, f64)> =
                pocket_boundary.iter().map(|p| (p.x, p.y)).collect();
            d.set_item("pocket_boundary", boundary_py)?;
            let islands_py: Vec<Vec<(f64, f64)>> = islands
                .iter()
                .map(|isl| isl.iter().map(|p| (p.x, p.y)).collect())
                .collect();
            d.set_item("islands", islands_py)?;
            d.set_item("tool_radius", *tool_radius)?;
            d.set_item("step_over", *step_over)?;
            d.set_item("step_length", *step_length)?;
            d.set_item("target_z", *target_z)?;
            d.set_item("safe_z", *safe_z)?;
            d.set_item("max_deflection_deg", *max_deflection_deg)?;
            d.set_item("wall_margin", *wall_margin)?;
            d.set_item("area_tolerance", *area_tolerance)?;
            d.set_item("angular_step", *angular_step)?;
        }
        WorkplanStep::ProfileInner {
            boundary,
            islands,
            tool_radius,
            step_over,
            step_length,
            target_z,
            safe_z,
            wall_margin,
            stock_to_leave,
        } => {
            d.set_item("kind", "ProfileInner")?;
            let boundary_py: Vec<(f64, f64)> =
                boundary.iter().map(|p| (p.x, p.y)).collect();
            d.set_item("boundary", boundary_py)?;
            let islands_py: Vec<Vec<(f64, f64)>> = islands
                .iter()
                .map(|isl| isl.iter().map(|p| (p.x, p.y)).collect())
                .collect();
            d.set_item("islands", islands_py)?;
            d.set_item("tool_radius", *tool_radius)?;
            d.set_item("step_over", *step_over)?;
            d.set_item("step_length", *step_length)?;
            d.set_item("target_z", *target_z)?;
            d.set_item("safe_z", *safe_z)?;
            d.set_item("wall_margin", *wall_margin)?;
            d.set_item("stock_to_leave", *stock_to_leave)?;
        }
        WorkplanStep::Retract { safe_z } => {
            d.set_item("kind", "Retract")?;
            d.set_item("safe_z", *safe_z)?;
        }
    }
    Ok(d)
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc

    def build_entry_workplan(
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        tool_radius: float = 3.0,
        step_over: float = 2.0,
        safe_z: float = 2.0,
        target_z: float = -5.0,
        plunge_pitch: float = 1.0,
        safe_margin: float = 1.0,
        angular_step: float = 0.1,
    ) -> list[dict]:
        """Build an entry workplan for a pocket.

        Uses feature detection to determine the best entry strategy per
        disconnected wide sub-region: helix+spiral (if r_max >= 2xD),
        toroidal ramp (if find_ramp_carrier succeeds), or zigzag ramp
        (last resort).

        :param pocket_boundary: Outer boundary as [(x, y), ...].
        :param islands: List of island polygons (default None).
        :param tool_radius: Tool radius in mm (default 3.0).
        :param step_over: Radial step-over (default 2.0).
        :param safe_z: Safe Z height (default 2.0).
        :param target_z: Target cutting depth (default -5.0).
        :param plunge_pitch: Helix pitch per revolution (default 1.0).
        :param safe_margin: Safety margin from tool edge (default 1.0).
        :param angular_step: Angular step in radians (default 0.1).
        :returns: List of WorkplanStep dicts with a "kind" key.
        """
    "#,
    module = "raygeo.cnc.machining.entry"
)]
#[pyfunction(name = "build_entry_workplan")]
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
#[allow(clippy::too_many_arguments)]
fn build_entry_workplan_py(
    py: Python<'_>,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    tool_radius: f64,
    step_over: f64,
    safe_z: f64,
    target_z: f64,
    plunge_pitch: f64,
    safe_margin: f64,
    angular_step: f64,
) -> PyResult<Vec<Bound<'_, PyDict>>> {
    let boundary: Vec<Point> = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_vec: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = EntryWorkplanOptions {
        pocket_boundary: boundary,
        islands: islands_vec,
        tool_radius,
        step_over,
        safe_z,
        target_z,
        plunge_pitch,
        safe_margin,
        angular_step,
    };

    let steps = build_entry_workplan(&opts)?;
    let mut result: Vec<Bound<'_, PyDict>> = Vec::with_capacity(steps.len());
    for step in &steps {
        result.push(step_to_dict(py, step)?);
    }
    Ok(result)
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
    let islands_vec: Vec<Vec<Point>> = islands
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect();

    let opts = AdaptiveEntryOptions {
        pocket_boundary: boundary,
        islands: islands_vec,
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
        safe_margin: float = 1.0,
    ) -> str:
        """Classify a pocket by its largest inscribed circle radius.

        :param r_max: Largest inscribed circle radius (mm).
        :param tool_radius: Tool radius in mm.
        :param safe_margin: Safety margin (mm, default 1.0).
        :returns: One of ``"HelixSpiral"``, ``"Toroid"``, ``"Ramp"``, or
                  ``"None"``.
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
        EntryMethod::HelixSpiral => "HelixSpiral".to_string(),
        EntryMethod::Toroid => "Toroid".to_string(),
        EntryMethod::Ramp => "Ramp".to_string(),
        EntryMethod::None => "None".to_string(),
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

    let opts = AdaptiveEntryOptions {
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
