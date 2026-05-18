use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyType};
use pyo3::{Bound, Py, PyAny, PyResult};

use raygeo_core::ops::{Axis, CommandCategory, CommandType};

use super::axis::PyAxis;
use super::enums::{PyCommandCategory, PyCommandType, PySectionType};
use super::state::PyState;
use crate::geo::geometry::Geometry as PyGeometry;

fn normalize_index(idx: isize, len: usize) -> PyResult<usize> {
    let len = len as isize;
    let idx = if idx < 0 { len + idx } else { idx };
    if idx < 0 || idx >= len {
        Err(pyo3::exceptions::PyIndexError::new_err(format!(
            "index out of range: {}",
            idx
        )))
    } else {
        Ok(idx as usize)
    }
}

fn py_to_axis_map(dict: &Bound<'_, PyDict>) -> PyResult<Vec<(Axis, f64)>> {
    super::serialize::py_to_axis_map_helper(dict)
}

fn axis_map_to_py<'a>(
    py: Python<'a>,
    axes: &[(Axis, f64)],
) -> PyResult<Bound<'a, PyDict>> {
    super::serialize::axis_map_to_py_helper(py, axes)
}

#[pyclass(module = "raygeo.ops", name = "CommandInfo")]
pub struct PyCommandInfo {
    #[pyo3(get)]
    pub type_: PyCommandType,
    #[pyo3(get)]
    pub end: Option<(f64, f64, f64)>,
    #[pyo3(get)]
    pub extra_axes: Option<Py<PyDict>>,
    #[pyo3(get)]
    pub state: Option<Py<PyState>>,
    #[pyo3(get)]
    pub center_offset: Option<(f64, f64)>,
    #[pyo3(get)]
    pub clockwise: Option<bool>,
    #[pyo3(get)]
    pub control1: Option<(f64, f64, f64)>,
    #[pyo3(get)]
    pub control2: Option<(f64, f64, f64)>,
    #[pyo3(get)]
    pub control: Option<(f64, f64, f64)>,
    #[pyo3(get)]
    pub power_values: Option<Py<PyBytes>>,
    #[pyo3(get)]
    pub power: Option<f64>,
    #[pyo3(get)]
    pub speed: Option<i32>,
    #[pyo3(get)]
    pub frequency: Option<i32>,
    #[pyo3(get)]
    pub pulse_width: Option<f64>,
    #[pyo3(get)]
    pub laser_uid: Option<String>,
    #[pyo3(get)]
    pub duration_ms: Option<f64>,
    #[pyo3(get)]
    pub layer_uid: Option<String>,
    #[pyo3(get)]
    pub workpiece_uid: Option<String>,
    #[pyo3(get)]
    pub section_type: Option<String>,
}

#[pymethods]
impl PyCommandInfo {
    fn __eq__(
        &self,
        py: Python<'_>,
        other: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        if let Ok(other_info) = other.extract::<PyRef<'_, PyCommandInfo>>() {
            if self.type_ != other_info.type_ {
                return Ok(false);
            }
            if self.end != other_info.end {
                return Ok(false);
            }
            if self.center_offset != other_info.center_offset {
                return Ok(false);
            }
            if self.clockwise != other_info.clockwise {
                return Ok(false);
            }
            if self.control1 != other_info.control1 {
                return Ok(false);
            }
            if self.control2 != other_info.control2 {
                return Ok(false);
            }
            if self.control != other_info.control {
                return Ok(false);
            }
            if self.power != other_info.power {
                return Ok(false);
            }
            if self.speed != other_info.speed {
                return Ok(false);
            }
            if self.frequency != other_info.frequency {
                return Ok(false);
            }
            if self.pulse_width != other_info.pulse_width {
                return Ok(false);
            }
            if self.laser_uid != other_info.laser_uid {
                return Ok(false);
            }
            if self.duration_ms != other_info.duration_ms {
                return Ok(false);
            }
            if self.layer_uid != other_info.layer_uid {
                return Ok(false);
            }
            if self.workpiece_uid != other_info.workpiece_uid {
                return Ok(false);
            }
            if self.section_type != other_info.section_type {
                return Ok(false);
            }
            if !py_pyany_eq(
                py,
                self.extra_axes.as_ref(),
                other_info.extra_axes.as_ref(),
            )? {
                return Ok(false);
            }
            if !py_pyany_eq(py, self.state.as_ref(), other_info.state.as_ref())?
            {
                return Ok(false);
            }
            if !py_pyany_eq(
                py,
                self.power_values.as_ref(),
                other_info.power_values.as_ref(),
            )? {
                return Ok(false);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn py_pyany_eq<T: pyo3::PyTypeInfo>(
    py: Python<'_>,
    a: Option<&Py<T>>,
    b: Option<&Py<T>>,
) -> PyResult<bool> {
    match (a, b) {
        (Some(a), Some(b)) => {
            let a_any = a.bind(py).as_any();
            let b_any = b.bind(py).as_any();
            a_any.eq(b_any)
        }
        (None, None) => Ok(true),
        _ => Ok(false),
    }
}

#[pyclass(module = "raygeo.ops", name = "Ops", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyOps {
    pub inner: raygeo_core::ops::Ops,
}

#[pymethods]
impl PyOps {
    #[new]
    pub fn new() -> Self {
        PyOps {
            inner: raygeo_core::ops::Ops::new(),
        }
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __add__(&self, other: &PyOps) -> PyOps {
        PyOps {
            inner: self.inner.ops_add(&other.inner),
        }
    }

    fn __mul__(&self, count: usize) -> PyOps {
        PyOps {
            inner: self.inner.ops_mul(count),
        }
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn command_type(&self, idx: isize) -> PyResult<PyCommandType> {
        let idx = normalize_index(idx, self.inner.len())?;
        Ok(PyCommandType(self.inner.command_type(idx)))
    }

    fn category(&self, idx: isize) -> PyResult<PyCommandCategory> {
        let idx = normalize_index(idx, self.inner.len())?;
        Ok(PyCommandCategory(self.inner.category(idx)))
    }

    fn is_travel(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(self.inner.is_travel(idx))
    }

    fn is_cutting(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(self.inner.is_cutting(idx))
    }

    fn is_state(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(self.inner.is_state(idx))
    }

    fn is_marker(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(self.inner.is_marker(idx))
    }

    fn is_scanline(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(self.inner.is_scanline(idx))
    }

    fn indices_of(&self, ct: &PyCommandType) -> Vec<usize> {
        self.inner.indices_of(ct.0)
    }

    #[pyo3(signature = (idx, last_point=None))]
    fn distance_at(
        &self,
        idx: usize,
        last_point: Option<(f64, f64, f64)>,
    ) -> f64 {
        self.inner.distance_at(idx, last_point)
    }

    fn distance(&self) -> f64 {
        self.inner.distance()
    }

    fn cut_distance(&self) -> f64 {
        self.inner.cut_distance()
    }

    #[getter]
    fn scanline_count(&self) -> usize {
        self.inner.scanline_count()
    }

    fn endpoint(&self, idx: isize) -> PyResult<(f64, f64, f64)> {
        let idx = normalize_index(idx, self.inner.len())?;
        Ok(self.inner.endpoint(idx))
    }

    fn arc_params(&self, idx: usize) -> PyResult<(f64, f64, bool)> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if self.inner.command_type(idx) != CommandType::ArcTo {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not an ArcToCommand",
            ));
        }
        Ok(*self.inner.arc_params(idx))
    }

    fn bezier_params(
        &self,
        idx: usize,
    ) -> PyResult<((f64, f64, f64), (f64, f64, f64))> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if self.inner.command_type(idx) != CommandType::BezierTo {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a BezierToCommand",
            ));
        }
        let bp = self.inner.bezier_params(idx);
        Ok((bp.0, bp.1))
    }

    fn quadratic_bezier_params(&self, idx: usize) -> PyResult<(f64, f64, f64)> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if self.inner.command_type(idx) != CommandType::QuadraticBezierTo {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a QuadraticBezierToCommand",
            ));
        }
        Ok(*self.inner.quad_params(idx))
    }

    fn scanline_data<'py>(
        &self,
        py: Python<'py>,
        idx: usize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if self.inner.command_type(idx) != CommandType::ScanLine {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a ScanLinePowerCommand",
            ));
        }
        let data = self.inner.scanline_data(idx);
        Ok(PyBytes::new(py, data))
    }

    fn dwell_duration(&self, idx: usize) -> PyResult<f64> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if self.inner.command_type(idx) != CommandType::Dwell {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a DwellCommand",
            ));
        }
        Ok(self.inner.dwell_duration(idx))
    }

    fn power(&self, idx: usize) -> PyResult<f64> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if self.inner.command_type(idx) != CommandType::SetPower {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetPowerCommand",
            ));
        }
        Ok(self.inner.power(idx))
    }

    fn speed(&self, idx: usize) -> PyResult<i32> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        let ct = self.inner.command_type(idx);
        if ct != CommandType::SetCutSpeed && ct != CommandType::SetTravelSpeed {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a speed command",
            ));
        }
        Ok(self.inner.speed(idx))
    }

    fn frequency(&self, idx: usize) -> PyResult<i32> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if self.inner.command_type(idx) != CommandType::SetFrequency {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetFrequencyCommand",
            ));
        }
        Ok(self.inner.frequency(idx))
    }

    fn pulse_width(&self, idx: usize) -> PyResult<f64> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if self.inner.command_type(idx) != CommandType::SetPulseWidth {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetPulseWidthCommand",
            ));
        }
        Ok(self.inner.pulse_width(idx))
    }

    fn laser_uid(&self, idx: usize) -> PyResult<String> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if self.inner.command_type(idx) != CommandType::SetLaser {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetLaserCommand",
            ));
        }
        Ok(self.inner.laser_uid(idx).to_string())
    }

    fn layer_uid(&self, idx: usize) -> PyResult<String> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        let ct = self.inner.command_type(idx);
        if ct != CommandType::LayerStart && ct != CommandType::LayerEnd {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a Layer command",
            ));
        }
        Ok(self.inner.layer_uid(idx).to_string())
    }

    fn workpiece_uid(&self, idx: usize) -> PyResult<String> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        let ct = self.inner.command_type(idx);
        if ct != CommandType::WorkpieceStart && ct != CommandType::WorkpieceEnd
        {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a Workpiece command",
            ));
        }
        Ok(self.inner.workpiece_uid(idx).to_string())
    }

    fn section_params(
        &self,
        idx: usize,
    ) -> PyResult<(PySectionType, Option<String>)> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        let ct = self.inner.command_type(idx);
        if ct == CommandType::OpsSectionStart {
            let st = PySectionType(self.inner.section_type(idx));
            let wu =
                self.inner.section_workpiece_uid(idx).map(|s| s.to_string());
            Ok((st, wu))
        } else if ct == CommandType::OpsSectionEnd {
            let st = PySectionType(self.inner.section_type(idx));
            Ok((st, None))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not an OpsSection command",
            ))
        }
    }

    fn extra_axes<'py>(
        &self,
        py: Python<'py>,
        idx: usize,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match self.inner.extra_axes(idx) {
            Some(axes) => Ok(Some(axis_map_to_py(py, axes)?)),
            None => Ok(None),
        }
    }

    fn preloaded_state(&self, idx: usize) -> PyResult<Option<PyState>> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match self.inner.preloaded_state(idx) {
            Some(s) => Ok(Some(PyState(s.clone()))),
            None => Ok(None),
        }
    }

    // --- Builder methods ---

    #[pyo3(signature = (x, y, z=0.0, extra=None))]
    fn move_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let ea = match extra {
            Some(ref d) => Some(py_to_axis_map(d)?),
            None => None,
        };
        self.inner.move_to(x, y, z, ea);
        Ok(())
    }

    #[pyo3(signature = (x, y, z=0.0, extra=None))]
    fn line_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        extra: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let ea = match extra {
            Some(ref d) => Some(py_to_axis_map(d)?),
            None => None,
        };
        self.inner.line_to(x, y, z, ea);
        Ok(())
    }

    fn close_path(&mut self) {
        self.inner.close_path();
    }

    #[pyo3(signature = (x, y, i, j, clockwise=true, z=0.0, extra=None))]
    fn arc_to(
        &mut self,
        x: f64,
        y: f64,
        i: f64,
        j: f64,
        clockwise: bool,
        z: f64,
        extra: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let ea = match extra {
            Some(ref d) => Some(py_to_axis_map(d)?),
            None => None,
        };
        self.inner.arc_to(x, y, i, j, clockwise, z, ea);
        Ok(())
    }

    #[pyo3(signature = (c1, c2, end, extra=None))]
    fn bezier_to(
        &mut self,
        c1: (f64, f64, f64),
        c2: (f64, f64, f64),
        end: (f64, f64, f64),
        extra: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let ea = match extra {
            Some(ref d) => Some(py_to_axis_map(d)?),
            None => None,
        };
        self.inner.bezier_to(c1, c2, end, ea);
        Ok(())
    }

    fn quadratic_bezier_to(
        &mut self,
        control: (f64, f64, f64),
        end: (f64, f64, f64),
        extra: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let ea = match extra {
            Some(ref d) => Some(py_to_axis_map(d)?),
            None => None,
        };
        self.inner.quadratic_bezier_to(control, end, ea);
        Ok(())
    }

    fn set_power(&mut self, power: f64) {
        self.inner.set_power(power);
    }

    fn set_cut_speed(&mut self, speed: i32) {
        self.inner.set_cut_speed(speed);
    }

    fn set_travel_speed(&mut self, speed: i32) {
        self.inner.set_travel_speed(speed);
    }

    fn dwell(&mut self, duration_ms: f64) {
        self.inner.dwell(duration_ms);
    }

    fn enable_air_assist(&mut self) {
        self.inner.enable_air_assist();
    }

    fn disable_air_assist(&mut self) {
        self.inner.disable_air_assist();
    }

    fn set_laser(&mut self, laser_uid: &str) {
        self.inner.set_laser(laser_uid);
    }

    fn set_frequency(&mut self, frequency: i32) {
        self.inner.set_frequency(frequency);
    }

    fn set_pulse_width(&mut self, pulse_width: f64) {
        self.inner.set_pulse_width(pulse_width);
    }

    #[pyo3(signature = (x, y, z=0.0, power_values=None, extra=None))]
    fn scan_to(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        power_values: Option<Vec<u8>>,
        extra: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let ea = match extra {
            Some(ref d) => Some(py_to_axis_map(d)?),
            None => None,
        };
        self.inner.scan_to(x, y, z, power_values, ea);
        Ok(())
    }

    fn job_start(&mut self) {
        self.inner.job_start();
    }

    fn job_end(&mut self) {
        self.inner.job_end();
    }

    fn layer_start(&mut self, layer_uid: &str) {
        self.inner.layer_start(layer_uid);
    }

    fn layer_end(&mut self, layer_uid: &str) {
        self.inner.layer_end(layer_uid);
    }

    fn workpiece_start(&mut self, workpiece_uid: &str) {
        self.inner.workpiece_start(workpiece_uid);
    }

    fn workpiece_end(&mut self, workpiece_uid: &str) {
        self.inner.workpiece_end(workpiece_uid);
    }

    fn ops_section_start(
        &mut self,
        section_type: &PySectionType,
        workpiece_uid: &str,
    ) {
        self.inner.ops_section_start(section_type.0, workpiece_uid);
    }

    fn ops_section_end(&mut self, section_type: &PySectionType) {
        self.inner.ops_section_end(section_type.0);
    }

    // --- Copy / Transfer ---

    fn copy(&self) -> PyOps {
        PyOps {
            inner: self.inner.copy(),
        }
    }

    fn copy_command_from(&mut self, source: &PyOps, idx: usize) {
        self.inner.copy_command_from(&source.inner, idx);
    }

    fn transfer_command_from(&mut self, source: &PyOps, idx: usize) {
        self.inner.transfer_command_from(&source.inner, idx);
    }

    fn extend(&mut self, other: Option<&PyOps>) {
        if let Some(other) = other {
            self.inner.extend(&other.inner);
        }
    }

    fn clear(&mut self) {
        self.inner.clear();
    }

    fn subpath_indices(&self) -> Vec<Vec<usize>> {
        self.inner.subpath_indices()
    }

    fn split_into_subpaths(&self) -> Vec<PyOps> {
        self.inner
            .split_into_subpaths()
            .into_iter()
            .map(|o| PyOps { inner: o })
            .collect()
    }

    fn flip_ops(&self) -> PyOps {
        PyOps {
            inner: self.inner.flip_ops(),
        }
    }

    fn without_state(&self) -> PyOps {
        PyOps {
            inner: self.inner.without_state(),
        }
    }

    fn segments(&self) -> Vec<Vec<usize>> {
        self.inner.segments()
    }

    fn state_at(&self, idx: usize) -> PyResult<PyState> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(PyState(self.inner.state_at(idx)))
    }

    fn sub_ops(&self, indices: Vec<usize>) -> PyOps {
        PyOps {
            inner: self.inner.sub_ops(&indices),
        }
    }

    fn replace_all(&mut self, source: &PyOps) {
        self.inner.replace_all(&source.inner);
    }

    fn replace_with(&mut self, source: &PyOps) {
        self.inner.replace_with(&source.inner);
    }

    #[classmethod]
    fn from_geometry(_cls: &Bound<'_, PyType>, geometry: &PyGeometry) -> Self {
        PyOps {
            inner: raygeo_core::ops::Ops::from_geometry(&geometry.inner),
        }
    }

    fn to_geometry(&self) -> PyGeometry {
        PyGeometry {
            inner: self.inner.to_geometry(),
        }
    }

    fn preload_state(&mut self) {
        self.inner.preload_state();
    }

    fn set_state_on_moving(&mut self, state: &PyState) {
        self.inner.set_state_on_moving(&state.0);
    }

    fn set_state_at(&mut self, idx: usize, state: &PyState) {
        self.inner.set_state_at(idx, &state.0);
    }

    fn dump(&self, py: Python<'_>) -> PyResult<()> {
        let output = self.inner.format_dump();
        let print_fn = py.import("builtins")?.getattr("print")?;
        for line in output.lines() {
            print_fn.call1((line,))?;
        }
        Ok(())
    }

    fn inspect(&self, py: Python<'_>, idx: usize) -> PyResult<PyCommandInfo> {
        let inner = &self.inner;
        let ct = inner.command_type(idx);
        let cat = inner.category(idx);

        let mut info = PyCommandInfo {
            type_: PyCommandType(ct),
            end: None,
            extra_axes: None,
            state: None,
            center_offset: None,
            clockwise: None,
            control1: None,
            control2: None,
            control: None,
            power_values: None,
            power: None,
            speed: None,
            frequency: None,
            pulse_width: None,
            laser_uid: None,
            duration_ms: None,
            layer_uid: None,
            workpiece_uid: None,
            section_type: None,
        };

        if cat == CommandCategory::Moving {
            info.end = Some(inner.endpoint(idx));
            if let Some(ea) = inner.extra_axes(idx) {
                let dict = PyDict::new(py);
                for &(axis, val) in ea {
                    let py_axis = Py::new(py, PyAxis(axis))?;
                    dict.set_item(py_axis, val)?;
                }
                info.extra_axes = Some(dict.unbind());
            }
            if let Some(s) = inner.soa.state(idx) {
                info.state = Some(Py::new(py, PyState(s.clone()))?);
            }
        }

        match ct {
            CommandType::ArcTo => {
                let &(i, j, cw) = inner.arc_params(idx);
                info.center_offset = Some((i, j));
                info.clockwise = Some(cw);
            }
            CommandType::BezierTo => {
                let &(c1, c2) = inner.bezier_params(idx);
                info.control1 = Some(c1);
                info.control2 = Some(c2);
            }
            CommandType::QuadraticBezierTo => {
                info.control = Some(*inner.quad_params(idx));
            }
            CommandType::ScanLine => {
                info.power_values =
                    Some(PyBytes::new(py, inner.scanline_data(idx)).unbind());
            }
            CommandType::SetPower => {
                info.power = Some(inner.power(idx));
            }
            CommandType::SetCutSpeed | CommandType::SetTravelSpeed => {
                info.speed = Some(inner.speed(idx));
            }
            CommandType::SetFrequency => {
                info.frequency = Some(inner.frequency(idx));
            }
            CommandType::SetPulseWidth => {
                info.pulse_width = Some(inner.pulse_width(idx));
            }
            CommandType::SetLaser => {
                info.laser_uid = Some(inner.laser_uid(idx).to_string());
            }
            CommandType::Dwell => {
                info.duration_ms = Some(inner.dwell_duration(idx));
            }
            CommandType::LayerStart | CommandType::LayerEnd => {
                info.layer_uid = Some(inner.layer_uid(idx).to_string());
            }
            CommandType::WorkpieceStart | CommandType::WorkpieceEnd => {
                info.workpiece_uid = Some(inner.workpiece_uid(idx).to_string());
            }
            CommandType::OpsSectionStart | CommandType::OpsSectionEnd => {
                info.section_type =
                    Some(format!("{:?}", inner.section_type(idx)));
                if let Some(wp) = inner.section_workpiece_uid(idx) {
                    info.workpiece_uid = Some(wp.to_string());
                }
            }
            _ => {}
        }

        Ok(info)
    }

    // --- Geometry transforms ---

    #[pyo3(signature = (dx, dy, dz = 0.0))]
    fn translate(&mut self, dx: f64, dy: f64, dz: f64) -> PyResult<()> {
        self.inner.translate(dx, dy, dz);
        Ok(())
    }

    fn scale(&mut self, sx: f64, sy: f64, sz: f64) -> PyResult<()> {
        self.inner.scale(sx, sy, sz);
        Ok(())
    }

    fn rotate(&mut self, angle_deg: f64, cx: f64, cy: f64) -> PyResult<()> {
        self.inner.rotate(angle_deg, cx, cy);
        Ok(())
    }

    fn transform(&mut self, matrix: Vec<Vec<f64>>) -> PyResult<()> {
        if matrix.len() != 4 || matrix.iter().any(|r| r.len() != 4) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "transform requires a 4x4 matrix",
            ));
        }
        let m: [[f64; 4]; 4] = [
            [matrix[0][0], matrix[0][1], matrix[0][2], matrix[0][3]],
            [matrix[1][0], matrix[1][1], matrix[1][2], matrix[1][3]],
            [matrix[2][0], matrix[2][1], matrix[2][2], matrix[2][3]],
            [matrix[3][0], matrix[3][1], matrix[3][2], matrix[3][3]],
        ];
        self.inner.transform(&m);
        Ok(())
    }

    // --- Clipping ---

    fn clip_rect(&self, rect: (f64, f64, f64, f64)) -> PyOps {
        PyOps {
            inner: self.inner.clip_rect(rect),
        }
    }

    fn subtract_regions(
        &mut self,
        regions: Vec<Vec<(f64, f64)>>,
    ) -> PyResult<()> {
        self.inner.subtract_regions(&regions);
        Ok(())
    }

    #[pyo3(signature = (regions, tolerance = 0.3))]
    fn clip_to_regions(
        &mut self,
        regions: Vec<Vec<(f64, f64)>>,
        tolerance: f64,
    ) -> PyResult<()> {
        self.inner.clip_to_regions(&regions, tolerance);
        Ok(())
    }

    #[pyo3(signature = (regions, tolerance = 0.3))]
    fn clip_ops_to_regions(
        &mut self,
        regions: Vec<Vec<(f64, f64)>>,
        tolerance: f64,
    ) -> PyResult<()> {
        self.inner.clip_ops_to_regions(&regions, tolerance);
        Ok(())
    }

    fn clip_at(&mut self, x: f64, y: f64, width: f64) -> bool {
        self.inner.clip_at(x, y, width)
    }

    #[pyo3(signature = (default_offset, layer_offsets = None))]
    fn translate_layers(
        &mut self,
        default_offset: (f64, f64, f64),
        layer_offsets: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let layer_offsets_rust: Option<Vec<(String, (f64, f64, f64))>> =
            if let Some(dict) = layer_offsets {
                let mut v = Vec::new();
                for item in dict.iter() {
                    let (key, val) = item;
                    let key_str: String = key.extract()?;
                    let val_tuple: (f64, f64, f64) = val.extract()?;
                    v.push((key_str, val_tuple));
                }
                Some(v)
            } else {
                None
            };
        self.inner.translate_layers(
            default_offset,
            layer_offsets_rust.as_ref().map(|v| v.as_slice()),
        );
        Ok(())
    }

    fn transform_layers(
        &mut self,
        py: Python<'_>,
        callback: Py<PyAny>,
    ) -> PyResult<()> {
        use raygeo_core::ops::SoA;
        let mut i = 0;
        while i < self.inner.len() {
            if self.inner.command_type(i) != CommandType::LayerStart {
                i += 1;
                continue;
            }

            let layer_uid = self.inner.layer_uid(i).to_string();
            let layer_start = i;
            let mut collected_indices: Vec<usize> = Vec::new();
            while i < self.inner.len() {
                collected_indices.push(i);
                i += 1;
                if self.inner.command_type(i - 1) == CommandType::LayerEnd {
                    break;
                }
            }
            let layer_end = i;

            let layer_ops = self.inner.sub_ops(&collected_indices);
            let py_layer_ops = Py::new(py, PyOps { inner: layer_ops })?;
            callback.call1(py, (layer_uid, &py_layer_ops))?;
            let layer_ops_ref = py_layer_ops.borrow(py);
            let layer_ops = &layer_ops_ref.inner;

            let mut new_soa = SoA::new();
            for j in 0..layer_start {
                let args = self.inner.soa.deep_copy_entry(j);
                SoA::append_from_args(&mut new_soa, &args);
            }
            for j in 0..layer_ops.len() {
                let args = layer_ops.soa.deep_copy_entry(j);
                SoA::append_from_args(&mut new_soa, &args);
            }
            for j in layer_end..self.inner.len() {
                let args = self.inner.soa.deep_copy_entry(j);
                SoA::append_from_args(&mut new_soa, &args);
            }
            self.inner.soa = new_soa;
            i = layer_start + layer_ops.len();
        }
        self.inner.invalidate_time_cache();
        Ok(())
    }

    #[pyo3(signature = (on_endpoint, on_aux_point = None))]
    fn transform_moving(
        &mut self,
        py: Python<'_>,
        on_endpoint: Py<PyAny>,
        on_aux_point: Option<Py<PyAny>>,
    ) -> PyResult<()> {
        for i in 0..self.inner.len() {
            if self.inner.category(i) != CommandCategory::Moving {
                continue;
            }

            let ct = self.inner.command_type(i);
            let end = self.inner.endpoint(i);
            let end_list = vec![end.0, end.1, end.2];
            let end_py_list = PyList::new(py, &end_list)?;

            let ea = self.inner.extra_axes(i);
            let ea_arg = if let Some(axes) = ea {
                axis_map_to_py(py, axes)?
            } else {
                PyDict::new(py)
            };

            on_endpoint.call1(py, (&end_py_list, &ea_arg))?;

            let new_end: Vec<f64> = end_py_list.extract()?;
            self.inner
                .soa
                .set_endpoint(i, (new_end[0], new_end[1], new_end[2]));

            if !ea_arg.is_empty() {
                let ea_vec = py_to_axis_map(&ea_arg)?;
                self.inner.soa.set_extra_axes(i, ea_vec);
            }

            if let Some(ref aux_cb) = on_aux_point {
                if ct == CommandType::ArcTo {
                    let (ci, cj, cw) = *self.inner.arc_params(i);
                    let off_list = vec![ci, cj];
                    let off_py_list = PyList::new(py, &off_list)?;
                    aux_cb.call1(py, (&off_py_list,))?;
                    let new_off: Vec<f64> = off_py_list.extract()?;
                    self.inner.soa.set_arc_params(
                        i,
                        Some((new_off[0], new_off[1])),
                        Some(cw),
                    );
                } else if ct == CommandType::BezierTo {
                    let &(c1, c2) = self.inner.bezier_params(i);
                    for (cp_idx, cp) in [c1, c2].iter().enumerate() {
                        let cp_list = vec![cp.0, cp.1, cp.2];
                        let cp_py_list = PyList::new(py, &cp_list)?;
                        aux_cb.call1(py, (&cp_py_list,))?;
                        let new_cp: Vec<f64> = cp_py_list.extract()?;
                        if cp_idx == 0 {
                            let (_, c2) = *self.inner.bezier_params(i);
                            self.inner.soa.set_bezier_params(
                                i,
                                (new_cp[0], new_cp[1], new_cp[2]),
                                c2,
                            );
                        } else {
                            let (c1, _) = *self.inner.bezier_params(i);
                            self.inner.soa.set_bezier_params(
                                i,
                                c1,
                                (new_cp[0], new_cp[1], new_cp[2]),
                            );
                        }
                    }
                } else if ct == CommandType::QuadraticBezierTo {
                    let c = *self.inner.quad_params(i);
                    let cp_list = vec![c.0, c.1, c.2];
                    let cp_py_list = PyList::new(py, &cp_list)?;
                    aux_cb.call1(py, (&cp_py_list,))?;
                    let new_cp: Vec<f64> = cp_py_list.extract()?;
                    self.inner
                        .soa
                        .set_quad_params(i, (new_cp[0], new_cp[1], new_cp[2]));
                }
            }
        }
        self.inner.invalidate_time_cache();
        Ok(())
    }

    fn linearize(
        &self,
        idx: usize,
        start_point: (f64, f64, f64),
    ) -> PyResult<Self> {
        let ct = self.inner.command_type(idx);
        match ct {
            CommandType::ScanLine
            | CommandType::ArcTo
            | CommandType::BezierTo
            | CommandType::QuadraticBezierTo
            | CommandType::MoveTo
            | CommandType::LineTo => Ok(PyOps {
                inner: self.inner.linearize(idx, start_point),
            }),
            _ => Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "Cannot linearize command at index {}: {:?}",
                idx, ct,
            ))),
        }
    }

    fn linearize_all(&mut self) {
        self.inner.linearize_all();
    }

    fn linearize_curves(&mut self) {
        self.inner.linearize_curves();
    }

    fn linearize_arcs(&mut self) {
        self.inner.linearize_arcs();
    }

    fn segment_indices(&self) -> Vec<Vec<usize>> {
        self.inner.segment_indices()
    }

    fn group_by_state_continuity(&self) -> Vec<PyOps> {
        self.inner
            .group_by_state_continuity()
            .into_iter()
            .map(|o| PyOps { inner: o })
            .collect()
    }

    #[pyo3(signature = (include_travel = false))]
    fn rect(&self, include_travel: bool) -> (f64, f64, f64, f64) {
        self.inner.rect(include_travel)
    }

    #[pyo3(signature = (power = None, speed = None))]
    fn get_frame(&self, power: Option<f64>, speed: Option<f64>) -> PyOps {
        PyOps {
            inner: self.inner.get_frame(power, speed),
        }
    }

    #[pyo3(signature = (default_cut_speed = 1000.0, default_travel_speed = 3000.0, acceleration = 1000.0))]
    fn estimate_time(
        &mut self,
        default_cut_speed: f64,
        default_travel_speed: f64,
        acceleration: f64,
    ) -> f64 {
        self.inner.estimate_time(
            default_cut_speed,
            default_travel_speed,
            acceleration,
        )
    }

    // --- Properties ---

    #[getter]
    fn get_last_move_to(&self) -> (f64, f64, f64) {
        self.inner.last_move_to
    }

    #[setter]
    fn set_last_move_to(&mut self, val: (f64, f64, f64)) {
        self.inner.last_move_to = val;
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        super::serialize::ops_to_dict(py, &self.inner)
    }

    #[classmethod]
    fn from_dict(
        _cls: &Bound<'_, PyType>,
        data: &Bound<'_, PyDict>,
    ) -> PyResult<Self> {
        let inner = super::serialize::ops_from_dict(data)?;
        Ok(PyOps { inner })
    }

    fn to_numpy_arrays(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        super::serialize::ops_to_numpy_arrays(py, &self.inner)
    }

    #[classmethod]
    fn from_numpy_arrays(
        _cls: &Bound<'_, PyType>,
        arrays: &Bound<'_, PyDict>,
    ) -> PyResult<Self> {
        let inner = super::serialize::ops_from_numpy_arrays(arrays)?;
        Ok(PyOps { inner })
    }

    #[getter]
    #[allow(non_snake_case)]
    fn get__time_dirty(&self) -> bool {
        self.inner.time_dirty
    }

    #[getter]
    #[allow(non_snake_case)]
    fn get__cached_time(&self) -> f64 {
        self.inner.cached_time
    }

    #[getter]
    #[allow(non_snake_case)]
    fn get__time_params(&self) -> Option<(f64, f64, f64)> {
        self.inner.time_params
    }
}
