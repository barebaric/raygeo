use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::cnc::machining::plan::{self, WorkplanStep};
use crate::geo::algo::helix::HelixDirection;
use crate::ops::state::State;
use crate::python::ops::assembly::result::PyAssemblyResult;
use crate::types::{Point, Point3D, Polygon};

fn dir_str(d: HelixDirection) -> &'static str {
    match d {
        HelixDirection::Cw => "CW",
        HelixDirection::Ccw => "CCW",
    }
}

fn parse_direction(s: &str) -> PyResult<HelixDirection> {
    match s {
        "CW" => Ok(HelixDirection::Cw),
        "CCW" => Ok(HelixDirection::Ccw),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "invalid direction {:?} (expected \"CW\" or \"CCW\")",
            other
        ))),
    }
}

fn polygon_to_py(poly: &Polygon) -> Vec<(f64, f64)> {
    poly.iter().map(|p| (p.x, p.y)).collect()
}

fn islands_to_py(islands: &[Polygon]) -> Vec<Vec<(f64, f64)>> {
    islands.iter().map(polygon_to_py).collect()
}

fn get_f64(d: &Bound<'_, PyDict>, key: &str) -> PyResult<f64> {
    d.get_item(key)?
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "missing step field {:?}",
                key
            ))
        })?
        .extract()
}

fn get_string(d: &Bound<'_, PyDict>, key: &str) -> PyResult<String> {
    d.get_item(key)?
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "missing step field {:?}",
                key
            ))
        })?
        .extract()
}

fn get_point(d: &Bound<'_, PyDict>, key: &str) -> PyResult<Point> {
    let (x, y): (f64, f64) = d
        .get_item(key)?
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "missing step field {:?}",
                key
            ))
        })?
        .extract()?;
    Ok(Point::new(x, y))
}

fn get_point3d(d: &Bound<'_, PyDict>, key: &str) -> PyResult<Point3D> {
    let (x, y, z): (f64, f64, f64) = d
        .get_item(key)?
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "missing step field {:?}",
                key
            ))
        })?
        .extract()?;
    Ok(Point3D::new(x, y, z))
}

fn get_polygon(d: &Bound<'_, PyDict>, key: &str) -> PyResult<Polygon> {
    let pts: Vec<(f64, f64)> = d
        .get_item(key)?
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "missing step field {:?}",
                key
            ))
        })?
        .extract()?;
    Ok(pts.into_iter().map(|(x, y)| Point::new(x, y)).collect())
}

fn get_islands(d: &Bound<'_, PyDict>, key: &str) -> PyResult<Vec<Polygon>> {
    let raw: Vec<Vec<(f64, f64)>> = d
        .get_item(key)?
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "missing step field {:?}",
                key
            ))
        })?
        .extract()?;
    Ok(raw
        .into_iter()
        .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        .collect())
}

fn get_carrier(d: &Bound<'_, PyDict>, key: &str) -> PyResult<Vec<Point>> {
    let raw: Vec<(f64, f64)> = d
        .get_item(key)?
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "missing step field {:?}",
                key
            ))
        })?
        .extract()?;
    Ok(raw.into_iter().map(|(x, y)| Point::new(x, y)).collect())
}

/// Serialise a [`WorkplanStep`] into an inspectable Python dict (with a
/// `"kind"` key). Shared by the workplan-builder bindings.
pub(crate) fn step_to_dict<'a>(
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
            d.set_item("direction", dir_str(*direction))?;
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
            d.set_item("direction", dir_str(*direction))?;
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
            d.set_item("carrier", carrier_to_py(carrier))?;
            d.set_item("start", (start.x, start.y, start.z))?;
            d.set_item("target_z", *target_z)?;
            d.set_item("tool_radius", *tool_radius)?;
            d.set_item("step_over", *step_over)?;
            d.set_item("max_ramp_angle_deg", *max_ramp_angle_deg)?;
            d.set_item("direction", dir_str(*direction))?;
            d.set_item("angular_step", *angular_step)?;
        }
        WorkplanStep::Slot {
            carrier,
            tool_radius,
            target_z,
        } => {
            d.set_item("kind", "Slot")?;
            d.set_item("carrier", carrier_to_py(carrier))?;
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
            d.set_item("pocket_boundary", polygon_to_py(pocket_boundary))?;
            d.set_item("islands", islands_to_py(islands))?;
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
            d.set_item("boundary", polygon_to_py(boundary))?;
            d.set_item("islands", islands_to_py(islands))?;
            d.set_item("tool_radius", *tool_radius)?;
            d.set_item("step_over", *step_over)?;
            d.set_item("step_length", *step_length)?;
            d.set_item("target_z", *target_z)?;
            d.set_item("safe_z", *safe_z)?;
            d.set_item("wall_margin", *wall_margin)?;
            d.set_item("stock_to_leave", *stock_to_leave)?;
        }
        WorkplanStep::Wavefront {
            pocket_boundary,
            islands,
            tool_radius,
            step_over,
            z,
            area_tolerance,
            precision,
        } => {
            d.set_item("kind", "Wavefront")?;
            d.set_item("pocket_boundary", polygon_to_py(pocket_boundary))?;
            d.set_item("islands", islands_to_py(islands))?;
            d.set_item("tool_radius", *tool_radius)?;
            d.set_item("step_over", *step_over)?;
            d.set_item("z", *z)?;
            d.set_item("area_tolerance", *area_tolerance)?;
            d.set_item("precision", *precision)?;
        }
        WorkplanStep::Retract { safe_z } => {
            d.set_item("kind", "Retract")?;
            d.set_item("safe_z", *safe_z)?;
        }
    }
    Ok(d)
}

fn carrier_to_py(carrier: &[Point]) -> Vec<(f64, f64)> {
    carrier.iter().map(|p| (p.x, p.y)).collect()
}

/// Deserialise a Python dict (with a `"kind"` key) back into a
/// [`WorkplanStep`]. Inverse of [`step_to_dict`]; used by the executor
/// binding so builders and the executor can communicate via plain dicts.
pub(crate) fn dict_to_step(d: &Bound<'_, PyDict>) -> PyResult<WorkplanStep> {
    let kind = get_string(d, "kind")?;
    match kind.as_str() {
        "HelixPlunge" => Ok(WorkplanStep::HelixPlunge {
            center: get_point(d, "center")?,
            helix_r: get_f64(d, "helix_r")?,
            z_start: get_f64(d, "z_start")?,
            z_end: get_f64(d, "z_end")?,
            pitch: get_f64(d, "pitch")?,
            direction: parse_direction(&get_string(d, "direction")?)?,
            angular_step: get_f64(d, "angular_step")?,
        }),
        "FlatSpiral" => Ok(WorkplanStep::FlatSpiral {
            center: get_point(d, "center")?,
            z: get_f64(d, "z")?,
            start_radius: get_f64(d, "start_radius")?,
            end_radius: get_f64(d, "end_radius")?,
            revolutions: get_f64(d, "revolutions")?,
            direction: parse_direction(&get_string(d, "direction")?)?,
            angular_step: get_f64(d, "angular_step")?,
            start_angle: get_f64(d, "start_angle")?,
        }),
        "RampEntry" => Ok(WorkplanStep::RampEntry {
            start: get_point(d, "start")?,
            end: get_point(d, "end")?,
            z_start: get_f64(d, "z_start")?,
            z_end: get_f64(d, "z_end")?,
            max_ramp_angle_deg: get_f64(d, "max_ramp_angle_deg")?,
            lateral_amplitude: get_f64(d, "lateral_amplitude")?,
        }),
        "ToroidalClear" => Ok(WorkplanStep::ToroidalClear {
            carrier: get_carrier(d, "carrier")?,
            start: get_point3d(d, "start")?,
            target_z: get_f64(d, "target_z")?,
            tool_radius: get_f64(d, "tool_radius")?,
            step_over: get_f64(d, "step_over")?,
            max_ramp_angle_deg: get_f64(d, "max_ramp_angle_deg")?,
            direction: parse_direction(&get_string(d, "direction")?)?,
            angular_step: get_f64(d, "angular_step")?,
        }),
        "Slot" => Ok(WorkplanStep::Slot {
            carrier: get_carrier(d, "carrier")?,
            tool_radius: get_f64(d, "tool_radius")?,
            target_z: get_f64(d, "target_z")?,
        }),
        "AdaptiveClear" => Ok(WorkplanStep::AdaptiveClear {
            pocket_boundary: get_polygon(d, "pocket_boundary")?,
            islands: get_islands(d, "islands")?,
            tool_radius: get_f64(d, "tool_radius")?,
            step_over: get_f64(d, "step_over")?,
            step_length: get_f64(d, "step_length")?,
            target_z: get_f64(d, "target_z")?,
            safe_z: get_f64(d, "safe_z")?,
            max_deflection_deg: get_f64(d, "max_deflection_deg")?,
            wall_margin: get_f64(d, "wall_margin")?,
            area_tolerance: get_f64(d, "area_tolerance")?,
            angular_step: get_f64(d, "angular_step")?,
        }),
        "ProfileInner" => Ok(WorkplanStep::ProfileInner {
            boundary: get_polygon(d, "boundary")?,
            islands: get_islands(d, "islands")?,
            tool_radius: get_f64(d, "tool_radius")?,
            step_over: get_f64(d, "step_over")?,
            step_length: get_f64(d, "step_length")?,
            target_z: get_f64(d, "target_z")?,
            safe_z: get_f64(d, "safe_z")?,
            wall_margin: get_f64(d, "wall_margin")?,
            stock_to_leave: get_f64(d, "stock_to_leave")?,
        }),
        "Wavefront" => Ok(WorkplanStep::Wavefront {
            pocket_boundary: get_polygon(d, "pocket_boundary")?,
            islands: get_islands(d, "islands")?,
            tool_radius: get_f64(d, "tool_radius")?,
            step_over: get_f64(d, "step_over")?,
            z: get_f64(d, "z")?,
            area_tolerance: get_f64(d, "area_tolerance")?,
            precision: get_f64(d, "precision")?,
        }),
        "Retract" => Ok(WorkplanStep::Retract {
            safe_z: get_f64(d, "safe_z")?,
        }),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown workplan step kind {:?}",
            other
        ))),
    }
}

pub(crate) fn register(machining_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = machining_mod.py();
    let m = PyModule::new(py, "plan")?;
    register_functions!(m, execute_workplan_py,);
    machining_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.machining.plan", &m)?;

    Ok(())
}

#[gen_stub_pyfunction(
    python = r#"
    import collections.abc
    import raygeo

    def execute_workplan(
        steps: list[dict],
        pocket_boundary: collections.abc.Sequence[tuple[float, float]],
        islands: collections.abc.Sequence[collections.abc.Sequence[tuple[float, float]]] | None = None,
        cut_feed_rate: int = 1200,
        cut_power: float = 1.0,
        rapid_feed_rate: int | None = None,
    ) -> raygeo.ops.assembly.result.AssemblyResult:
        """Execute a workplan: dispatch each step to its assembler.

        Each entry in *steps* is a ``WorkplanStep`` dict as produced by a
        builder such as :func:`raygeo.cnc.machining.wavefront.build_wavefront_workplan`
        or :func:`raygeo.cnc.machining.entry.build_entry_workplan`. The
        executor owns a shared cleared area, asks each step to invoke its
        own assembler, and chains the results into a single
        :class:`AssemblyResult`.

        :param steps: List of WorkplanStep dicts (with a ``"kind"`` key).
        :param pocket_boundary: Outer boundary of the pocket.
        :param islands: List of island polygons (default None).
        :param cut_feed_rate: Feed rate for cutting moves (default 1200).
        :param cut_power: Laser power for cutting moves (default 1.0).
        :param rapid_feed_rate: Feed rate for travel/retract moves, or
            ``None`` to leave them unmodified (default None).
        :returns: The combined :class:`AssemblyResult`.
        """
    "#,
    module = "raygeo.cnc.machining.plan"
)]
#[pyfunction(name = "execute_workplan")]
#[pyo3(signature = (
    steps,
    pocket_boundary,
    islands = None,
    cut_feed_rate = 1200,
    cut_power = 1.0,
    rapid_feed_rate = None,
))]
#[allow(clippy::too_many_arguments)]
fn execute_workplan_py(
    steps: &Bound<'_, PyAny>,
    pocket_boundary: Vec<(f64, f64)>,
    islands: Option<Vec<Vec<(f64, f64)>>>,
    cut_feed_rate: i32,
    cut_power: f64,
    rapid_feed_rate: Option<i32>,
) -> PyResult<PyAssemblyResult> {
    let mut parsed: Vec<WorkplanStep> = Vec::new();
    for item in steps.try_iter()? {
        let item = item?;
        let d = item.cast::<PyDict>()?;
        parsed.push(dict_to_step(d)?);
    }

    let boundary: Polygon = pocket_boundary
        .into_iter()
        .map(|(x, y)| Point::new(x, y))
        .collect();
    let islands_vec: Vec<Polygon> = islands
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
        power: 0.0,
        feed_rate: rapid_feed_rate,
        ..Default::default()
    };

    let result = plan::execute_workplan(
        &parsed,
        &boundary,
        &islands_vec,
        &cut_state,
        &travel_state,
    )?;
    Ok(PyAssemblyResult::from_inner(result))
}
