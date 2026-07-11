use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::ops::{
    AirAssistMode, Axis, CommandCategory, CommandType, CoolantMode,
    HeadCoolantMode, MarkerCmd, MoveCmd, OpCategory, StateCmd,
};

use crate::python::ops::axis::PyAxis;
use crate::types::Point3D;

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

pub(crate) fn cmd_to_dict<'a>(
    py: Python<'a>,
    ops: &crate::ops::Ops,
    idx: usize,
) -> PyResult<Bound<'a, PyDict>> {
    let d = PyDict::new(py);
    let node = &ops.commands[idx];
    let ct = node.command_type();
    let ct_name = ct.to_string();
    d.set_item("type", ct_name)?;

    if let OpCategory::Moving { end, .. } = &node.category {
        d.set_item("end", (end.x, end.y, end.z))?;
        if let Some(ea) = node.extra_axes() {
            let ea_dict = PyDict::new(py);
            for &(axis, val) in ea {
                let py_axis = Py::new(py, PyAxis(axis))?;
                let label: String =
                    py_axis.bind(py).getattr("label")?.extract()?;
                ea_dict.set_item(label, val)?;
            }
            d.set_item("extra_axes", ea_dict)?;
        }
    }

    match &node.category {
        OpCategory::Moving { cmd, .. } => match cmd {
            MoveCmd::ArcTo { center, cw, .. } => {
                d.set_item("center_offset", (center.x, center.y))?;
                d.set_item("clockwise", *cw)?;
            }
            MoveCmd::BezierTo {
                control1, control2, ..
            } => {
                d.set_item("control1", (control1.x, control1.y, control1.z))?;
                d.set_item("control2", (control2.x, control2.y, control2.z))?;
            }
            MoveCmd::QuadraticBezierTo { control, .. } => {
                d.set_item("control", (control.x, control.y, control.z))?;
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
            StateCmd::Dwell(dur) => d.set_item("duration_ms", *dur)?,
            StateCmd::SetPower(p) => d.set_item("power", *p)?,
            StateCmd::SetFeedRate(s) | StateCmd::SetRapidRate(s) => {
                d.set_item("speed", *s)?;
            }
            StateCmd::SetFrequency(f) => d.set_item("frequency", *f)?,
            StateCmd::SetPulseWidth(pw) => d.set_item("pulse_width", *pw)?,
            StateCmd::SetSpindleRpm(s) => d.set_item("spindle_speed", *s)?,
            StateCmd::SetCoolant(mode) => {
                d.set_item("coolant", format!("{:?}", mode))?;
            }
            StateCmd::SetAirAssist(mode) => {
                d.set_item("air_assist", format!("{:?}", mode))?;
            }
            StateCmd::SetHeadCoolant(mode) => {
                d.set_item("head_coolant", format!("{:?}", mode))?;
            }
            StateCmd::SetHead(uid) => {
                d.set_item("head_uid", uid.to_string())?
            }
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

pub fn create_and_append_command(
    cmd_data: &Bound<'_, PyDict>,
    ops: &mut crate::ops::Ops,
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
        let end_tuple = Point3D::new(end_data[0], end_data[1], end_data[2]);

        match ct {
            CommandType::MoveTo => {
                ops.move_to(end_tuple.x, end_tuple.y, end_tuple.z, extra_axes);
            }
            CommandType::LineTo => {
                ops.line_to(end_tuple.x, end_tuple.y, end_tuple.z, extra_axes);
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
                    end_tuple.x,
                    end_tuple.y,
                    co_vec[0],
                    co_vec[1],
                    cw,
                    end_tuple.z,
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
                let control1 = Point3D::new(c1_vec[0], c1_vec[1], c1_vec[2]);
                let control2 = Point3D::new(c2_vec[0], c2_vec[1], c2_vec[2]);
                ops.bezier_to(control1, control2, end_tuple, extra_axes);
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
                let c = Point3D::new(c_vec[0], c_vec[1], c_vec[2]);
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
                    end_tuple.x,
                    end_tuple.y,
                    end_tuple.z,
                    pv,
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
    } else if ct == CommandType::SetFeedRate || ct == CommandType::SetRapidRate
    {
        let s: i32 = cmd_data
            .get_item("speed")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'speed'")
            })?
            .extract()?;
        if ct == CommandType::SetFeedRate {
            ops.set_feed_rate(s);
        } else {
            ops.set_rapid_rate(s);
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
    } else if ct == CommandType::SetSpindleRpm {
        let s: u32 = cmd_data
            .get_item("spindle_speed")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'spindle_speed'")
            })?
            .extract()?;
        ops.set_spindle_rpm(s);
    } else if ct == CommandType::SetCoolant {
        let mode_str: String = cmd_data
            .get_item("coolant")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'coolant'")
            })?
            .extract()?;
        let mode = match mode_str.as_str() {
            "Flood" => CoolantMode::Flood,
            "Mist" => CoolantMode::Mist,
            _ => CoolantMode::Off,
        };
        ops.set_coolant(mode);
    } else if ct == CommandType::SetAirAssist {
        let mode_str: String = cmd_data
            .get_item("air_assist")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'air_assist'")
            })?
            .extract()?;
        let mode = match mode_str.as_str() {
            "On" => AirAssistMode::On,
            _ => AirAssistMode::Off,
        };
        ops.set_air_assist(mode);
    } else if ct == CommandType::SetHeadCoolant {
        let mode_str: String = cmd_data
            .get_item("head_coolant")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'head_coolant'")
            })?
            .extract()?;
        let mode = match mode_str.as_str() {
            "On" => HeadCoolantMode::On,
            _ => HeadCoolantMode::Off,
        };
        ops.set_head_coolant(mode);
    } else if ct == CommandType::SetHead {
        let uid: String = cmd_data
            .get_item("head_uid")?
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err("missing 'head_uid'")
            })?
            .extract()?;
        ops.set_head(&uid);
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
        let st =
            crate::ops::SectionType::from_name(&st_str).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown section type: {}",
                    st_str
                ))
            })?;
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
        let st =
            crate::ops::SectionType::from_name(&st_str).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown section type: {}",
                    st_str
                ))
            })?;
        ops.ops_section_end(st);
    } else if ct == CommandType::JobStart {
        ops.job_start();
    } else if ct == CommandType::JobEnd {
        ops.job_end();
    }

    Ok(())
}

pub fn ops_to_dict(
    py: Python<'_>,
    ops: &crate::ops::Ops,
) -> PyResult<Py<PyDict>> {
    let commands = PyList::empty(py);
    for i in 0..ops.len() {
        let d = cmd_to_dict(py, ops, i)?;
        commands.append(d)?;
    }
    let result = PyDict::new(py);
    result.set_item("commands", commands)?;
    result.set_item(
        "last_move_to",
        (ops.last_move_to.x, ops.last_move_to.y, ops.last_move_to.z),
    )?;
    Ok(result.unbind())
}

pub fn ops_from_dict(data: &Bound<'_, PyDict>) -> PyResult<crate::ops::Ops> {
    let _py = data.py();
    let mut ops = crate::ops::Ops::new();
    let last_move: Point3D = match data.get_item("last_move_to")? {
        Some(v) => {
            let l: Vec<f64> = v.extract()?;
            if l.len() != 3 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "last_move_to must be a 3-tuple",
                ));
            }
            Point3D::new(l[0], l[1], l[2])
        }
        None => Point3D::new(0.0, 0.0, 0.0),
    };
    ops.last_move_to = last_move;

    let commands_list = data.get_item("commands")?.ok_or_else(|| {
        pyo3::exceptions::PyKeyError::new_err("missing 'commands'")
    })?;
    let commands_list = commands_list.cast::<PyList>()?;

    for cmd_data_bound in commands_list.iter() {
        let cmd_data: &Bound<'_, PyDict> = cmd_data_bound.cast()?;
        create_and_append_command(cmd_data, &mut ops)?;
    }

    Ok(ops)
}
