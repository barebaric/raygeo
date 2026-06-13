use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyType};
use pyo3::{Bound, Py, PyAny, PyResult};
use pyo3_stub_gen::derive::{
    gen_methods_from_python, gen_stub_pyclass, gen_stub_pymethods,
};
use pyo3_stub_gen::inventory::submit;

use crate::ops::{
    Axis, CommandType, MarkerCmd, MoveCmd, OpCategory, OpsSection,
    OpsSectionRange, StateCmd,
};
use crate::types::Rect;

use super::axis::PyAxis;
use super::state::PyState;
use super::types::{PyCommandCategory, PyCommandType, PySectionType};
use crate::python::geo::geometry::Geometry as PyGeometry;

/// Normalize a Python-style index (negative = from end) to a usize.
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

/// Thin wrapper around :func:`serialize::py_to_axis_map_helper`.
fn py_to_axis_map(dict: &Bound<'_, PyDict>) -> PyResult<Vec<(Axis, f64)>> {
    super::serialize::py_to_axis_map_helper(dict)
}

/// Thin wrapper around :func:`serialize::axis_map_to_py_helper`.
fn axis_map_to_py<'a>(
    py: Python<'a>,
    axes: &[(Axis, f64)],
) -> PyResult<Bound<'a, PyDict>> {
    super::serialize::axis_map_to_py_helper(py, axes)
}

/// A section of operations parsed into marker and content index groups.
///
/// Produced by :meth:`Ops.sections` when splitting an Ops sequence
/// into logical sections based on ``OpsSectionStart``/``OpsSectionEnd`` markers.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops", name = "OpsSection", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOpsSection(pub OpsSection);

#[gen_stub_pymethods]
#[pymethods]
impl PyOpsSection {
    /// The type of this section (VectorOutline or RasterFill), if any.
    #[getter]
    fn section_type(&self) -> Option<PySectionType> {
        self.0.section_type.map(PySectionType)
    }

    /// Indices of the section-marker commands (start/end) for this section.
    #[getter]
    fn marker_indices(&self) -> Vec<usize> {
        self.0.marker_indices.clone()
    }

    /// Indices of the content commands belonging to this section.
    #[getter]
    fn content_indices(&self) -> Vec<usize> {
        self.0.content_indices.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "OpsSection(section_type={:?}, marker_indices={:?}, content_indices={:?})",
            self.0.section_type, self.0.marker_indices, self.0.content_indices
        )
    }
}

/// A contiguous range of indices that belong to a section.
///
/// Similar to :class:`OpsSection` but stores start/end index ranges
/// instead of individual index lists. Produced by :meth:`Ops.section_ranges`.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops", name = "OpsSectionRange", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOpsSectionRange(pub OpsSectionRange);

#[gen_stub_pymethods]
#[pymethods]
impl PyOpsSectionRange {
    /// The type of this section range (VectorOutline or RasterFill), if any.
    #[getter]
    fn section_type(&self) -> Option<PySectionType> {
        self.0.section_type.map(PySectionType)
    }

    /// Indices of the section-marker commands that bracket this range.
    #[getter]
    fn marker_indices(&self) -> Vec<usize> {
        self.0.marker_indices.clone()
    }

    /// Starting index of the content within this section range.
    #[getter]
    fn content_indices(&self) -> Vec<usize> {
        self.0.content_indices.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "OpsSectionRange(section_type={:?}, marker_indices={:?}, content_indices={:?})",
            self.0.section_type, self.0.marker_indices, self.0.content_indices
        )
    }
}

/// Detailed information about a single command in an Ops sequence.
///
/// Returned by :meth:`Ops.inspect` and provides the full set of
/// parameters for any command type in a structured form.
#[gen_stub_pyclass]
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

#[gen_stub_pymethods]
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

/// A sequence of laser cutting operations (commands).
///
/// ``Ops`` is a container of ordered commands that define a complete
/// laser engraving or cutting job. It supports building command sequences
/// programmatically, transforming them, clipping, serializing, and more.
///
/// Use the builder methods (``move_to``, ``line_to``, ``arc_to``, etc.)
/// to construct a sequence, or load from geometry/dict/numpy arrays.
#[gen_stub_pyclass]
#[pyclass(dict, module = "raygeo.ops", name = "Ops", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyOps {
    pub inner: crate::ops::Ops,
}

submit! {
    gen_methods_from_python! {
        r#"
        from raygeo import geo

        class PyOps:
            def transform(self, matrix: geo.types.TransformMatrix) -> None:
                """Apply a 4x4 affine transformation matrix to all geometry.

                See ``geo.types.TransformMatrix`` for the matrix layout.

                :param matrix: A 4x4 affine transformation matrix.
                """
                ...
        "#
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyOps {
    /// Create a new, empty Ops sequence.
    #[new]
    pub fn new() -> Self {
        PyOps {
            inner: crate::ops::Ops::new(),
        }
    }

    /// Return the number of commands.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Concatenate two Ops sequences (``ops1 + ops2``).
    fn __add__(&self, other: &PyOps) -> PyOps {
        PyOps {
            inner: self.inner.ops_add(&other.inner),
        }
    }

    /// Repeat the ops sequence *count* times (``ops * n``).
    fn __mul__(&self, count: usize) -> PyOps {
        PyOps {
            inner: self.inner.ops_mul(count),
        }
    }

    /// Check if the ops sequence is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Return the number of commands.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Get the :class:`CommandType` at the given index.
    ///
    /// :param idx: Command index (negative = from end).
    /// :returns: The :class:`CommandType` of the command.
    fn command_type(&self, idx: isize) -> PyResult<PyCommandType> {
        let idx = normalize_index(idx, self.inner.len())?;
        Ok(PyCommandType(self.inner.commands[idx].command_type()))
    }

    /// Get the :class:`CommandCategory` at the given index.
    ///
    /// :param idx: Command index (negative = from end).
    /// :returns: The category (MOVING, STATE, or MARKER).
    fn category(&self, idx: isize) -> PyResult<PyCommandCategory> {
        let idx = normalize_index(idx, self.inner.len())?;
        Ok(PyCommandCategory(
            self.inner.commands[idx].command_type().category(),
        ))
    }

    /// Check whether the command at *idx* is a travel (non-cutting) move.
    ///
    /// :param idx: Command index.
    /// :returns: True if the command is a travel move.
    fn is_travel(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(matches!(
            self.inner.commands[idx].category,
            OpCategory::Moving {
                cmd: MoveCmd::MoveTo,
                ..
            }
        ))
    }

    /// Check whether the command at *idx* is a cutting move.
    ///
    /// :param idx: Command index.
    /// :returns: True if the command is a cutting move.
    fn is_cutting(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(matches!(
            self.inner.commands[idx].category,
            OpCategory::Moving {
                cmd: MoveCmd::LineTo
                    | MoveCmd::ArcTo { .. }
                    | MoveCmd::BezierTo { .. }
                    | MoveCmd::QuadraticBezierTo { .. }
                    | MoveCmd::ScanLine { .. },
                ..
            }
        ))
    }

    /// Check whether the command at *idx* is a state command.
    ///
    /// :param idx: Command index.
    /// :returns: True if the command modifies machine state.
    fn is_state(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(self.inner.commands[idx].is_state_cmd())
    }

    /// Check whether the command at *idx* is a marker command.
    ///
    /// :param idx: Command index.
    /// :returns: True if the command is a structural marker (JobStart, LayerStart, etc.).
    fn is_marker(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(self.inner.commands[idx].is_marker())
    }

    /// Check whether the command at *idx* is a scanline command.
    ///
    /// :param idx: Command index.
    /// :returns: True if the command is a ScanLine power command.
    fn is_scanline(&self, idx: usize) -> PyResult<bool> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(self.inner.commands[idx].command_type() == CommandType::ScanLine)
    }

    /// Return all indices where the command type matches *ct*.
    ///
    /// :param ct: The :class:`CommandType` to search for.
    /// :returns: List of matching command indices.
    fn indices_of(&self, ct: &PyCommandType) -> Vec<usize> {
        self.inner
            .commands
            .iter()
            .enumerate()
            .filter(|(_, node)| node.command_type() == ct.0)
            .map(|(i, _)| i)
            .collect()
    }

    /// Compute the distance traveled up to command *idx*.
    ///
    /// :param idx: Command index.
    /// :param last_point: Optional starting point override.
    /// :returns: Cumulative distance.
    #[pyo3(signature = (idx, last_point=None))]
    fn distance_at(
        &self,
        idx: usize,
        last_point: Option<(f64, f64, f64)>,
    ) -> f64 {
        if let OpCategory::Moving { end, .. } =
            &self.inner.commands[idx].category
        {
            match last_point {
                None => 0.0,
                Some(lp) => {
                    let dx = end.0 - lp.0;
                    let dy = end.1 - lp.1;
                    (dx * dx + dy * dy).sqrt()
                }
            }
        } else {
            0.0
        }
    }

    /// Compute the total distance of all commands.
    fn distance(&self) -> f64 {
        self.inner.distance()
    }

    /// Compute the total cutting distance (excluding travel moves).
    fn cut_distance(&self) -> f64 {
        self.inner.cut_distance()
    }

    /// Return the number of scanline commands in the sequence.
    #[getter]
    fn scanline_count(&self) -> usize {
        self.inner
            .commands
            .iter()
            .filter(|node| {
                matches!(
                    node.category,
                    OpCategory::Moving {
                        cmd: MoveCmd::ScanLine { .. },
                        ..
                    }
                )
            })
            .count()
    }

    /// Get the endpoint coordinates of a moving command.
    ///
    /// :param idx: Command index (negative = from end).
    /// :returns: ``(x, y, z)`` tuple.
    fn endpoint(&self, idx: isize) -> PyResult<(f64, f64, f64)> {
        let idx = normalize_index(idx, self.inner.len())?;
        Ok(self.inner.commands[idx].end_point())
    }

    /// Get the arc parameters (center offset i, j, and clockwise flag).
    ///
    /// :param idx: Command index.
    /// :returns: ``(i, j, clockwise)`` tuple.
    /// :raises TypeError: If the command is not an ArcTo.
    fn arc_params(&self, idx: usize) -> PyResult<(f64, f64, bool)> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::Moving {
            cmd: MoveCmd::ArcTo { center, cw },
            ..
        } = &self.inner.commands[idx].category
        {
            Ok((center.0, center.1, *cw))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not an ArcToCommand",
            ))
        }
    }

    /// Get the cubic bezier control points.
    ///
    /// :param idx: Command index.
    /// :returns: ``((c1x, c1y, c1z), (c2x, c2y, c2z))`` control points.
    /// :raises TypeError: If the command is not a BezierTo.
    #[allow(clippy::type_complexity)]
    fn bezier_params(
        &self,
        idx: usize,
    ) -> PyResult<((f64, f64, f64), (f64, f64, f64))> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::Moving {
            cmd: MoveCmd::BezierTo { control1, control2 },
            ..
        } = &self.inner.commands[idx].category
        {
            Ok((*control1, *control2))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a BezierToCommand",
            ))
        }
    }

    /// Get the quadratic bezier control point.
    ///
    /// :param idx: Command index.
    /// :returns: ``(cx, cy, cz)`` control point.
    /// :raises TypeError: If the command is not a QuadraticBezierTo.
    fn quadratic_bezier_params(&self, idx: usize) -> PyResult<(f64, f64, f64)> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::Moving {
            cmd: MoveCmd::QuadraticBezierTo { control },
            ..
        } = &self.inner.commands[idx].category
        {
            Ok(*control)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a QuadraticBezierToCommand",
            ))
        }
    }

    /// Get the raw scanline power data for a scanline command.
    ///
    /// :param idx: Command index.
    /// :returns: Raw bytes of scanline power data.
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
        if let OpCategory::Moving {
            cmd: MoveCmd::ScanLine { power_values },
            ..
        } = &self.inner.commands[idx].category
        {
            Ok(PyBytes::new(py, power_values.as_ref()))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a ScanLinePowerCommand",
            ))
        }
    }

    /// Get the duration (milliseconds) of a Dwell command.
    ///
    /// :param idx: Command index.
    /// :returns: Duration in milliseconds.
    /// :raises TypeError: If the command is not a Dwell.
    fn dwell_duration(&self, idx: usize) -> PyResult<f64> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::Dwell(d)) =
            &self.inner.commands[idx].category
        {
            Ok(*d)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a DwellCommand",
            ))
        }
    }

    /// Get the power level of a SetPower command.
    ///
    /// :param idx: Command index.
    /// :returns: Power level (0.0–1.0 typically).
    /// :raises TypeError: If the command is not a SetPower.
    fn power(&self, idx: usize) -> PyResult<f64> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::SetPower(p)) =
            &self.inner.commands[idx].category
        {
            Ok(*p)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetPowerCommand",
            ))
        }
    }

    /// Get the speed value from a SetCutSpeed or SetTravelSpeed command.
    ///
    /// :param idx: Command index.
    /// :returns: Speed in mm/s.
    /// :raises TypeError: If the command is not a speed command.
    fn speed(&self, idx: usize) -> PyResult<i32> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match &self.inner.commands[idx].category {
            OpCategory::State(StateCmd::SetCutSpeed(s))
            | OpCategory::State(StateCmd::SetTravelSpeed(s)) => Ok(*s),
            _ => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a speed command",
            )),
        }
    }

    /// Get the frequency of a SetFrequency command.
    ///
    /// :param idx: Command index.
    /// :returns: Frequency in Hz.
    /// :raises TypeError: If the command is not a SetFrequency.
    fn frequency(&self, idx: usize) -> PyResult<i32> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::SetFrequency(f)) =
            &self.inner.commands[idx].category
        {
            Ok(*f)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetFrequencyCommand",
            ))
        }
    }

    /// Get the pulse width of a SetPulseWidth command.
    ///
    /// :param idx: Command index.
    /// :returns: Pulse width in microseconds.
    /// :raises TypeError: If the command is not a SetPulseWidth.
    fn pulse_width(&self, idx: usize) -> PyResult<f64> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::SetPulseWidth(pw)) =
            &self.inner.commands[idx].category
        {
            Ok(*pw)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetPulseWidthCommand",
            ))
        }
    }

    /// Get the laser UID from a SetLaser command.
    ///
    /// :param idx: Command index.
    /// :returns: The laser source identifier.
    /// :raises TypeError: If the command is not a SetLaser.
    fn laser_uid(&self, idx: usize) -> PyResult<String> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::SetLaser(uid)) =
            &self.inner.commands[idx].category
        {
            Ok(uid.to_string())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetLaserCommand",
            ))
        }
    }

    /// Get the layer UID from a LayerStart or LayerEnd command.
    ///
    /// :param idx: Command index.
    /// :returns: The layer identifier.
    /// :raises TypeError: If the command is not a Layer command.
    fn layer_uid(&self, idx: usize) -> PyResult<String> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match &self.inner.commands[idx].category {
            OpCategory::Marker(MarkerCmd::LayerStart(uid))
            | OpCategory::Marker(MarkerCmd::LayerEnd(uid)) => {
                Ok(uid.to_string())
            }
            _ => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a Layer command",
            )),
        }
    }

    /// Get the workpiece UID from a WorkpieceStart or WorkpieceEnd command.
    ///
    /// :param idx: Command index.
    /// :returns: The workpiece identifier.
    /// :raises TypeError: If the command is not a Workpiece command.
    fn workpiece_uid(&self, idx: usize) -> PyResult<String> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match &self.inner.commands[idx].category {
            OpCategory::Marker(MarkerCmd::WorkpieceStart(uid))
            | OpCategory::Marker(MarkerCmd::WorkpieceEnd(uid)) => {
                Ok(uid.to_string())
            }
            _ => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a Workpiece command",
            )),
        }
    }

    /// Get the section type and optional workpiece UID from an OpsSection command.
    ///
    /// :param idx: Command index.
    /// :returns: ``(SectionType, Optional[workpiece_uid])``.
    /// :raises TypeError: If the command is not an OpsSectionStart or OpsSectionEnd.
    fn section_params(
        &self,
        idx: usize,
    ) -> PyResult<(PySectionType, Option<String>)> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match &self.inner.commands[idx].category {
            OpCategory::Marker(MarkerCmd::OpsSectionStart {
                section_type,
                workpiece_uid,
            }) => Ok((
                PySectionType(*section_type),
                workpiece_uid.as_ref().map(|s| s.to_string()),
            )),
            OpCategory::Marker(MarkerCmd::OpsSectionEnd {
                section_type,
                ..
            }) => Ok((PySectionType(*section_type), None)),
            _ => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not an OpsSection command",
            )),
        }
    }

    /// Get the extra axes data for a moving command.
    ///
    /// :param idx: Command index.
    /// :returns: Dict mapping axis names to values, or None.
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
        match self.inner.commands[idx].extra_axes() {
            Some(axes) => Ok(Some(axis_map_to_py(py, axes)?)),
            None => Ok(None),
        }
    }

    /// Get the preloaded machine state for a moving command (if available).
    ///
    /// The preloaded state is the state that was in effect at the time
    /// this command was created (after calling :meth:`preload_state`).
    ///
    /// :param idx: Command index.
    /// :returns: The :class:`State` at that index, or None.
    fn preloaded_state(&self, idx: usize) -> PyResult<Option<PyState>> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match self.inner.commands[idx].state() {
            Some(s) => Ok(Some(PyState(s.clone()))),
            None => Ok(None),
        }
    }

    // --- Builder methods ---

    /// Add a rapid (non-cutting) move to the given coordinates.
    ///
    /// :param x: X coordinate.
    /// :param y: Y coordinate.
    /// :param z: Z coordinate (default 0.0).
    /// :param extra: Optional dict of extra axis values.
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

    /// Add a cutting line to the given coordinates.
    ///
    /// :param x: X coordinate.
    /// :param y: Y coordinate.
    /// :param z: Z coordinate (default 0.0).
    /// :param extra: Optional dict of extra axis values.
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

    /// Close the current sub-path by adding a line back to the start.
    fn close_path(&mut self) {
        self.inner.close_path();
    }

    /// Add a circular arc to the given coordinates.
    ///
    /// :param x: End X coordinate.
    /// :param y: End Y coordinate.
    /// :param i: I offset from current point to arc center.
    /// :param j: J offset from current point to arc center.
    /// :param clockwise: Whether the arc is clockwise (default True).
    /// :param z: End Z coordinate (default 0.0).
    /// :param extra: Optional dict of extra axis values.
    #[pyo3(signature = (x, y, i, j, clockwise=true, z=0.0, extra=None))]
    #[allow(clippy::too_many_arguments)]
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

    /// Add a cubic bezier curve to the given endpoint.
    ///
    /// :param control1: First control point ``(x, y, z)``.
    /// :param control2: Second control point ``(x, y, z)``.
    /// :param end: End point ``(x, y, z)``.
    /// :param extra: Optional dict of extra axis values.
    #[pyo3(signature = (control1, control2, end, extra=None))]
    fn bezier_to(
        &mut self,
        control1: (f64, f64, f64),
        control2: (f64, f64, f64),
        end: (f64, f64, f64),
        extra: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let ea = match extra {
            Some(ref d) => Some(py_to_axis_map(d)?),
            None => None,
        };
        self.inner.bezier_to(control1, control2, end, ea);
        Ok(())
    }

    /// Add a quadratic bezier curve to the given endpoint.
    ///
    /// :param control: Control point ``(x, y, z)``.
    /// :param end: End point ``(x, y, z)``.
    /// :param extra: Optional dict of extra axis values.
    #[pyo3(signature = (control, end, extra=None))]
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

    /// Set the laser power for subsequent commands.
    ///
    /// :param power: Power level (0.0–1.0).
    fn set_power(&mut self, power: f64) {
        self.inner.set_power(power);
    }

    /// Set the cutting speed for subsequent commands.
    ///
    /// :param speed: Cutting speed in units per second.
    fn set_cut_speed(&mut self, speed: f64) {
        self.inner.set_cut_speed(speed as i32);
    }

    /// Set the travel (rapid) speed for subsequent commands.
    ///
    /// :param speed: Travel speed in units per second.
    fn set_travel_speed(&mut self, speed: f64) {
        self.inner.set_travel_speed(speed as i32);
    }

    /// Pause execution for a given duration.
    ///
    /// :param duration_ms: Dwell duration in milliseconds.
    fn dwell(&mut self, duration_ms: f64) {
        self.inner.dwell(duration_ms);
    }

    /// Enable air assist for subsequent cutting.
    ///
    /// :param enabled: Whether to enable air assist (default True).
    #[pyo3(signature = (enabled = true))]
    fn enable_air_assist(&mut self, enabled: bool) {
        self.inner.enable_air_assist(enabled);
    }

    /// Switch to a specific laser by UID.
    ///
    /// :param laser_uid: The laser identifier.
    fn set_laser(&mut self, laser_uid: &str) {
        self.inner.set_laser(laser_uid);
    }

    /// Set the laser pulse frequency.
    ///
    /// :param frequency: Frequency in Hz.
    fn set_frequency(&mut self, frequency: i32) {
        self.inner.set_frequency(frequency);
    }

    /// Set the laser pulse width.
    ///
    /// :param pulse_width: Pulse width in microseconds.
    fn set_pulse_width(&mut self, pulse_width: f64) {
        self.inner.set_pulse_width(pulse_width);
    }

    /// Add a scan-line move with per-pixel power values.
    ///
    /// :param x: End X coordinate.
    /// :param y: End Y coordinate.
    /// :param z: End Z coordinate (default 0.0).
    /// :param power_values: Optional per-pixel 8-bit power values.
    /// :param extra: Optional dict of extra axis values.
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
        let pv = power_values.unwrap_or_else(|| vec![255]);
        self.inner.scan_to(x, y, z, pv, ea);
        Ok(())
    }

    /// Mark the start of a job.
    fn job_start(&mut self) {
        self.inner.job_start();
    }

    /// Mark the end of a job.
    fn job_end(&mut self) {
        self.inner.job_end();
    }

    /// Mark the start of a layer.
    ///
    /// :param layer_uid: The layer identifier.
    fn layer_start(&mut self, layer_uid: &str) {
        self.inner.layer_start(layer_uid);
    }

    /// Mark the end of a layer.
    ///
    /// :param layer_uid: The layer identifier.
    fn layer_end(&mut self, layer_uid: &str) {
        self.inner.layer_end(layer_uid);
    }

    /// Mark the start of a workpiece.
    ///
    /// :param workpiece_uid: The workpiece identifier.
    fn workpiece_start(&mut self, workpiece_uid: &str) {
        self.inner.workpiece_start(workpiece_uid);
    }

    /// Mark the end of a workpiece.
    ///
    /// :param workpiece_uid: The workpiece identifier.
    fn workpiece_end(&mut self, workpiece_uid: &str) {
        self.inner.workpiece_end(workpiece_uid);
    }

    /// Mark the start of an ops section.
    ///
    /// :param section_type: The type of section.
    /// :param workpiece_uid: The workpiece identifier.
    fn ops_section_start(
        &mut self,
        section_type: &PySectionType,
        workpiece_uid: &str,
    ) {
        self.inner.ops_section_start(section_type.0, workpiece_uid);
    }

    /// Mark the end of an ops section.
    ///
    /// :param section_type: The type of section.
    fn ops_section_end(&mut self, section_type: &PySectionType) {
        self.inner.ops_section_end(section_type.0);
    }

    // --- Copy / Transfer ---

    /// Return a deep copy of this Ops sequence.
    fn copy(&self) -> PyOps {
        PyOps {
            inner: self.inner.copy(),
        }
    }

    /// Shallow copy (same as :meth:`copy` since Ops is immutable).
    fn __copy__(&self) -> PyOps {
        self.copy()
    }

    /// Deep copy (same as :meth:`copy` since Ops is immutable).
    fn __deepcopy__(&self, _memo: Bound<'_, PyAny>) -> PyOps {
        self.copy()
    }

    /// Copy a single command from another Ops sequence into this one.
    ///
    /// :param source: The source Ops sequence.
    /// :param idx: Index of the command to copy.
    fn copy_command_from(&mut self, source: &PyOps, idx: usize) {
        self.inner.copy_command_from(&source.inner, idx);
    }

    /// Transfer (move) a single command from another Ops sequence into this one.
    ///
    /// :param source: The source Ops sequence.
    /// :param idx: Index of the command to transfer.
    fn transfer_command_from(&mut self, source: &PyOps, idx: usize) {
        self.inner.transfer_command_from(&source.inner, idx);
    }

    /// Extend this Ops sequence with commands from another.
    ///
    /// :param other: The other Ops sequence (or None for no-op).
    fn extend(&mut self, other: Option<&PyOps>) {
        if let Some(other) = other {
            self.inner.extend(&other.inner);
        }
    }

    /// Remove all commands from this Ops sequence.
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Return index ranges for each subpath.
    ///
    /// :returns: A list of index lists, one per subpath.
    fn subpath_indices(&self) -> Vec<Vec<usize>> {
        self.inner.subpath_indices()
    }

    /// Split this Ops sequence into separate subpaths.
    ///
    /// :returns: A list of Ops sequences, one per subpath.
    fn split_into_subpaths(&self) -> Vec<PyOps> {
        self.inner
            .split_into_subpaths()
            .into_iter()
            .map(|o| PyOps { inner: o })
            .collect()
    }

    /// Reverse the order of subpaths.
    ///
    /// :returns: A new Ops with subpath order reversed.
    fn flip_ops(&self) -> PyOps {
        PyOps {
            inner: self.inner.flip_ops(),
        }
    }

    /// Return a copy with all state commands removed.
    ///
    /// :returns: A new Ops containing only moving commands.
    fn without_state(&self) -> PyOps {
        PyOps {
            inner: self.inner.without_state(),
        }
    }

    /// Return the accumulated state at a given command index.
    ///
    /// :param idx: The command index.
    /// :returns: The state at that point.
    /// :raises IndexError: If the index is out of range.
    fn state_at(&self, idx: usize) -> PyResult<PyState> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        Ok(PyState(self.inner.state_at(idx)))
    }

    /// Extract a subset of commands by index.
    ///
    /// :param indices: List of command indices to extract.
    /// :returns: A new Ops sequence containing only the specified commands.
    fn sub_ops(&self, indices: Vec<usize>) -> PyOps {
        PyOps {
            inner: self.inner.sub_ops(&indices),
        }
    }

    /// Replace all commands in this sequence with those from another.
    ///
    /// :param source: The source Ops sequence.
    fn replace_all(&mut self, source: &PyOps) {
        self.inner.replace_all(&source.inner);
    }

    /// Replace the internal buffer of this sequence with a copy from another.
    ///
    /// :param source: The source Ops sequence.
    fn replace_with(&mut self, source: &PyOps) {
        self.inner.replace_with(&source.inner);
    }

    /// Create an Ops sequence from a Geometry.
    ///
    /// :param geometry: The geometry to convert.
    #[classmethod]
    fn from_geometry(
        _cls: &Bound<'_, PyType>,
        geometry: &PyGeometry,
    ) -> PyResult<Self> {
        Ok(PyOps {
            inner: crate::ops::Ops::from_geometry(&geometry.inner).map_err(
                |e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        e.to_string(),
                    )
                },
            )?,
        })
    }

    /// Convert this Ops sequence back into a Geometry.
    ///
    /// :returns: A Geometry representing the same paths.
    fn to_geometry(&self) -> PyGeometry {
        PyGeometry {
            inner: self.inner.to_geometry(),
        }
    }

    /// Pre-compute and store the accumulated state at each moving command.
    fn preload_state(&mut self) {
        self.inner.preload_state();
    }

    /// Apply a state to all moving commands without an explicit state.
    ///
    /// :param state: The state to apply.
    fn set_state_on_moving(&mut self, state: &PyState) {
        self.inner.set_state_on_moving(&state.0);
    }

    /// Overwrite the state at a specific command index.
    ///
    /// :param idx: The command index.
    /// :param state: The new state.
    fn set_state_at(&mut self, idx: usize, state: &PyState) {
        self.inner.set_state_at(idx, &state.0);
    }

    /// Print a human-readable dump of all commands.
    fn dump(&self, py: Python<'_>) -> PyResult<()> {
        let output = self.inner.format_dump();
        let print_fn = py.import("builtins")?.getattr("print")?;
        for line in output.lines() {
            print_fn.call1((line,))?;
        }
        Ok(())
    }

    /// Return detailed information about a single command.
    ///
    /// :param idx: The command index.
    /// :returns: A CommandInfo object with type, endpoint, state, axes, etc.
    fn inspect(&self, py: Python<'_>, idx: usize) -> PyResult<PyCommandInfo> {
        let inner = &self.inner;
        let ct = inner.commands[idx].command_type();

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

        if inner.commands[idx].is_moving() {
            info.end = Some(inner.commands[idx].end_point());
            if let Some(ea) = inner.commands[idx].extra_axes() {
                let dict = PyDict::new(py);
                for &(axis, val) in ea {
                    let py_axis = Py::new(py, PyAxis(axis))?;
                    dict.set_item(py_axis, val)?;
                }
                info.extra_axes = Some(dict.unbind());
            }
            if let Some(s) = inner.commands[idx].state() {
                info.state = Some(Py::new(py, PyState(s.clone()))?);
            }
        }

        match &inner.commands[idx].category {
            OpCategory::Moving { cmd, .. } => match cmd {
                MoveCmd::ArcTo { center, cw } => {
                    info.center_offset = Some(*center);
                    info.clockwise = Some(*cw);
                }
                MoveCmd::BezierTo { control1, control2 } => {
                    info.control1 = Some(*control1);
                    info.control2 = Some(*control2);
                }
                MoveCmd::QuadraticBezierTo { control } => {
                    info.control = Some(*control);
                }
                MoveCmd::ScanLine { power_values } => {
                    info.power_values =
                        Some(PyBytes::new(py, power_values.as_ref()).unbind());
                }
                _ => {}
            },
            OpCategory::State(cmd) => match cmd {
                StateCmd::SetPower(p) => info.power = Some(*p),
                StateCmd::SetCutSpeed(s) | StateCmd::SetTravelSpeed(s) => {
                    info.speed = Some(*s)
                }
                StateCmd::SetFrequency(f) => info.frequency = Some(*f),
                StateCmd::SetPulseWidth(pw) => info.pulse_width = Some(*pw),
                StateCmd::SetLaser(uid) => {
                    info.laser_uid = Some(uid.to_string())
                }
                StateCmd::Dwell(d) => info.duration_ms = Some(*d),
                _ => {}
            },
            OpCategory::Marker(cmd) => match cmd {
                MarkerCmd::LayerStart(uid) | MarkerCmd::LayerEnd(uid) => {
                    info.layer_uid = Some(uid.to_string());
                }
                MarkerCmd::WorkpieceStart(uid)
                | MarkerCmd::WorkpieceEnd(uid) => {
                    info.workpiece_uid = Some(uid.to_string());
                }
                MarkerCmd::OpsSectionStart {
                    section_type,
                    workpiece_uid,
                } => {
                    info.section_type = Some(format!("{:?}", section_type));
                    if let Some(wp) = workpiece_uid {
                        info.workpiece_uid = Some(wp.to_string());
                    }
                }
                MarkerCmd::OpsSectionEnd { section_type, .. } => {
                    info.section_type = Some(format!("{:?}", section_type));
                }
                _ => {}
            },
        }

        Ok(info)
    }

    // --- Geometry transforms ---

    /// Translate all moving commands by the given offset.
    ///
    /// :param dx: X offset.
    /// :param dy: Y offset.
    /// :param dz: Z offset (default 0.0).
    #[pyo3(signature = (dx, dy, dz = 0.0))]
    fn translate(&mut self, dx: f64, dy: f64, dz: f64) -> PyResult<()> {
        self.inner.translate(dx, dy, dz);
        Ok(())
    }

    /// Scale all coordinates by the given factors.
    ///
    /// :param sx: X scale factor.
    /// :param sy: Y scale factor.
    /// :param sz: Z scale factor (default 1.0).
    #[pyo3(signature = (sx, sy, sz = 1.0))]
    fn scale(&mut self, sx: f64, sy: f64, sz: f64) -> PyResult<()> {
        self.inner.scale(sx, sy, sz);
        Ok(())
    }

    /// Rotate all coordinates around a pivot point.
    ///
    /// :param angle_deg: Rotation angle in degrees.
    /// :param cx: Pivot X coordinate.
    /// :param cy: Pivot Y coordinate.
    fn rotate(&mut self, angle_deg: f64, cx: f64, cy: f64) -> PyResult<()> {
        self.inner.rotate(angle_deg, cx, cy);
        Ok(())
    }

    #[gen_stub(skip)]
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

    /// Clip this sequence to a rectangle, keeping only commands inside.
    ///
    /// :param rect: ``(x_min, y_min, x_max, y_max)``.
    /// :returns: A new Ops sequence containing the clipped commands.
    fn clip_rect(&self, rect: (f64, f64, f64, f64)) -> PyOps {
        PyOps {
            inner: self.inner.clip_rect(Rect(rect.0, rect.1, rect.2, rect.3)),
        }
    }

    /// Subtract polygonal regions from the cutting paths.
    ///
    /// :param regions: List of polygons, each being a list of ``(x, y)`` vertices.
    fn subtract_regions(
        &mut self,
        regions: Vec<Vec<(f64, f64)>>,
    ) -> PyResult<()> {
        self.inner.subtract_regions(&regions);
        Ok(())
    }

    /// Clip paths to the given polygonal regions, keeping only what is inside.
    ///
    /// :param regions: List of polygons, each being a list of ``(x, y)`` vertices.
    /// :param tolerance: Approximation tolerance (default 0.3).
    #[pyo3(signature = (regions, tolerance = 0.3))]
    fn clip_to_regions(
        &mut self,
        regions: Vec<Vec<(f64, f64)>>,
        tolerance: f64,
    ) -> PyResult<()> {
        self.inner.clip_to_regions(&regions, tolerance);
        Ok(())
    }

    /// Clip paths using polygonal regions as boundaries; keeps what is inside.
    ///
    /// :param regions: List of polygons, each being a list of ``(x, y)`` vertices.
    /// :param tolerance: Approximation tolerance (default 0.3).
    #[pyo3(signature = (regions, tolerance = 0.3))]
    fn clip_ops_to_regions(
        &mut self,
        regions: Vec<Vec<(f64, f64)>>,
        tolerance: f64,
    ) -> PyResult<()> {
        self.inner.clip_ops_to_regions(&regions, tolerance);
        Ok(())
    }

    /// Clip at a single vertical swath, keeping commands that intersect the band.
    ///
    /// :param x: X coordinate of the left edge.
    /// :param y: Y coordinate (used to find the relevant segment).
    /// :param width: Width of the band.
    /// :returns: True if any commands were kept.
    fn clip_at(&mut self, x: f64, y: f64, width: f64) -> bool {
        self.inner.clip_at(x, y, width)
    }

    /// Translate each layer by its own offset, with a default fallback.
    ///
    /// :param default_offset: The ``(x, y, z)`` offset for layers not listed in layer_offsets.
    /// :param layer_offsets: Optional dict mapping layer UIDs to ``(x, y, z)`` offsets.
    #[pyo3(signature = (default_offset, layer_offsets = None))]
    #[allow(clippy::type_complexity)]
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
        self.inner
            .translate_layers(default_offset, layer_offsets_rust.as_deref());
        Ok(())
    }

    /// Transform each layer by calling a Python callback with the layer UID and ops.
    ///
    /// The callback receives ``(layer_uid: str, layer_ops: Ops)`` and should
    /// mutate the layer_ops in place.
    ///
    /// :param callback: A callable accepting ``(layer_uid, layer_ops)``.
    fn transform_layers(
        &mut self,
        py: Python<'_>,
        callback: Py<PyAny>,
    ) -> PyResult<()> {
        let mut i = 0;
        while i < self.inner.len() {
            let layer_uid =
                if let OpCategory::Marker(MarkerCmd::LayerStart(uid)) =
                    &self.inner.commands[i].category
                {
                    uid.to_string()
                } else {
                    i += 1;
                    continue;
                };

            let layer_start = i;
            let mut collected_indices: Vec<usize> = Vec::new();
            while i < self.inner.len() {
                collected_indices.push(i);
                i += 1;
                if matches!(
                    self.inner.commands[i - 1].category,
                    OpCategory::Marker(MarkerCmd::LayerEnd(_))
                ) {
                    break;
                }
            }
            let layer_end = i;

            let layer_ops = self.inner.sub_ops(&collected_indices);
            let py_layer_ops = Py::new(py, PyOps { inner: layer_ops })?;
            callback.call1(py, (layer_uid, &py_layer_ops))?;
            let layer_ops_ref = py_layer_ops.borrow(py);
            let layer_ops = &layer_ops_ref.inner;

            let mut new_cmds = Vec::new();
            for j in 0..layer_start {
                new_cmds.push(self.inner.commands[j].clone());
            }
            for j in 0..layer_ops.len() {
                new_cmds.push(layer_ops.commands[j].clone());
            }
            for j in layer_end..self.inner.len() {
                new_cmds.push(self.inner.commands[j].clone());
            }
            self.inner.commands = new_cmds;
            i = layer_start + layer_ops.len();
        }
        self.inner.invalidate_time_cache();
        Ok(())
    }

    /// Transform moving commands by calling Python callbacks on each endpoint and aux point.
    ///
    /// The ``on_endpoint`` callback receives ``(endpoint, extra_axes)`` and
    /// should mutate the endpoint list in-place. The optional ``on_aux_point``
    /// callback receives control points for curve commands.
    ///
    /// :param on_endpoint: Callable ``(endpoint, extra_axes) -> None``.
    /// :param on_aux_point: Optional callable ``(point,) -> None`` for curve control points.
    #[pyo3(signature = (on_endpoint, on_aux_point = None))]
    fn transform_moving(
        &mut self,
        py: Python<'_>,
        on_endpoint: Py<PyAny>,
        on_aux_point: Option<Py<PyAny>>,
    ) -> PyResult<()> {
        for i in 0..self.inner.len() {
            if !self.inner.commands[i].is_moving() {
                continue;
            }

            let end = self.inner.commands[i].end_point();
            let end_list = vec![end.0, end.1, end.2];
            let end_py_list = PyList::new(py, &end_list)?;

            let ea = self.inner.commands[i].extra_axes();
            let ea_arg = if let Some(axes) = ea {
                axis_map_to_py(py, axes)?
            } else {
                PyDict::new(py)
            };

            on_endpoint.call1(py, (&end_py_list, &ea_arg))?;

            let new_end: Vec<f64> = end_py_list.extract()?;
            if let OpCategory::Moving { end: ref mut e, .. } =
                &mut self.inner.commands[i].category
            {
                *e = (new_end[0], new_end[1], new_end[2]);
            }

            if !ea_arg.is_empty() {
                let ea_vec = py_to_axis_map(&ea_arg)?;
                self.inner.commands[i]
                    .set_extra_axes(std::sync::Arc::from(ea_vec));
            }

            if let Some(ref aux_cb) = on_aux_point {
                if let OpCategory::Moving { cmd, .. } =
                    &mut self.inner.commands[i].category
                {
                    match cmd {
                        MoveCmd::ArcTo { center, .. } => {
                            let off_list = vec![center.0, center.1];
                            let off_py_list = PyList::new(py, &off_list)?;
                            aux_cb.call1(py, (&off_py_list,))?;
                            let new_off: Vec<f64> = off_py_list.extract()?;
                            *center = (new_off[0], new_off[1]);
                        }
                        MoveCmd::BezierTo {
                            control1, control2, ..
                        } => {
                            for cp in [control1, control2].iter_mut() {
                                let cp_list = vec![cp.0, cp.1, cp.2];
                                let cp_py_list = PyList::new(py, &cp_list)?;
                                aux_cb.call1(py, (&cp_py_list,))?;
                                let new_cp: Vec<f64> = cp_py_list.extract()?;
                                **cp = (new_cp[0], new_cp[1], new_cp[2]);
                            }
                        }
                        MoveCmd::QuadraticBezierTo { control, .. } => {
                            let cp_list = vec![control.0, control.1, control.2];
                            let cp_py_list = PyList::new(py, &cp_list)?;
                            aux_cb.call1(py, (&cp_py_list,))?;
                            let new_cp: Vec<f64> = cp_py_list.extract()?;
                            *control = (new_cp[0], new_cp[1], new_cp[2]);
                        }
                        _ => {}
                    }
                }
            }
        }
        self.inner.invalidate_time_cache();
        Ok(())
    }

    /// Decompose a curved command into linear segments.
    ///
    /// :param idx: Index of the command to linearize.
    /// :param start_point: The ``(x, y, z)`` start point of the curve.
    /// :returns: A new Ops containing the linearized sub-commands.
    /// :raises TypeError: If the command at idx is not a curve or line type.
    fn linearize(
        &self,
        idx: usize,
        start_point: (f64, f64, f64),
    ) -> PyResult<Self> {
        let ct = self.inner.commands[idx].command_type();
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

    /// Replace all curved commands with linear approximations in-place.
    fn linearize_all(&mut self) {
        self.inner.linearize_all();
    }

    /// Replace only bezier and quadratic bezier curves with linear approximations.
    fn linearize_curves(&mut self) {
        self.inner.linearize_curves();
    }

    /// Replace only arc commands with linear approximations.
    fn linearize_arcs(&mut self) {
        self.inner.linearize_arcs();
    }

    /// Return index ranges for each contiguous cutting segment.
    ///
    /// :returns: A list of index lists, one per segment.
    fn segment_indices(&self) -> Vec<Vec<usize>> {
        self.inner.segment_indices()
    }

    /// Group contiguous commands with the same state into separate Ops sequences.
    ///
    /// :returns: A list of Ops sequences grouped by state continuity.
    fn group_by_state_continuity(&self) -> Vec<PyOps> {
        self.inner
            .group_by_state_continuity()
            .into_iter()
            .map(|o| PyOps { inner: o })
            .collect()
    }

    /// Return the logical sections of the ops.
    ///
    /// Sections are delimited by ``OpsSectionStart``/``OpsSectionEnd``
    /// markers and group commands into vector-outline and raster-fill
    /// portions.
    ///
    /// :returns: List of OpsSection objects.
    fn sections(&self) -> Vec<PyOpsSection> {
        self.inner
            .iter_sections()
            .into_iter()
            .map(PyOpsSection)
            .collect()
    }

    /// Return the section ranges of the ops as index ranges.
    ///
    /// Similar to :meth:`sections` but returns contiguous
    /// index ranges instead of individual index lists.
    ///
    /// :returns: List of OpsSectionRange objects.
    fn section_ranges(&self) -> Vec<PyOpsSectionRange> {
        self.inner
            .iter_section_ranges()
            .into_iter()
            .map(PyOpsSectionRange)
            .collect()
    }

    /// Compute the bounding rectangle of all commands.
    ///
    /// :param include_travel: Whether to include travel moves (default False).
    /// :returns: ``(x_min, y_min, x_max, y_max)``.
    #[pyo3(signature = (include_travel = false))]
    fn rect(&self, include_travel: bool) -> (f64, f64, f64, f64) {
        match self.inner.rect(include_travel) {
            Some(r) => (r.0, r.1, r.2, r.3),
            None => (0.0, 0.0, 0.0, 0.0),
        }
    }

    /// Extract a frame (first and last endpoints) from the sequence.
    ///
    /// :param power: Optional power to set on the frame commands.
    /// :param speed: Optional speed to set on the frame commands.
    /// :returns: A new Ops containing only the frame endpoints.
    #[pyo3(signature = (power = None, speed = None))]
    fn get_frame(&self, power: Option<f64>, speed: Option<f64>) -> PyOps {
        PyOps {
            inner: self.inner.get_frame(power, speed),
        }
    }

    /// Estimate the total processing time for this sequence.
    ///
    /// :param default_cut_speed: Default cutting speed (default 1000.0).
    /// :param default_travel_speed: Default travel speed (default 3000.0).
    /// :param acceleration: Acceleration value (default 1000.0).
    /// :returns: Estimated time in seconds.
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

    /// Estimate the time of each individual command in the sequence.
    ///
    /// Returns a list with one entry per command. Moving commands
    /// (MoveTo, LineTo, ArcTo, etc.) yield their estimated execution
    /// time in seconds. Non-moving commands (state changes, markers)
    /// yield 0.0.
    ///
    /// :param default_cut_speed: Default cutting speed (default 1000.0).
    /// :param default_travel_speed: Default travel speed (default 3000.0).
    /// :param acceleration: Acceleration value (default 1000.0).
    /// :returns: List of estimated times in seconds, one per command.
    #[pyo3(signature = (default_cut_speed = 1000.0, default_travel_speed = 3000.0, acceleration = 1000.0))]
    fn estimate_command_times(
        &mut self,
        default_cut_speed: f64,
        default_travel_speed: f64,
        acceleration: f64,
    ) -> Vec<f64> {
        self.inner.estimate_command_times(
            default_cut_speed,
            default_travel_speed,
            acceleration,
        )
    }

    // --- Properties ---

    /// The last ``(x, y, z)`` endpoint from a MoveTo command.
    #[getter]
    fn get_last_move_to(&self) -> (f64, f64, f64) {
        self.inner.last_move_to
    }

    #[setter]
    fn set_last_move_to(&mut self, val: (f64, f64, f64)) {
        self.inner.last_move_to = val;
    }

    /// Serialize this Ops sequence to a dict suitable for JSON export.
    ///
    /// :returns: A Python dict representation.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        super::serialize::ops_to_dict(py, &self.inner)
    }

    /// Create an Ops sequence from a dictionary.
    ///
    /// :param data: Dictionary as produced by to_dict.
    #[classmethod]
    fn from_dict(
        _cls: &Bound<'_, PyType>,
        data: &Bound<'_, PyDict>,
    ) -> PyResult<Self> {
        let inner = super::serialize::ops_from_dict(data)?;
        Ok(PyOps { inner })
    }

    /// Serialize this Ops sequence to numpy arrays.
    ///
    /// :returns: A Python dict of numpy arrays.
    fn to_numpy_arrays(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        super::serialize::ops_to_numpy_arrays(py, &self.inner)
    }

    /// Create an Ops sequence from numpy arrays.
    ///
    /// :param arrays: Dictionary as produced by to_numpy_arrays.
    #[classmethod]
    fn from_numpy_arrays(
        _cls: &Bound<'_, PyType>,
        arrays: &Bound<'_, PyDict>,
    ) -> PyResult<Self> {
        let inner = super::serialize::ops_from_numpy_arrays(arrays)?;
        Ok(PyOps { inner })
    }

    #[gen_stub(skip)]
    #[getter]
    #[allow(non_snake_case)]
    fn get__time_dirty(&self) -> bool {
        self.inner.time_dirty
    }

    #[gen_stub(skip)]
    #[getter]
    #[allow(non_snake_case)]
    fn get__cached_time(&self) -> f64 {
        self.inner.cached_time
    }

    #[gen_stub(skip)]
    #[getter]
    #[allow(non_snake_case)]
    fn get__time_params(&self) -> Option<(f64, f64, f64)> {
        self.inner.time_params
    }

    /// Apply holding tabs as gaps in the toolpath.
    ///
    /// For each clip point, the closest subpath is found and a gap of
    /// the specified width is cut at the nearest point on the path.
    /// Only ``VECTOR_OUTLINE`` sections are modified.
    ///
    /// :param clips: List of ``(x, y, width)`` tuples defining tab positions.
    fn apply_tab_gaps(&mut self, clips: Vec<(f64, f64, f64)>) {
        let clip_points: Vec<crate::ops::tabs::ClipPoint> = clips
            .into_iter()
            .map(|(x, y, width)| crate::ops::tabs::ClipPoint { x, y, width })
            .collect();
        crate::ops::tabs::apply_tab_gaps(&mut self.inner, &clip_points);
    }

    /// Apply holding tabs by reducing laser power in tab regions.
    ///
    /// Instead of cutting a gap, the laser power is lowered in the tab
    /// area so the material stays connected but weaker. Only
    /// ``VECTOR_OUTLINE`` sections are modified.
    ///
    /// :param clips: List of ``(x, y, width)`` tuples defining tab positions.
    /// :param tab_power: Power level inside tab regions (0.0–1.0).
    /// :param original_power: Normal cutting power to restore after the tab.
    fn apply_tab_power(
        &mut self,
        clips: Vec<(f64, f64, f64)>,
        tab_power: f64,
        original_power: f64,
    ) {
        let clip_points: Vec<crate::ops::tabs::ClipPoint> = clips
            .into_iter()
            .map(|(x, y, width)| crate::ops::tabs::ClipPoint { x, y, width })
            .collect();
        crate::ops::tabs::apply_tab_power(
            &mut self.inner,
            &clip_points,
            tab_power,
            original_power,
        );
    }

    /// Merge overlapping line segments across all paths.
    ///
    /// Detects line segments that are collinear and overlapping and
    /// replaces the covered sub-segments with travel moves to avoid
    /// cutting the same line twice.
    ///
    /// :param tolerance: Maximum distance for considering lines collinear.
    fn merge_overlapping_lines(&mut self, tolerance: f64) {
        crate::ops::merge_lines::merge_overlapping_lines(
            &mut self.inner,
            tolerance,
        );
    }

    /// Apply overscan to raster lines.
    ///
    /// Extends raster line start/end points by ``distance_mm`` along
    /// the line direction, adding zero-power lead-in and lead-out
    /// segments for constant engraving velocity.
    ///
    /// :param distance_mm: Overscan distance in millimeters.
    fn apply_overscan(&mut self, distance_mm: f64) {
        crate::ops::overscan::apply_overscan(&mut self.inner, distance_mm);
    }

    /// Apply lead-in and lead-out to vector contour paths.
    ///
    /// For each contour within a VECTOR_OUTLINE section, extends the
    /// toolpath with zero-power lead-in and lead-out segments along
    /// the tangent direction at the path start and end.
    ///
    /// :param lead_in_mm: Lead-in distance in millimeters.
    /// :param lead_out_mm: Lead-out distance in millimeters.
    fn apply_lead_in_out(&mut self, lead_in_mm: f64, lead_out_mm: f64) {
        crate::ops::lead_in_out::apply_lead_in_out(
            &mut self.inner,
            lead_in_mm,
            lead_out_mm,
        );
    }

    /// Optimize travel distance by reordering segments.
    ///
    /// Performs two-level optimization: workpiece-level reordering
    /// (when workpiece markers are present) and segment-level
    /// nearest-neighbor + 2-opt refinement.
    ///
    /// :param allow_flip: Whether to allow flipping subpaths.
    /// :param preserve_first: Keep the first workpiece in place.
    /// :param preserve_order: Workpiece UIDs whose order to preserve.
    /// :param progress_cb: Optional callable(progress, message).
    #[pyo3(signature = (allow_flip=true, preserve_first=false, preserve_order=Vec::new(), progress_cb=None))]
    fn optimize_travel(
        &mut self,
        allow_flip: bool,
        preserve_first: bool,
        preserve_order: Vec<String>,
        progress_cb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        use crate::ops::optimize::ProgressCallback;

        struct PyProgress<'py> {
            cb: Option<&'py Bound<'py, PyAny>>,
        }

        impl<'py> ProgressCallback for PyProgress<'py> {
            fn report(&self, progress: f64, message: &str) {
                if let Some(cb) = self.cb {
                    let _ = cb.call1((progress, message));
                }
            }

            fn is_cancelled(&self) -> bool {
                if let Some(cb) = self.cb {
                    if let Ok(result) = cb.call_method0("is_cancelled") {
                        if let Ok(cancelled) = result.extract::<bool>() {
                            return cancelled;
                        }
                    }
                }
                false
            }
        }

        let py_progress = PyProgress { cb: progress_cb };
        crate::ops::optimize::optimize_travel(
            &mut self.inner,
            allow_flip,
            preserve_first,
            preserve_order,
            Some(&py_progress),
        );
        Ok(())
    }
}
