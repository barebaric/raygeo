use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use std::path::PathBuf;

use crate::cnc::machining::plan::{self, WorkplanStep};
use crate::geo::algo::helix::HelixDirection;
use crate::ops::assembly::Tracelet;
use crate::ops::part::Part;
use crate::ops::state::State;
use crate::python::ops::assembly::progress_event_to_py;
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
            part,
            tool_radius,
            step_over,
            step_length,
            target_z,
            safe_z,
            max_deflection_deg,
            wall_margin,
            area_tolerance,
            angular_step,
            start_pos,
            start_heading,
        } => {
            let (boundary, islands) = part.extract_boundary();
            let boundary = boundary.unwrap_or_default();
            d.set_item("kind", "AdaptiveClear")?;
            d.set_item("pocket_boundary", polygon_to_py(&boundary))?;
            d.set_item("islands", islands_to_py(&islands))?;
            d.set_item("tool_radius", *tool_radius)?;
            d.set_item("step_over", *step_over)?;
            d.set_item("step_length", *step_length)?;
            d.set_item("target_z", *target_z)?;
            d.set_item("safe_z", *safe_z)?;
            d.set_item("max_deflection_deg", *max_deflection_deg)?;
            d.set_item("wall_margin", *wall_margin)?;
            d.set_item("area_tolerance", *area_tolerance)?;
            d.set_item("angular_step", *angular_step)?;
            d.set_item("start_pos", start_pos.map(|p| (p.x, p.y, p.z)))?;
            d.set_item("start_heading", *start_heading)?;
        }
        WorkplanStep::ProfileInner {
            part,
            tool_radius,
            step_over,
            step_length,
            target_z,
            safe_z,
            wall_margin,
            stock_to_leave,
        } => {
            let (boundary, islands) = part.extract_boundary();
            let boundary = boundary.unwrap_or_default();
            d.set_item("kind", "ProfileInner")?;
            d.set_item("boundary", polygon_to_py(&boundary))?;
            d.set_item("islands", islands_to_py(&islands))?;
            d.set_item("tool_radius", *tool_radius)?;
            d.set_item("step_over", *step_over)?;
            d.set_item("step_length", *step_length)?;
            d.set_item("target_z", *target_z)?;
            d.set_item("safe_z", *safe_z)?;
            d.set_item("wall_margin", *wall_margin)?;
            d.set_item("stock_to_leave", *stock_to_leave)?;
        }
        WorkplanStep::Wavefront {
            part,
            step_over,
            z,
            area_tolerance,
            precision,
        } => {
            let (boundary, islands) = part.extract_boundary();
            let boundary = boundary.unwrap_or_default();
            d.set_item("kind", "Wavefront")?;
            d.set_item("pocket_boundary", polygon_to_py(&boundary))?;
            d.set_item("islands", islands_to_py(&islands))?;
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
        "AdaptiveClear" => {
            let start_pos = match d.get_item("start_pos")? {
                Some(v) if !v.is_none() => {
                    let (x, y, z): (f64, f64, f64) = v.extract()?;
                    Some(Point3D::new(x, y, z))
                }
                _ => None,
            };
            let start_heading = match d.get_item("start_heading")? {
                Some(v) if !v.is_none() => Some(v.extract::<f64>()?),
                _ => None,
            };
            let boundary = get_polygon(d, "pocket_boundary")?;
            let islands = get_islands(d, "islands")?;
            Ok(WorkplanStep::AdaptiveClear {
                part: Part::from_polygons(&boundary, &islands, (0.0, 0.0)),
                tool_radius: get_f64(d, "tool_radius")?,
                step_over: get_f64(d, "step_over")?,
                step_length: get_f64(d, "step_length")?,
                target_z: get_f64(d, "target_z")?,
                safe_z: get_f64(d, "safe_z")?,
                max_deflection_deg: get_f64(d, "max_deflection_deg")?,
                wall_margin: get_f64(d, "wall_margin")?,
                area_tolerance: get_f64(d, "area_tolerance")?,
                angular_step: get_f64(d, "angular_step")?,
                start_pos,
                start_heading,
            })
        }
        "ProfileInner" => {
            let boundary = get_polygon(d, "boundary")?;
            let islands = get_islands(d, "islands")?;
            Ok(WorkplanStep::ProfileInner {
                part: Part::from_polygons(&boundary, &islands, (0.0, 0.0)),
                tool_radius: get_f64(d, "tool_radius")?,
                step_over: get_f64(d, "step_over")?,
                step_length: get_f64(d, "step_length")?,
                target_z: get_f64(d, "target_z")?,
                safe_z: get_f64(d, "safe_z")?,
                wall_margin: get_f64(d, "wall_margin")?,
                stock_to_leave: get_f64(d, "stock_to_leave")?,
            })
        }
        "Wavefront" => {
            let boundary = get_polygon(d, "pocket_boundary")?;
            let islands = get_islands(d, "islands")?;
            Ok(WorkplanStep::Wavefront {
                part: Part::from_polygons(&boundary, &islands, (0.0, 0.0)),
                step_over: get_f64(d, "step_over")?,
                z: get_f64(d, "z")?,
                area_tolerance: get_f64(d, "area_tolerance")?,
                precision: get_f64(d, "precision")?,
            })
        }
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
    m.add_class::<PyWorkplan>()?;
    machining_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.machining.plan", &m)?;

    Ok(())
}

/// Plan-time context and executor for a sequence of WorkplanSteps.
///
/// Captures the pocket boundary, islands, and safe-Z at construction
/// time.  After extending with steps from a builder (e.g.
///     :func:`raygeo.cnc.machining.entry.build_entry_workplan`
/// or :func:`raygeo.cnc.machining.entry.build_entry_workplan`),
/// call :meth:`execute` to produce a combined :class:`AssemblyResult`.
#[gen_stub_pyclass(module = "raygeo.cnc.machining.plan")]
#[pyclass(
    name = "Workplan",
    module = "raygeo.cnc.machining.plan",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyWorkplan {
    inner: plan::Workplan,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyWorkplan {
    #[new]
    #[pyo3(signature = (pocket_boundary, islands = None, safe_z = 2.0))]
    fn __new__(
        pocket_boundary: Vec<(f64, f64)>,
        islands: Option<Vec<Vec<(f64, f64)>>>,
        safe_z: f64,
    ) -> Self {
        let boundary: Polygon = pocket_boundary
            .into_iter()
            .map(|(x, y)| Point::new(x, y))
            .collect();
        let islands_vec: Vec<Polygon> = islands
            .unwrap_or_default()
            .into_iter()
            .map(|h| h.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        PyWorkplan {
            inner: plan::Workplan::new(boundary, islands_vec, safe_z),
        }
    }

    /// Create a Workplan from a :class:`raygeo.ops.part.Part`, extracting boundary
    /// and islands from ``part.geometry``.
    ///
    /// Raises ``ValueError`` if the part has no extractable boundary geometry.
    #[staticmethod]
    #[pyo3(signature = (part, safe_z = 2.0))]
    fn from_part(
        part: &crate::python::ops::part::part::PyPart,
        safe_z: f64,
    ) -> PyResult<Self> {
        match plan::Workplan::from_part(&part.inner, safe_z) {
            Some(wp) => Ok(PyWorkplan { inner: wp }),
            None => Err(pyo3::exceptions::PyValueError::new_err(
                "Part has no extractable boundary geometry",
            )),
        }
    }

    /// Append builder output steps (list of WorkplanStep dicts).
    fn extend(&mut self, steps: &Bound<'_, PyAny>) -> PyResult<()> {
        let mut parsed: Vec<WorkplanStep> = Vec::new();
        for item in steps.try_iter()? {
            let item = item?;
            let d = item.cast::<PyDict>()?;
            parsed.push(dict_to_step(d)?);
        }
        self.inner.extend(&parsed);
        Ok(())
    }

    /// Execute all steps and return the combined :class:`AssemblyResult`.
    ///
    /// :param cut_feed_rate: Feed rate for cutting moves (default 1200).
    /// :param cut_power: Laser power for cutting moves (default 1.0).
    /// :param rapid_feed_rate: Feed rate for travel/retract moves, or
    ///     ``None`` to leave them unmodified (default None).
    /// :param trace: Optional file path for a trace file (``.bin``).
    /// :param on_progress: Optional callback receiving progress dicts.
    /// :param batch_size: Ops batch size for on_progress (default 128).
    /// :returns: The combined :class:`AssemblyResult`.
    #[pyo3(signature = (cut_feed_rate = 1200, cut_power = 1.0, rapid_feed_rate = None, trace = None, on_progress = None, batch_size = 128))]
    fn execute(
        &self,
        cut_feed_rate: i32,
        cut_power: f64,
        rapid_feed_rate: Option<i32>,
        trace: Option<String>,
        on_progress: Option<Py<PyAny>>,
        batch_size: usize,
    ) -> PyResult<PyAssemblyResult> {
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
        let trace_path = trace.map(PathBuf::from);

        let mut tl = if let Some(cb) = on_progress {
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
        let meta = self.inner.execute(
            &mut tl,
            &cut_state,
            &travel_state,
            trace_path,
        )?;
        let ops = tl.into_ops();

        // Workplan results have no trace events in the result (they went to the file)
        Ok(PyAssemblyResult::from_parts(ops, meta, None, vec![]))
    }

    fn __repr__(&self) -> String {
        format!(
            "Workplan(steps={}, safe_z={})",
            self.inner.steps.len(),
            self.inner.safe_z,
        )
    }
}
