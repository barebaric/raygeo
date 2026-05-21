use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PySlice, PyString};

use raygeo_core::ops::{
    Axis, CommandCategory, CommandType, MarkerCmd, MoveCmd, OpCategory,
    StateCmd,
};

use super::axis::PyAxis;

/// Convert a Python dictionary of axis-value pairs into a Rust vector.
///
/// Keys may be :class:`Axis` objects or strings (``"X"``, ``"Y"``, ``"Z"``, etc.).
///
/// :param dict: Python dict mapping axes to float values.
/// :returns: Vector of ``(Axis, f64)`` tuples.
pub fn py_to_axis_map_helper(
    dict: &Bound<'_, PyDict>,
) -> PyResult<Vec<(Axis, f64)>> {
    let mut result = Vec::new();
    for item in dict.iter() {
        let (key, val) = item;
        let value: f64 = val.extract()?;
        if let Ok(py_axis) = key.cast::<PyAxis>() {
            result.push((py_axis.borrow().0, value));
        } else {
            let label: String = key.extract()?;
            let axis = Axis::from_str_name(&label).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown axis label: {}",
                    label
                ))
            })?;
            result.push((axis, value));
        }
    }
    Ok(result)
}

/// Convert a Rust vector of axis-value pairs into a Python dictionary.
///
/// :param py: Python GIL token.
/// :param axes: Slice of ``(Axis, f64)`` tuples.
/// :returns: Python dict mapping Axis objects to float values.
pub fn axis_map_to_py_helper<'a>(
    py: Python<'a>,
    axes: &[(Axis, f64)],
) -> PyResult<Bound<'a, PyDict>> {
    let dict = PyDict::new(py);
    for &(axis, val) in axes {
        let py_axis = Py::new(py, PyAxis(axis))?;
        dict.set_item(py_axis, val)?;
    }
    Ok(dict)
}

/// Serialize a single command at index *idx* to a Python dictionary.
///
/// :param py: Python GIL token.
/// :param ops: The ops container.
/// :param idx: Command index.
/// :returns: Dict with keys like ``type``, ``end``, ``power``, etc.
fn cmd_to_dict<'a>(
    py: Python<'a>,
    ops: &raygeo_core::ops::Ops,
    idx: usize,
) -> PyResult<Bound<'a, PyDict>> {
    let d = PyDict::new(py);
    let node = &ops.commands[idx];
    let ct = node.command_type();
    let ct_name = ct.name();
    d.set_item("type", ct_name)?;

    if let OpCategory::Moving { end, .. } = &node.category {
        d.set_item("end", (end.0, end.1, end.2))?;
        if let Some(ea) = node.extra_axes() {
            let ea_dict = PyDict::new(py);
            for &(axis, val) in ea {
                let py_axis = Py::new(py, PyAxis(axis))?;
                let label: String =
                    py_axis.bind(py).getattr("label")?.extract()?;
                ea_dict.set_item(label, val as f64)?;
            }
            d.set_item("extra_axes", ea_dict)?;
        }
    }

    match &node.category {
        OpCategory::Moving { cmd, .. } => match cmd {
            MoveCmd::ArcTo { center, cw, .. } => {
                d.set_item("center_offset", (center.0, center.1))?;
                d.set_item("clockwise", *cw)?;
            }
            MoveCmd::BezierTo { c1, c2, .. } => {
                d.set_item("control1", *c1)?;
                d.set_item("control2", *c2)?;
            }
            MoveCmd::QuadraticBezierTo { control, .. } => {
                d.set_item("control", *control)?;
            }
            MoveCmd::ScanLine { power_values, .. } => {
                d.set_item(
                    "power_values",
                    PyList::new(py, power_values.iter().copied())?,
                )?;
            }
            _ => {}
        },
        OpCategory::State(cmd) => match cmd {
            StateCmd::Dwell(dur) => {
                d.set_item("duration_ms", *dur)?;
            }
            StateCmd::SetPower(p) => {
                d.set_item("power", *p)?;
            }
            StateCmd::SetCutSpeed(s) | StateCmd::SetTravelSpeed(s) => {
                d.set_item("speed", *s)?;
            }
            StateCmd::SetFrequency(f) => {
                d.set_item("frequency", *f)?;
            }
            StateCmd::SetPulseWidth(pw) => {
                d.set_item("pulse_width", *pw)?;
            }
            StateCmd::SetLaser(uid) => {
                d.set_item("laser_uid", uid.to_string())?;
            }
            _ => {}
        },
        OpCategory::Marker(cmd) => match cmd {
            MarkerCmd::LayerStart(uid) | MarkerCmd::LayerEnd(uid) => {
                d.set_item("layer_uid", uid.to_string())?;
            }
            MarkerCmd::WorkpieceStart(uid) | MarkerCmd::WorkpieceEnd(uid) => {
                d.set_item("workpiece_uid", uid.to_string())?;
            }
            MarkerCmd::OpsSectionStart {
                section_type,
                workpiece_uid,
            } => {
                d.set_item("section_type", section_type.name())?;
                if let Some(wp) = workpiece_uid {
                    d.set_item("workpiece_uid", wp.to_string())?;
                }
            }
            MarkerCmd::OpsSectionEnd { section_type, .. } => {
                d.set_item("section_type", section_type.name())?;
            }
            _ => {}
        },
    }

    Ok(d)
}

/// Deserialize a command from a Python dictionary and append it to *ops*.
///
/// :param cmd_data: Dict with command data.
/// :param ops: The ops container to mutate.
fn create_and_append_command(
    cmd_data: &Bound<'_, PyDict>,
    ops: &mut raygeo_core::ops::Ops,
) -> PyResult<()> {
    let ct_str: String = cmd_data
        .get_item("type")?
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err("missing 'type'"))?
        .extract()?;

    let ct = CommandType::from_name(&ct_str).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "unknown command type: {}",
            ct_str
        ))
    })?;
    let cat = ct.category();

    let ea_raw: Option<Bound<'_, PyDict>> = cmd_data
        .get_item("extra_axes")?
        .and_then(|v| v.cast_into().ok());
    let extra_axes = match ea_raw {
        Some(ref d) => Some(py_to_axis_map_helper(d)?),
        None => None,
    };

    if cat == CommandCategory::Moving {
        let end_data: Vec<f64> = cmd_data
            .get_item("end")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'end'")
            })?
            .extract()?;
        let end_tuple = (end_data[0], end_data[1], end_data[2]);

        match ct {
            CommandType::MoveTo => {
                ops.move_to(end_tuple.0, end_tuple.1, end_tuple.2, extra_axes);
            }
            CommandType::LineTo => {
                ops.line_to(end_tuple.0, end_tuple.1, end_tuple.2, extra_axes);
            }
            CommandType::ArcTo => {
                let co_vec: Vec<f64> = cmd_data
                    .get_item("center_offset")?
                    .ok_or_else(|| {
                        pyo3::exceptions::PyKeyError::new_err(
                            "missing 'center_offset'",
                        )
                    })?
                    .extract()?;
                let cw: bool = cmd_data
                    .get_item("clockwise")?
                    .ok_or_else(|| {
                        pyo3::exceptions::PyKeyError::new_err(
                            "missing 'clockwise'",
                        )
                    })?
                    .extract()?;
                ops.arc_to(
                    end_tuple.0,
                    end_tuple.1,
                    co_vec[0],
                    co_vec[1],
                    cw,
                    end_tuple.2,
                    extra_axes,
                );
            }
            CommandType::BezierTo => {
                let c1_vec: Vec<f64> = cmd_data
                    .get_item("control1")?
                    .ok_or_else(|| {
                        pyo3::exceptions::PyKeyError::new_err(
                            "missing 'control1'",
                        )
                    })?
                    .extract()?;
                let c2_vec: Vec<f64> = cmd_data
                    .get_item("control2")?
                    .ok_or_else(|| {
                        pyo3::exceptions::PyKeyError::new_err(
                            "missing 'control2'",
                        )
                    })?
                    .extract()?;
                let c1 = (c1_vec[0], c1_vec[1], c1_vec[2]);
                let c2 = (c2_vec[0], c2_vec[1], c2_vec[2]);
                ops.bezier_to(c1, c2, end_tuple, extra_axes);
            }
            CommandType::QuadraticBezierTo => {
                let c_vec: Vec<f64> = cmd_data
                    .get_item("control")?
                    .ok_or_else(|| {
                        pyo3::exceptions::PyKeyError::new_err(
                            "missing 'control'",
                        )
                    })?
                    .extract()?;
                let c = (c_vec[0], c_vec[1], c_vec[2]);
                ops.quadratic_bezier_to(c, end_tuple, extra_axes);
            }
            CommandType::ScanLine => {
                let pv: Vec<u8> = cmd_data
                    .get_item("power_values")?
                    .ok_or_else(|| {
                        pyo3::exceptions::PyKeyError::new_err(
                            "missing 'power_values'",
                        )
                    })?
                    .extract()?;
                ops.scan_to(
                    end_tuple.0,
                    end_tuple.1,
                    end_tuple.2,
                    Some(pv),
                    extra_axes,
                );
            }
            _ => {}
        }
    } else if ct == CommandType::Dwell {
        let dur: f64 = cmd_data
            .get_item("duration_ms")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'duration_ms'")
            })?
            .extract()?;
        ops.dwell(dur);
    } else if ct == CommandType::SetPower {
        let p: f64 = cmd_data
            .get_item("power")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'power'")
            })?
            .extract()?;
        ops.set_power(p);
    } else if ct == CommandType::SetCutSpeed
        || ct == CommandType::SetTravelSpeed
    {
        let s: i32 = cmd_data
            .get_item("speed")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'speed'")
            })?
            .extract()?;
        if ct == CommandType::SetCutSpeed {
            ops.set_cut_speed(s);
        } else {
            ops.set_travel_speed(s);
        }
    } else if ct == CommandType::SetFrequency {
        let f: i32 = cmd_data
            .get_item("frequency")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'frequency'")
            })?
            .extract()?;
        ops.set_frequency(f);
    } else if ct == CommandType::SetPulseWidth {
        let pw: f64 = cmd_data
            .get_item("pulse_width")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'pulse_width'")
            })?
            .extract()?;
        ops.set_pulse_width(pw);
    } else if ct == CommandType::SetLaser {
        let uid: String = cmd_data
            .get_item("laser_uid")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'laser_uid'")
            })?
            .extract()?;
        ops.set_laser(&uid);
    } else if ct == CommandType::EnableAirAssist {
        ops.enable_air_assist(true);
    } else if ct == CommandType::DisableAirAssist {
        ops.disable_air_assist();
    } else if ct == CommandType::LayerStart || ct == CommandType::LayerEnd {
        let uid: String = cmd_data
            .get_item("layer_uid")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'layer_uid'")
            })?
            .extract()?;
        if ct == CommandType::LayerStart {
            ops.layer_start(&uid);
        } else {
            ops.layer_end(&uid);
        }
    } else if ct == CommandType::WorkpieceStart
        || ct == CommandType::WorkpieceEnd
    {
        let uid: String = cmd_data
            .get_item("workpiece_uid")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'workpiece_uid'")
            })?
            .extract()?;
        if ct == CommandType::WorkpieceStart {
            ops.workpiece_start(&uid);
        } else {
            ops.workpiece_end(&uid);
        }
    } else if ct == CommandType::OpsSectionStart {
        let st_str: String = cmd_data
            .get_item("section_type")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'section_type'")
            })?
            .extract()?;
        let st = raygeo_core::ops::SectionType::from_name(&st_str).ok_or_else(
            || {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown section type: {}",
                    st_str
                ))
            },
        )?;
        let wp_uid: Option<String> = cmd_data
            .get_item("workpiece_uid")?
            .and_then(|v| v.extract().ok());
        ops.ops_section_start(st, wp_uid.as_deref().unwrap_or(""));
    } else if ct == CommandType::OpsSectionEnd {
        let st_str: String = cmd_data
            .get_item("section_type")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'section_type'")
            })?
            .extract()?;
        let st = raygeo_core::ops::SectionType::from_name(&st_str).ok_or_else(
            || {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown section type: {}",
                    st_str
                ))
            },
        )?;
        ops.ops_section_end(st);
    } else if ct == CommandType::JobStart {
        ops.job_start();
    } else if ct == CommandType::JobEnd {
        ops.job_end();
    }

    Ok(())
}

/// Serialize the full ``Ops`` sequence to a Python dictionary.
///
/// The result contains a ``"commands"`` list and ``"last_move_to"``.
///
/// :param py: Python GIL token.
/// :param ops: The ops to serialize.
/// :returns: Python dict suitable for :func:`ops_from_dict`.
pub fn ops_to_dict(
    py: Python<'_>,
    ops: &raygeo_core::ops::Ops,
) -> PyResult<Py<PyDict>> {
    let commands = PyList::empty(py);
    for i in 0..ops.len() {
        let d = cmd_to_dict(py, ops, i)?;
        commands.append(d)?;
    }
    let result = PyDict::new(py);
    result.set_item("commands", commands)?;
    result.set_item("last_move_to", ops.last_move_to)?;
    Ok(result.unbind())
}

/// Deserialize an ``Ops`` sequence from a Python dictionary.
///
/// The dict should have the same structure as produced by :func:`ops_to_dict`.
///
/// :param data: Dict with ``"commands"`` and optionally ``"last_move_to"``.
/// :returns: A new Ops instance.
pub fn ops_from_dict(
    data: &Bound<'_, PyDict>,
) -> PyResult<raygeo_core::ops::Ops> {
    let _py = data.py();
    let mut ops = raygeo_core::ops::Ops::new();
    let last_move: (f64, f64, f64) = match data.get_item("last_move_to")? {
        Some(v) => {
            let l: Vec<f64> = v.extract()?;
            if l.len() != 3 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "last_move_to must be a 3-tuple",
                ));
            }
            (l[0], l[1], l[2])
        }
        None => (0.0, 0.0, 0.0),
    };
    ops.last_move_to = last_move;

    let commands_list_bound = data.get_item("commands")?.ok_or_else(|| {
        pyo3::exceptions::PyKeyError::new_err("missing 'commands'")
    })?;
    let commands_list = commands_list_bound.cast::<PyList>()?;

    for cmd_data_bound in commands_list.iter() {
        let cmd_data: &Bound<'_, PyDict> = cmd_data_bound.cast()?;
        create_and_append_command(cmd_data, &mut ops)?;
    }

    Ok(ops)
}

pub fn ops_to_numpy_arrays(
    py: Python<'_>,
    ops: &raygeo_core::ops::Ops,
) -> PyResult<Py<PyDict>> {
    use pyo3::types::PyByteArray;

    let num_cmds = ops.len();

    let num_arcs: usize = (0..num_cmds)
        .filter(|&i| ops.commands[i].command_type() == CommandType::ArcTo)
        .count();
    let num_beziers: usize = (0..num_cmds)
        .filter(|&i| {
            ops.commands[i].command_type() == CommandType::BezierTo
                || ops.commands[i].command_type()
                    == CommandType::QuadraticBezierTo
        })
        .count();

    let scanline_lengths: Vec<usize> = ops
        .commands
        .iter()
        .filter_map(|node| {
            if let OpCategory::Moving {
                cmd: MoveCmd::ScanLine { power_values },
                ..
            } = &node.category
            {
                Some(power_values.len())
            } else {
                None
            }
        })
        .collect();
    let total_scanline_bytes: usize = scanline_lengths.iter().sum();
    let num_scanlines = scanline_lengths.len();

    let numpy = py.import("numpy")?;

    let types = numpy
        .call_method1("zeros", (num_cmds,))?
        .call_method1("astype", ("int32",))?;
    let endpoints = numpy
        .call_method1("zeros", ((num_cmds, 3),))?
        .call_method1("astype", ("float32",))?;

    let arc_data = numpy
        .call_method1("zeros", ((num_arcs, 3),))?
        .call_method1("astype", ("float32",))?;
    let arc_map = numpy
        .call_method1("full", (num_cmds, -1))?
        .call_method1("astype", ("int32",))?;

    let bezier_data = numpy
        .call_method1("zeros", ((num_beziers, 6),))?
        .call_method1("astype", ("float32",))?;
    let bezier_map = numpy
        .call_method1("full", (num_cmds, -1))?
        .call_method1("astype", ("int32",))?;

    let scanline_data_arr = numpy
        .call_method1("zeros", (total_scanline_bytes,))?
        .call_method1("astype", ("uint8",))?;
    let scanline_map = numpy
        .call_method1("full", (num_cmds, -1))?
        .call_method1("astype", ("int32",))?;
    let scanline_indices = numpy
        .call_method1("zeros", ((num_scanlines, 2),))?
        .call_method1("astype", ("int32",))?;

    let mut state_marker_cmds_data: Vec<(usize, Py<PyDict>)> = Vec::new();
    let mut extra_axes_map: Vec<(usize, Py<PyDict>)> = Vec::new();
    let mut arc_idx: usize = 0;
    let mut bezier_idx: usize = 0;
    let mut scanline_idx: usize = 0;
    let mut scanline_offset: usize = 0;

    for i in 0..num_cmds {
        let node = &ops.commands[i];
        let ct = node.command_type();

        types.call_method1("__setitem__", (i, ct as i32))?;

        if let OpCategory::Moving { end, cmd } = &node.category {
            endpoints
                .call_method1("__setitem__", (i, vec![end.0, end.1, end.2]))?;

            match cmd {
                MoveCmd::BezierTo { c1, c2 } => {
                    bezier_data.call_method1(
                        "__setitem__",
                        (bezier_idx, vec![c1.0, c1.1, c1.2, c2.0, c2.1, c2.2]),
                    )?;
                    bezier_map
                        .call_method1("__setitem__", (i, bezier_idx as i32))?;
                    bezier_idx += 1;
                }
                MoveCmd::QuadraticBezierTo { control } => {
                    bezier_data.call_method1(
                        "__setitem__",
                        (
                            bezier_idx,
                            vec![
                                control.0, control.1, control.2, 0.0, 0.0, 0.0,
                            ],
                        ),
                    )?;
                    bezier_map
                        .call_method1("__setitem__", (i, bezier_idx as i32))?;
                    bezier_idx += 1;
                }
                MoveCmd::ArcTo { center, cw } => {
                    arc_data.call_method1(
                        "__setitem__",
                        (
                            arc_idx,
                            vec![
                                center.0,
                                center.1,
                                if *cw { 1.0 } else { 0.0 },
                            ],
                        ),
                    )?;
                    arc_map.call_method1("__setitem__", (i, arc_idx as i32))?;
                    arc_idx += 1;
                }
                MoveCmd::ScanLine { power_values } => {
                    let length = power_values.len();
                    let py_bytes = PyByteArray::new(py, power_values.as_ref());
                    let slice = PySlice::new(
                        py,
                        scanline_offset as isize,
                        (scanline_offset + length) as isize,
                        1,
                    );
                    scanline_data_arr
                        .call_method1("__setitem__", (slice, py_bytes))?;
                    scanline_indices.call_method1(
                        "__setitem__",
                        (
                            scanline_idx,
                            (scanline_offset, scanline_offset + length),
                        ),
                    )?;
                    scanline_map.call_method1(
                        "__setitem__",
                        (i, scanline_idx as i32),
                    )?;
                    scanline_offset += length;
                    scanline_idx += 1;
                }
                _ => {}
            }

            if let Some(ea) = node.extra_axes() {
                let ea_dict = PyDict::new(py);
                for &(axis, val) in ea {
                    let py_axis = Py::new(py, PyAxis(axis))?;
                    let label: String =
                        py_axis.bind(py).getattr("label")?.extract()?;
                    ea_dict.set_item(label, val)?;
                }
                extra_axes_map.push((i, ea_dict.unbind()));
            }
        } else {
            let d = cmd_to_dict(py, ops, i)?;
            state_marker_cmds_data.push((i, d.into()));
        }
    }

    let json_mod = py.import("json")?;
    let sm_dict = PyDict::new(py);
    for (idx, d) in &state_marker_cmds_data {
        sm_dict.set_item(idx.to_string(), d)?;
    }
    let json_str: String =
        json_mod.call_method1("dumps", (sm_dict,))?.extract()?;
    let json_bytes = numpy.call_method1(
        "frombuffer",
        (PyBytes::new(py, json_str.as_bytes()), "uint8"),
    )?;

    let result = PyDict::new(py);
    result.set_item("types", types)?;
    result.set_item("endpoints", endpoints)?;
    result.set_item("arc_data", arc_data)?;
    result.set_item("arc_map", arc_map)?;
    result.set_item("bezier_data", bezier_data)?;
    result.set_item("bezier_map", bezier_map)?;
    result.set_item("scanline_data", scanline_data_arr)?;
    result.set_item("scanline_indices", scanline_indices)?;
    result.set_item("scanline_map", scanline_map)?;
    result.set_item("state_marker_json_bytes", json_bytes)?;

    if !extra_axes_map.is_empty() {
        let ea_dict = PyDict::new(py);
        for (idx, d) in &extra_axes_map {
            ea_dict.set_item(idx.to_string(), d)?;
        }
        let ea_json_str: String =
            json_mod.call_method1("dumps", (ea_dict,))?.extract()?;
        let ea_json_bytes = numpy.call_method1(
            "frombuffer",
            (PyBytes::new(py, ea_json_str.as_bytes()), "uint8"),
        )?;
        result.set_item("extra_axes_json", ea_json_bytes)?;
    }

    Ok(result.unbind())
}

pub fn ops_from_numpy_arrays(
    arrays: &Bound<'_, PyDict>,
) -> PyResult<raygeo_core::ops::Ops> {
    let py = arrays.py();
    let numpy = py.import("numpy")?;

    let types_arr: Vec<i32> = arrays
        .get_item("types")?
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err("missing 'types'")
        })?
        .call_method0("tolist")?
        .extract()?;

    let num_cmds = types_arr.len();
    let mut ops = raygeo_core::ops::Ops::new();

    let json_bytes_bound = arrays
        .get_item("state_marker_json_bytes")?
        .unwrap_or_else(|| {
            numpy.call_method1("array", (Vec::<u8>::new(),)).unwrap()
        });
    let json_bytes_data: Vec<u8> =
        json_bytes_bound.call_method1("tobytes", ())?.extract()?;
    let json_mod = py.import("json")?;
    let state_marker_cmds_data: Py<PyDict> = if json_bytes_data.is_empty() {
        PyDict::new(py).unbind()
    } else {
        let json_str = String::from_utf8(json_bytes_data).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(e.to_string())
        })?;
        json_mod.call_method1("loads", (json_str,))?.extract()?
    };

    let extra_axes_data_bound: Option<Bound<'_, PyAny>> =
        arrays.get_item("extra_axes_json")?;
    let extra_axes_data: Py<PyDict> = if let Some(ref ea) =
        extra_axes_data_bound
    {
        let ea_bytes: Vec<u8> = ea.call_method1("tobytes", ())?.extract()?;
        if ea_bytes.is_empty() {
            PyDict::new(py).unbind()
        } else {
            let ea_str = String::from_utf8(ea_bytes).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(e.to_string())
            })?;
            json_mod.call_method1("loads", (ea_str,))?.extract()?
        }
    } else {
        PyDict::new(py).unbind()
    };

    let endpoints_bound = arrays.get_item("endpoints")?.ok_or_else(|| {
        pyo3::exceptions::PyKeyError::new_err("missing 'endpoints'")
    })?;
    let arc_data_bound = arrays.get_item("arc_data")?.ok_or_else(|| {
        pyo3::exceptions::PyKeyError::new_err("missing 'arc_data'")
    })?;
    let arc_map_bound = arrays.get_item("arc_map")?.ok_or_else(|| {
        pyo3::exceptions::PyKeyError::new_err("missing 'arc_map'")
    })?;
    let bezier_data_bound =
        arrays.get_item("bezier_data")?.unwrap_or_else(|| {
            numpy
                .call_method1("zeros", ((0, 6),))
                .unwrap()
                .call_method1("astype", ("float32",))
                .unwrap()
        });
    let bezier_map_bound =
        arrays.get_item("bezier_map")?.unwrap_or_else(|| {
            numpy
                .call_method1("full", (0, -1))
                .unwrap()
                .call_method1("astype", ("int32",))
                .unwrap()
        });
    let scanline_data_bound =
        arrays.get_item("scanline_data")?.ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err("missing 'scanline_data'")
        })?;
    let scanline_indices_bound =
        arrays.get_item("scanline_indices")?.ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err("missing 'scanline_indices'")
        })?;
    let scanline_map_bound =
        arrays.get_item("scanline_map")?.ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err("missing 'scanline_map'")
        })?;

    for i in 0..num_cmds {
        let i_str = i.to_string();
        let py_key = PyString::new(py, &i_str);

        let sm_bound = state_marker_cmds_data.bind(py);
        if let Some(cmd_data_any) = sm_bound.get_item(&py_key)? {
            let cmd_data = cmd_data_any.cast::<PyDict>()?;
            create_and_append_command(&cmd_data, &mut ops)?;
            continue;
        }

        let cmd_type_val = types_arr[i];
        let ct = CommandType::try_from(cmd_type_val as u8).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("{e}"))
        })?;
        let cat = ct.category();

        if cat != CommandCategory::Moving {
            continue;
        }

        let end_list: Vec<f32> = endpoints_bound
            .call_method1("__getitem__", (i,))?
            .call_method0("tolist")?
            .extract()?;
        let end_tuple =
            (end_list[0] as f64, end_list[1] as f64, end_list[2] as f64);

        match ct {
            CommandType::MoveTo => {
                ops.move_to(end_tuple.0, end_tuple.1, end_tuple.2, None);
            }
            CommandType::LineTo => {
                ops.line_to(end_tuple.0, end_tuple.1, end_tuple.2, None);
            }
            CommandType::ArcTo => {
                let arc_idx_val: i32 = arc_map_bound
                    .call_method1("__getitem__", (i,))?
                    .extract()?;
                let arc_vals: Vec<f32> = arc_data_bound
                    .call_method1("__getitem__", (arc_idx_val as isize,))?
                    .call_method0("tolist")?
                    .extract()?;
                ops.arc_to(
                    end_tuple.0,
                    end_tuple.1,
                    arc_vals[0] as f64,
                    arc_vals[1] as f64,
                    arc_vals[2] != 0.0,
                    end_tuple.2,
                    None,
                );
            }
            CommandType::BezierTo => {
                let bez_idx_val: i32 = bezier_map_bound
                    .call_method1("__getitem__", (i,))?
                    .extract()?;
                let bez_vals: Vec<f32> = bezier_data_bound
                    .call_method1("__getitem__", (bez_idx_val as isize,))?
                    .call_method0("tolist")?
                    .extract()?;
                ops.bezier_to(
                    (
                        bez_vals[0] as f64,
                        bez_vals[1] as f64,
                        bez_vals[2] as f64,
                    ),
                    (
                        bez_vals[3] as f64,
                        bez_vals[4] as f64,
                        bez_vals[5] as f64,
                    ),
                    end_tuple,
                    None,
                );
            }
            CommandType::QuadraticBezierTo => {
                let bez_idx_val: i32 = bezier_map_bound
                    .call_method1("__getitem__", (i,))?
                    .extract()?;
                let bez_vals: Vec<f32> = bezier_data_bound
                    .call_method1("__getitem__", (bez_idx_val as isize,))?
                    .call_method0("tolist")?
                    .extract()?;
                ops.quadratic_bezier_to(
                    (
                        bez_vals[0] as f64,
                        bez_vals[1] as f64,
                        bez_vals[2] as f64,
                    ),
                    end_tuple,
                    None,
                );
            }
            CommandType::ScanLine => {
                let scan_idx_val: i32 = scanline_map_bound
                    .call_method1("__getitem__", (i,))?
                    .extract()?;
                let si_list: Vec<i32> = scanline_indices_bound
                    .call_method1("__getitem__", (scan_idx_val as isize,))?
                    .call_method0("tolist")?
                    .extract()?;
                let start = si_list[0] as usize;
                let end = si_list[1] as usize;
                let pv_bytes: Vec<u8> = scanline_data_bound
                    .call_method1(
                        "__getitem__",
                        ((start..end).collect::<Vec<_>>(),),
                    )?
                    .call_method1("tobytes", ())?
                    .extract()?;
                ops.scan_to(
                    end_tuple.0,
                    end_tuple.1,
                    end_tuple.2,
                    Some(pv_bytes),
                    None,
                );
            }
            _ => {
                continue;
            }
        }

        let ea_bound = extra_axes_data.bind(py);
        if let Some(ea_item) = ea_bound.get_item(&py_key)? {
            let ea_dict = ea_item.cast::<PyDict>()?;
            let ea_vec = py_to_axis_map_helper(&ea_dict)?;
            let last_idx = ops.len() - 1;
            ops.commands[last_idx].set_extra_axes(std::sync::Arc::from(ea_vec));
        }
    }

    Ok(ops)
}
