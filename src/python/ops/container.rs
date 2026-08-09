use glam::{DMat4, DVec4};
use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyType};
use pyo3::{Bound, Py, PyAny, PyResult};
use pyo3_stub_gen::derive::{
    gen_methods_from_python, gen_stub_pyclass, gen_stub_pymethods,
};
use pyo3_stub_gen::inventory::submit;

use crate::geo::types::{Point, Point3D, Rect};
use crate::ops::{
    Axis, CommandType, MarkerCmd, MoveCmd, OpCategory, OpsSection,
    OpsSectionRange, StateCmd,
};
use crate::python::geo::flex_point::{
    point3d_to_tuple, polygons_from_tuples, tuple_to_point3d,
};
use crate::python::ops::transform as py_transform;
use py_transform::{extract_transformer, PyCallableCallbacks};

use super::axis::PyAxis;
use super::state::{
    PyAirAssistMode, PyCoolantMode, PyHeadCoolantMode, PyState,
};
use super::types::{
    PyCommandCategory, PyCommandType, PyRasterMode, PySectionType, PyStateBlock,
};
use crate::python::geo::geometry::Geometry as PyGeometry;
use crate::python::geo::matrix::Matrix as PyMatrix;
use crate::python::image::scan::PyScanMode;

/// Convert a Python dict to a JSON string, then deserialise into `T`.
fn from_pydict<T: serde::de::DeserializeOwned>(
    py: Python<'_>,
    dict: &Bound<'_, PyDict>,
) -> PyResult<T> {
    let json_mod = py.import("json")?;
    let json_str: String = json_mod
        .call_method1("dumps", (dict,))?
        .extract()
        .map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(
            "Failed to serialise dict to JSON",
        )
    })?;
    serde_json::from_str(&json_str).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "Failed to deserialise JSON into Rust struct: {e}"
        ))
    })
}

// ── Typed PyO3 objects replacing the dict handoff ──────────────

/// One rendering group (flat or rotary) with all vertex & overlay buffers.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops", name = "VertexGroup", skip_from_py_object)]
pub struct PyVertexGroup {
    #[pyo3(get)]
    pub is_rotary: bool,
    #[pyo3(get)]
    pub powered_verts: Py<PyArray1<f32>>,
    #[pyo3(get)]
    pub powered_attrib: Py<PyArray1<f32>>,
    #[pyo3(get)]
    pub travel_verts: Py<PyArray1<f32>>,
    #[pyo3(get)]
    pub zero_power_verts: Py<PyArray1<f32>>,
    #[pyo3(get)]
    pub powered_cmd_offsets: Py<PyArray1<i32>>,
    #[pyo3(get)]
    pub travel_cmd_offsets: Py<PyArray1<i32>>,
    #[pyo3(get)]
    pub overlay_positions: Py<PyArray1<f32>>,
    #[pyo3(get)]
    pub overlay_attrib: Py<PyArray1<f32>>,
    #[pyo3(get)]
    pub overlay_cmd_offsets: Py<PyArray1<i32>>,
}

/// One layer's metadata from compilation.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops", name = "LayerInfo", skip_from_py_object)]
pub struct PyLayerInfo {
    #[pyo3(get)]
    pub cmd_start: usize,
    #[pyo3(get)]
    pub cmd_end: usize,
    #[pyo3(get)]
    pub is_rotary: bool,
    #[pyo3(get)]
    pub diameter: f64,
    #[pyo3(get)]
    pub has_scanlines: bool,
    #[pyo3(get)]
    pub scanline_laser: String,
    #[pyo3(get)]
    pub activation_cmd_idx: usize,
    #[pyo3(get)]
    pub axis_position: f64,
    #[pyo3(get)]
    pub reverse: bool,
}

/// Top-level output of :meth:`Ops.compile_scene_3d`.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.ops", name = "CompiledScene3D", skip_from_py_object)]
pub struct PyCompiledScene3D {
    #[pyo3(get)]
    pub groups: Vec<Py<PyVertexGroup>>,
    #[pyo3(get)]
    pub layer_infos: Vec<Py<PyLayerInfo>>,
    #[pyo3(get)]
    pub laser_uid_order: Vec<String>,
}

/// Build a :class:`CompiledScene3D` from the Rust data, converting
/// plain `Vec<f32>`/`Vec<i32>` buffers into zero-copy numpy arrays.
fn py_scene_data_to_object<'py>(
    py: Python<'py>,
    data: crate::ops::convert::scene::CompiledSceneData,
) -> PyResult<Py<PyCompiledScene3D>> {
    use numpy::IntoPyArray;

    let groups: Vec<Py<PyVertexGroup>> = data
        .groups
        .into_iter()
        .map(|g| {
            Py::new(
                py,
                PyVertexGroup {
                    is_rotary: g.is_rotary,
                    powered_verts: g.powered_verts.into_pyarray(py).unbind(),
                    powered_attrib: g.powered_attrib.into_pyarray(py).unbind(),
                    travel_verts: g.travel_verts.into_pyarray(py).unbind(),
                    zero_power_verts: g
                        .zero_power_verts
                        .into_pyarray(py)
                        .unbind(),
                    powered_cmd_offsets: g
                        .powered_cmd_offsets
                        .into_pyarray(py)
                        .unbind(),
                    travel_cmd_offsets: g
                        .travel_cmd_offsets
                        .into_pyarray(py)
                        .unbind(),
                    overlay_positions: g
                        .overlay_positions
                        .into_pyarray(py)
                        .unbind(),
                    overlay_attrib: g.overlay_attrib.into_pyarray(py).unbind(),
                    overlay_cmd_offsets: g
                        .overlay_cmd_offsets
                        .into_pyarray(py)
                        .unbind(),
                },
            )
        })
        .collect::<PyResult<Vec<_>>>()?;

    let layer_infos: Vec<Py<PyLayerInfo>> = data
        .layer_infos
        .into_iter()
        .map(|li| {
            Py::new(
                py,
                PyLayerInfo {
                    cmd_start: li.cmd_start,
                    cmd_end: li.cmd_end,
                    is_rotary: li.is_rotary,
                    diameter: li.diameter,
                    has_scanlines: li.has_scanlines,
                    scanline_laser: li.scanline_laser,
                    activation_cmd_idx: li.activation_cmd_idx,
                    axis_position: li.axis_position,
                    reverse: li.reverse,
                },
            )
        })
        .collect::<PyResult<Vec<_>>>()?;

    Py::new(
        py,
        PyCompiledScene3D {
            groups,
            layer_infos,
            laser_uid_order: data.laser_uid_order,
        },
    )
}

/// Convert a Rust `usize`-keyed `HashMap` into a Python dict with
/// integer keys.
fn usize_hashmap_to_pydict<'py>(
    py: Python<'py>,
    map: &std::collections::HashMap<usize, Vec<usize>>,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    for (&k, v) in map {
        let py_list = PyList::empty(py);
        for &item in v {
            py_list.append(item)?;
        }
        dict.set_item(k, py_list)?;
    }
    Ok(dict)
}

/// Convert a Rust `usize → usize` `HashMap` into a Python dict.
fn usize_usize_hashmap_to_pydict<'py>(
    py: Python<'py>,
    map: &std::collections::HashMap<usize, usize>,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    for (&k, &v) in map {
        dict.set_item(k, v)?;
    }
    Ok(dict)
}

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

/// Thin wrapper around :func:`convert::dict::py_to_axis_map_helper`.
fn py_to_axis_map(dict: &Bound<'_, PyDict>) -> PyResult<Vec<(Axis, f64)>> {
    super::convert::dict::py_to_axis_map_helper(dict)
}

/// Thin wrapper around :func:`convert::dict::axis_map_to_py_helper`.
fn axis_map_to_py<'a>(
    py: Python<'a>,
    axes: &[(Axis, f64)],
) -> PyResult<Bound<'a, PyDict>> {
    super::convert::dict::axis_map_to_py_helper(py, axes)
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

    /// The raster mode of this section, if any.
    #[getter]
    fn raster_mode(&self) -> Option<PyRasterMode> {
        self.0.raster_mode.map(PyRasterMode)
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

    /// Extract the content commands of this section from an Ops sequence.
    ///
    /// :param ops: The Ops sequence containing this section.
    /// :returns: A new Ops containing only the content of this section.
    /// :complexity: O(n) time, O(n) space
    fn content(&self, ops: &PyOps) -> PyOps {
        PyOps {
            inner: ops.inner.section_ops(&self.0),
        }
    }

    /// Return the state blocks within this section.
    ///
    /// :param ops: The parent Ops sequence.
    /// :returns: List of StateBlock objects.
    /// :raises RuntimeError: If state block nesting is invalid.
    /// :complexity: O(n) time, O(n) space
    fn state_blocks(&self, ops: &PyOps) -> PyResult<Vec<PyStateBlock>> {
        ops.inner
            .state_blocks(&self.0)
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
            })
            .map(|blocks| {
                blocks
                    .into_iter()
                    .map(|b| PyStateBlock {
                        name: b.name.as_ref().map(|s| s.to_string()),
                        marker_indices: b.marker_indices,
                        content_indices: b.content_indices,
                    })
                    .collect()
            })
    }

    /// Extract a specific state block's content as Ops.
    ///
    /// :param ops: The parent Ops sequence.
    /// :param block: The StateBlock to extract.
    /// :returns: A new Ops containing only the block's content.
    /// :complexity: O(n) time, O(n) space
    fn state_block_content(&self, ops: &PyOps, block: &PyStateBlock) -> PyOps {
        PyOps {
            inner: ops.inner.state_block_content_from_indices(
                &block.marker_indices,
                &block.content_indices,
            ),
        }
    }

    /// Find state blocks by name pattern (``*`` prefix match or exact).
    ///
    /// :param ops: The parent Ops sequence.
    /// :param pattern: Name pattern (``"cell-*"`` for prefix, ``"labels"`` for exact).
    /// :returns: List of matching StateBlock objects.
    /// :raises RuntimeError: If state block nesting is invalid.
    /// :complexity: O(n) time, O(n) space
    fn state_blocks_by_name(
        &self,
        ops: &PyOps,
        pattern: &str,
    ) -> PyResult<Vec<PyStateBlock>> {
        ops.inner
            .state_blocks_by_name(&self.0, pattern)
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
            })
            .map(|blocks| {
                blocks
                    .into_iter()
                    .map(|b| PyStateBlock {
                        name: b.name.as_ref().map(|s| s.to_string()),
                        marker_indices: b.marker_indices,
                        content_indices: b.content_indices,
                    })
                    .collect()
            })
    }

    fn __repr__(&self) -> String {
        format!(
            "OpsSection(section_type={:?}, raster_mode={:?}, marker_indices={:?}, content_indices={:?})",
            self.0.section_type, self.0.raster_mode, self.0.marker_indices, self.0.content_indices
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

    /// The raster mode of this section range, if any.
    #[getter]
    fn raster_mode(&self) -> Option<PyRasterMode> {
        self.0.raster_mode.map(PyRasterMode)
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

    /// Extract the content commands of this section range from an Ops sequence.
    ///
    /// :param ops: The Ops sequence containing this section.
    /// :returns: A new Ops containing only the content of this section range.
    /// :complexity: O(n) time, O(n) space
    /// Extract the content commands of this section range from an Ops sequence.
    ///
    /// :param ops: The Ops sequence containing this section.
    /// :returns: A new Ops containing only the content of this section range.
    /// :complexity: O(n) time, O(n) space
    fn content(&self, ops: &PyOps) -> PyOps {
        PyOps {
            inner: ops.inner.section_range_ops(&self.0),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "OpsSectionRange(section_type={:?}, raster_mode={:?}, marker_indices={:?}, content_indices={:?})",
            self.0.section_type, self.0.raster_mode, self.0.marker_indices, self.0.content_indices
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
    /// The type of this command (e.g. Move, Line, Arc, Bezier, ScanTo, …).
    #[pyo3(get)]
    pub type_: PyCommandType,
    /// Endpoint of the command in 3D space, if applicable.
    #[pyo3(get)]
    pub end: Option<(f64, f64, f64)>,
    /// Extra axis positions, if any.
    #[pyo3(get)]
    pub extra_axes: Option<Py<PyDict>>,
    /// State snapshot at this command, if present.
    #[pyo3(get)]
    pub state: Option<Py<PyState>>,
    /// Arc centre offset from start point, if an arc command.
    #[pyo3(get)]
    pub center_offset: Option<(f64, f64)>,
    /// Whether an arc is clockwise, if an arc command.
    #[pyo3(get)]
    pub clockwise: Option<bool>,
    /// First cubic-Bezier control point, if a Bezier command.
    #[pyo3(get)]
    pub control1: Option<(f64, f64, f64)>,
    /// Second cubic-Bezier control point, if a Bezier command.
    #[pyo3(get)]
    pub control2: Option<(f64, f64, f64)>,
    /// Quadratic-Bezier control point, if a quad. Bezier command.
    #[pyo3(get)]
    pub control: Option<(f64, f64, f64)>,
    /// Per-step power byte values for scan-to commands.
    #[pyo3(get)]
    pub power_values: Option<Py<PyBytes>>,
    /// Power level (0–1), if a power-setting command.
    #[pyo3(get)]
    pub power: Option<f64>,
    /// Feed rate setting, if a SetFeedRate command.
    #[pyo3(get)]
    pub feed_rate: Option<i32>,
    /// Rapid rate setting, if a SetRapidRate command.
    #[pyo3(get)]
    pub rapid_rate: Option<i32>,
    /// Laser frequency (Hz), if a frequency-setting command.
    #[pyo3(get)]
    pub frequency: Option<i32>,
    /// Laser pulse width (µs), if a pulse-width-setting command.
    #[pyo3(get)]
    pub pulse_width: Option<f64>,
    /// Unique identifier of the active head, if a head-setting command.
    #[pyo3(get)]
    pub head_uid: Option<String>,
    /// Spindle RPM, if a SetSpindleRpm command.
    #[pyo3(get)]
    pub spindle_rpm: Option<u32>,
    /// Coolant mode, if a SetCoolant command.
    #[pyo3(get)]
    pub coolant: Option<PyCoolantMode>,
    /// Air assist mode, if a SetAirAssist command.
    #[pyo3(get)]
    pub air_assist: Option<PyAirAssistMode>,
    /// Head coolant mode, if a SetHeadCoolant command.
    #[pyo3(get)]
    pub head_coolant: Option<PyHeadCoolantMode>,
    /// Dwell duration in ms, if a dwell command.
    #[pyo3(get)]
    pub duration_ms: Option<f64>,
    /// Unique identifier of the active layer, if a layer-start command.
    #[pyo3(get)]
    pub layer_uid: Option<String>,
    /// Unique identifier of the active workpiece, if a workpiece-start command.
    #[pyo3(get)]
    pub workpiece_uid: Option<String>,
    /// Section type, if a section marker.
    #[pyo3(get)]
    pub section_type: Option<PySectionType>,
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
            if self.feed_rate != other_info.feed_rate {
                return Ok(false);
            }
            if self.rapid_rate != other_info.rapid_rate {
                return Ok(false);
            }
            if self.frequency != other_info.frequency {
                return Ok(false);
            }
            if self.pulse_width != other_info.pulse_width {
                return Ok(false);
            }
            if self.head_uid != other_info.head_uid {
                return Ok(false);
            }
            if self.spindle_rpm != other_info.spindle_rpm {
                return Ok(false);
            }
            if self.coolant != other_info.coolant {
                return Ok(false);
            }
            if self.air_assist != other_info.air_assist {
                return Ok(false);
            }
            if self.head_coolant != other_info.head_coolant {
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

/// A sequence of machining operations (commands).
///
/// ``Ops`` is a container of ordered commands that define a complete
/// machining job. It supports building command sequences
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
                :complexity: O(n) time, O(n) space
                """
                ...
        "#
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyOps {
    /// Create a new, empty Ops sequence.
    ///
    /// :complexity: O(1) time, O(1) space
    #[new]
    pub fn new() -> Self {
        PyOps {
            inner: crate::ops::Ops::new(),
        }
    }

    /// Return the number of commands.
    ///
    /// :complexity: O(1) time, O(1) space
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Concatenate two Ops sequences (``ops1 + ops2``).
    ///
    /// :complexity: O(n) time, O(n) space
    fn __add__(&self, other: &PyOps) -> PyOps {
        PyOps {
            inner: &self.inner + &other.inner,
        }
    }

    /// Repeat the ops sequence *count* times (``ops * n``).
    ///
    /// :complexity: O(n * k) time, O(n * k) space where k is the repeat count
    fn __mul__(&self, count: usize) -> PyOps {
        PyOps {
            inner: &self.inner * count,
        }
    }

    /// Check if the ops sequence is empty.
    ///
    /// :returns: ``True`` if the container is empty.
    /// :complexity: O(1) time, O(1) space
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Return the number of commands.
    ///
    /// :returns: Number of commands in the container.
    /// :complexity: O(1) time, O(1) space
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Estimated heap-allocated bytes for this Ops instance
    /// (commands Vec buffer + scanline power data).
    ///
    /// :returns: Estimated heap memory usage in bytes.
    /// :complexity: O(n) time, O(1) space
    fn heap_size(&self) -> usize {
        self.inner.heap_size()
    }

    /// Get the :class:`CommandType` at the given index.
    ///
    /// :param idx: Command index (negative = from end).
    /// :returns: The :class:`CommandType` of the command.
    /// :complexity: O(1) time, O(1) space
    fn command_type(&self, idx: isize) -> PyResult<PyCommandType> {
        let idx = normalize_index(idx, self.inner.len())?;
        Ok(PyCommandType(self.inner.commands[idx].command_type()))
    }

    /// Get the :class:`CommandCategory` at the given index.
    ///
    /// :param idx: Command index (negative = from end).
    /// :returns: The category (MOVING, STATE, or MARKER).
    /// :complexity: O(1) time, O(1) space
    fn category(&self, idx: isize) -> PyResult<PyCommandCategory> {
        let idx = normalize_index(idx, self.inner.len())?;
        Ok(PyCommandCategory(
            self.inner.commands[idx].command_type().category(),
        ))
    }

    /// Return the number of cutting commands in this sequence.
    ///
    /// Counts all LineTo, ArcTo, BezierTo, QuadraticBezierTo, and
    /// ScanLine commands.
    ///
    /// :returns: Number of cutting commands.
    /// :complexity: O(n) time, O(1) space
    fn count_cutting(&self) -> usize {
        self.inner.count_cutting()
    }

    /// Return the number of travel (MoveTo) commands in this sequence.
    ///
    /// :returns: Number of travel commands.
    /// :complexity: O(n) time, O(1) space
    fn count_travel(&self) -> usize {
        self.inner.count_travel()
    }

    /// Check whether the command at *idx* is a travel (non-cutting) move.
    ///
    /// :param idx: Command index.
    /// :returns: True if the command is a travel move.
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(n) time, O(n) space
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
    /// :complexity: O(1) time, O(1) space
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
                    let dx = end.x - lp.0;
                    let dy = end.y - lp.1;
                    (dx * dx + dy * dy).sqrt()
                }
            }
        } else {
            0.0
        }
    }

    /// Compute the total distance of all commands.
    ///
    /// :returns: Total path distance in mm.
    /// :complexity: O(n) time, O(1) space
    fn distance(&self) -> f64 {
        self.inner.distance()
    }

    /// Compute the total cutting distance (excluding travel moves).
    ///
    /// :returns: Total cut distance in mm.
    /// :complexity: O(n) time, O(1) space
    fn cut_distance(&self) -> f64 {
        self.inner.cut_distance()
    }

    /// Return the number of scanline commands in the sequence.
    ///
    /// :complexity: O(n) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
    fn endpoint(&self, idx: isize) -> PyResult<(f64, f64, f64)> {
        let idx = normalize_index(idx, self.inner.len())?;
        Ok(point3d_to_tuple(self.inner.commands[idx].end_point()))
    }

    /// Get the arc parameters (center offset i, j, and clockwise flag).
    ///
    /// :param idx: Command index.
    /// :returns: ``(i, j, clockwise)`` tuple.
    /// :raises TypeError: If the command is not an ArcTo.
    /// :complexity: O(1) time, O(1) space
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
            Ok((center.x, center.y, *cw))
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
    /// :complexity: O(1) time, O(1) space
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
            Ok((point3d_to_tuple(*control1), point3d_to_tuple(*control2)))
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
    /// :complexity: O(1) time, O(1) space
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
            Ok(point3d_to_tuple(*control))
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
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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

    /// Get the feed/rapid rate from a SetFeedRate or SetRapidRate command.
    ///
    /// :param idx: Command index.
    /// :returns: Rate in mm/min.
    /// :raises TypeError: If the command is not a rate command.
    /// :complexity: O(1) time, O(1) space
    fn rate(&self, idx: usize) -> PyResult<i32> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match &self.inner.commands[idx].category {
            OpCategory::State(StateCmd::SetFeedRate(s))
            | OpCategory::State(StateCmd::SetRapidRate(s)) => Ok(*s),
            _ => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a rate command",
            )),
        }
    }

    /// Get the frequency of a SetFrequency command.
    ///
    /// :param idx: Command index.
    /// :returns: Frequency in Hz.
    /// :raises TypeError: If the command is not a SetFrequency.
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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

    /// Get the spindle RPM from a SetSpindleRpm command.
    ///
    /// :param idx: Command index.
    /// :returns: Spindle RPM.
    /// :raises TypeError: If the command is not a SetSpindleRpm.
    /// :complexity: O(1) time, O(1) space
    fn spindle_rpm(&self, idx: usize) -> PyResult<u32> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::SetSpindleRpm(s)) =
            &self.inner.commands[idx].category
        {
            Ok(*s)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetSpindleRpmCommand",
            ))
        }
    }

    /// Get the coolant mode from a SetCoolant command.
    ///
    /// :param idx: Command index.
    /// :returns: The coolant mode.
    /// :raises TypeError: If the command is not a SetCoolant.
    /// :complexity: O(1) time, O(1) space
    fn coolant(&self, idx: usize) -> PyResult<PyCoolantMode> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::SetCoolant(mode)) =
            &self.inner.commands[idx].category
        {
            Ok(PyCoolantMode(*mode))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetCoolantCommand",
            ))
        }
    }

    /// Get the air assist mode from a SetAirAssist command.
    ///
    /// :param idx: Command index.
    /// :returns: The air assist mode.
    /// :raises TypeError: If the command is not a SetAirAssist.
    /// :complexity: O(1) time, O(1) space
    fn air_assist(&self, idx: usize) -> PyResult<PyAirAssistMode> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::SetAirAssist(mode)) =
            &self.inner.commands[idx].category
        {
            Ok(PyAirAssistMode(*mode))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetAirAssistCommand",
            ))
        }
    }

    /// Get the head coolant mode from a SetHeadCoolant command.
    ///
    /// :param idx: Command index.
    /// :returns: The head coolant mode.
    /// :raises TypeError: If the command is not a SetHeadCoolant.
    /// :complexity: O(1) time, O(1) space
    fn head_coolant(&self, idx: usize) -> PyResult<PyHeadCoolantMode> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::SetHeadCoolant(mode)) =
            &self.inner.commands[idx].category
        {
            Ok(PyHeadCoolantMode(*mode))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetHeadCoolantCommand",
            ))
        }
    }

    /// Get the head UID from a SetHead command.
    ///
    /// :param idx: Command index.
    /// :returns: The head identifier.
    /// :raises TypeError: If the command is not a SetHead.
    /// :complexity: O(1) time, O(1) space
    fn head_uid(&self, idx: usize) -> PyResult<String> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        if let OpCategory::State(StateCmd::SetHead(uid)) =
            &self.inner.commands[idx].category
        {
            Ok(uid.to_string())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not a SetHeadCommand",
            ))
        }
    }

    /// Get the layer UID from a LayerStart or LayerEnd command.
    ///
    /// :param idx: Command index.
    /// :returns: The layer identifier.
    /// :raises TypeError: If the command is not a Layer command.
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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

    /// Get the section type, optional workpiece UID, and optional raster mode from an OpsSection command.
    ///
    /// :param idx: Command index.
    /// :returns: ``(SectionType, Optional[str], Optional[RasterMode])``.
    /// :raises TypeError: If the command is not an OpsSectionStart or OpsSectionEnd.
    /// :complexity: O(1) time, O(1) space
    fn section_params(
        &self,
        idx: usize,
    ) -> PyResult<(PySectionType, Option<String>, Option<PyRasterMode>)> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match &self.inner.commands[idx].category {
            OpCategory::Marker(MarkerCmd::OpsSectionStart {
                section_type,
                workpiece_uid,
                raster_mode,
                ..
            }) => Ok((
                PySectionType(*section_type),
                workpiece_uid.as_ref().map(|s| s.to_string()),
                raster_mode.map(PyRasterMode),
            )),
            OpCategory::Marker(MarkerCmd::OpsSectionEnd {
                section_type,
                raster_mode,
                ..
            }) => Ok((
                PySectionType(*section_type),
                None,
                raster_mode.map(PyRasterMode),
            )),
            _ => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Not an OpsSection command",
            )),
        }
    }

    /// Get the extra axes data for a moving command.
    ///
    /// :param idx: Command index.
    /// :returns: Dict mapping axis names to values, or None.
    /// :complexity: O(1) time, O(1) space
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

    /// Get the machine state stored on a command (if available).
    ///
    /// :param idx: Command index.
    /// :returns: The :class:`State` at that index, or None.
    /// :complexity: O(1) time, O(1) space
    fn state(&self, idx: usize) -> PyResult<Option<PyState>> {
        if idx >= self.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                "index out of range",
            ));
        }
        match self.inner.state(idx) {
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
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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
    ///
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(1) time, O(1) space
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
        self.inner.bezier_to(
            tuple_to_point3d(control1),
            tuple_to_point3d(control2),
            tuple_to_point3d(end),
            ea,
        );
        Ok(())
    }

    /// Add a quadratic bezier curve to the given endpoint.
    ///
    /// :param control: Control point ``(x, y, z)``.
    /// :param end: End point ``(x, y, z)``.
    /// :param extra: Optional dict of extra axis values.
    /// :complexity: O(1) time, O(1) space
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
        self.inner.quadratic_bezier_to(
            tuple_to_point3d(control),
            tuple_to_point3d(end),
            ea,
        );
        Ok(())
    }

    /// Set the cutting power for subsequent commands.
    ///
    /// :param power: Power level (0.0–1.0).
    /// :complexity: O(1) time, O(1) space
    fn set_power(&mut self, power: f64) {
        self.inner.set_power(power);
    }

    /// Set the feed rate for subsequent commands.
    ///
    /// :param feed_rate: Feed rate in mm/min.
    /// :complexity: O(1) time, O(1) space
    fn set_feed_rate(&mut self, feed_rate: f64) {
        self.inner.set_feed_rate(feed_rate as i32);
    }

    /// Set the rapid (traverse) rate for subsequent commands.
    ///
    /// :param rapid_rate: Rapid rate in mm/min.
    /// :complexity: O(1) time, O(1) space
    fn set_rapid_rate(&mut self, rapid_rate: f64) {
        self.inner.set_rapid_rate(rapid_rate as i32);
    }

    /// Pause execution for a given duration.
    ///
    /// :param duration_ms: Dwell duration in milliseconds.
    /// :complexity: O(1) time, O(1) space
    fn dwell(&mut self, duration_ms: f64) {
        self.inner.dwell(duration_ms);
    }

    /// Switch to a specific head by UID.
    ///
    /// :param head_uid: The head identifier.
    /// :complexity: O(1) time, O(1) space
    fn set_head(&mut self, head_uid: &str) {
        self.inner.set_head(head_uid);
    }

    /// Set the laser pulse frequency.
    ///
    /// :param frequency: Frequency in Hz.
    /// :complexity: O(1) time, O(1) space
    fn set_frequency(&mut self, frequency: i32) {
        self.inner.set_frequency(frequency);
    }

    /// Set the laser pulse width.
    ///
    /// :param pulse_width: Pulse width in microseconds.
    /// :complexity: O(1) time, O(1) space
    fn set_pulse_width(&mut self, pulse_width: f64) {
        self.inner.set_pulse_width(pulse_width);
    }

    /// Set the spindle RPM for subsequent commands.
    ///
    /// :param rpm: Spindle RPM.
    /// :complexity: O(1) time, O(1) space
    fn set_spindle_rpm(&mut self, rpm: u32) {
        self.inner.set_spindle_rpm(rpm);
    }

    /// Set the coolant mode for subsequent commands.
    ///
    /// :param mode: Coolant mode.
    /// :complexity: O(1) time, O(1) space
    fn set_coolant(&mut self, mode: &PyCoolantMode) {
        self.inner.set_coolant(mode.0);
    }

    /// Set the air assist mode for subsequent commands.
    ///
    /// :param mode: Air assist mode.
    /// :complexity: O(1) time, O(1) space
    fn set_air_assist(&mut self, mode: &PyAirAssistMode) {
        self.inner.set_air_assist(mode.0);
    }

    /// Set the head coolant mode for subsequent commands.
    ///
    /// :param mode: Head coolant mode.
    /// :complexity: O(1) time, O(1) space
    fn set_head_coolant(&mut self, mode: &PyHeadCoolantMode) {
        self.inner.set_head_coolant(mode.0);
    }

    /// Emit the state commands needed to reach *state*.
    ///
    /// Power is always emitted (default 0.0). All other fields are
    /// emitted only when set (non-None). Domain-neutral: does not
    /// decide what values to use, just emits them.
    ///
    /// :param state: The target state to apply.
    /// :complexity: O(k) time where k = number of set fields, O(k) space
    fn apply_state(&mut self, state: &PyState) {
        self.inner.apply_state(&state.0);
    }

    /// Add a scan-line move with per-pixel power values.
    ///
    /// :param x: End X coordinate.
    /// :param y: End Y coordinate.
    /// :param z: End Z coordinate (default 0.0).
    /// :param power_values: Optional per-pixel 8-bit power values.
    /// :param extra: Optional dict of extra axis values.
    /// :complexity: O(1) time, O(1) space
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
    ///
    /// :complexity: O(1) time, O(1) space
    fn job_start(&mut self) {
        self.inner.job_start();
    }

    /// Mark the end of a job.
    ///
    /// :complexity: O(1) time, O(1) space
    fn job_end(&mut self) {
        self.inner.job_end();
    }

    /// Mark the start of a layer.
    ///
    /// :param layer_uid: The layer identifier.
    /// :complexity: O(1) time, O(1) space
    fn layer_start(&mut self, layer_uid: &str) {
        self.inner.layer_start(layer_uid);
    }

    /// Mark the end of a layer.
    ///
    /// :param layer_uid: The layer identifier.
    /// :complexity: O(1) time, O(1) space
    fn layer_end(&mut self, layer_uid: &str) {
        self.inner.layer_end(layer_uid);
    }

    /// Mark the start of a workpiece.
    ///
    /// :param workpiece_uid: The workpiece identifier.
    /// :complexity: O(1) time, O(1) space
    fn workpiece_start(&mut self, workpiece_uid: &str) {
        self.inner.workpiece_start(workpiece_uid);
    }

    /// Mark the end of a workpiece.
    ///
    /// :param workpiece_uid: The workpiece identifier.
    /// :complexity: O(1) time, O(1) space
    fn workpiece_end(&mut self, workpiece_uid: &str) {
        self.inner.workpiece_end(workpiece_uid);
    }

    /// Mark the start of an ops section.
    ///
    /// :param section_type: The type of section.
    /// :param workpiece_uid: The workpiece identifier.
    /// :param raster_mode: Optional raster mode.
    /// :raises ValueError: If section_type is RasterFill without a raster_mode,
    ///     or VectorOutline with a raster_mode.
    /// :complexity: O(1) time, O(1) space
    #[pyo3(signature = (section_type, workpiece_uid, *, raster_mode=None))]
    fn ops_section_start(
        &mut self,
        section_type: &PySectionType,
        workpiece_uid: &str,
        raster_mode: Option<&PyRasterMode>,
    ) -> PyResult<()> {
        self.inner
            .ops_section_start(
                section_type.0,
                workpiece_uid,
                raster_mode.map(|rm| rm.0),
            )
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Mark the end of an ops section.
    ///
    /// :param section_type: The type of section.
    /// :param raster_mode: Optional raster mode.
    /// :raises ValueError: If section_type is RasterFill without a raster_mode,
    ///     or VectorOutline with a raster_mode.
    /// :complexity: O(1) time, O(1) space
    #[pyo3(signature = (section_type, *, raster_mode=None))]
    fn ops_section_end(
        &mut self,
        section_type: &PySectionType,
        raster_mode: Option<&PyRasterMode>,
    ) -> PyResult<()> {
        self.inner
            .ops_section_end(section_type.0, raster_mode.map(|rm| rm.0))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Mark the start of a state block.
    ///
    /// :param name: Optional block name.
    /// :complexity: O(1) time, O(1) space
    fn state_block_start(&mut self, name: Option<&str>) {
        self.inner.state_block_start(name);
    }

    /// Mark the end of a state block.
    ///
    /// :complexity: O(1) time, O(1) space
    fn state_block_end(&mut self) {
        self.inner.state_block_end();
    }

    // --- Copy / Transfer ---

    /// Return a deep copy of this Ops sequence.
    ///
    /// :returns: A new ``Ops`` instance.
    /// :complexity: O(n) time, O(n) space
    fn copy(&self) -> PyOps {
        PyOps {
            inner: self.inner.copy(),
        }
    }

    /// Shallow copy (same as :meth:`copy` since Ops is immutable).
    ///
    /// :complexity: O(n) time, O(n) space
    fn __copy__(&self) -> PyOps {
        self.copy()
    }

    /// Deep copy (same as :meth:`copy` since Ops is immutable).
    ///
    /// :complexity: O(n) time, O(n) space
    fn __deepcopy__(&self, _memo: Bound<'_, PyAny>) -> PyOps {
        self.copy()
    }

    /// Copy a single command from another Ops sequence into this one.
    ///
    /// :param source: The source Ops sequence.
    /// :param idx: Index of the command to copy.
    /// :complexity: O(1) time, O(1) space
    fn copy_command_from(&mut self, source: &PyOps, idx: usize) {
        self.inner.copy_command_from(&source.inner, idx);
    }

    /// Transfer (move) a single command from another Ops sequence into this one.
    ///
    /// :param source: The source Ops sequence.
    /// :param idx: Index of the command to transfer.
    /// :complexity: O(1) time, O(1) space
    fn transfer_command_from(&mut self, source: &PyOps, idx: usize) {
        self.inner.transfer_command_from(&source.inner, idx);
    }

    /// Extend this Ops sequence with commands from another.
    ///
    /// :param other: The other Ops sequence (or None for no-op).
    /// :complexity: O(n) time, O(n) space
    fn extend(&mut self, other: Option<&PyOps>) {
        if let Some(other) = other {
            self.inner.extend(&other.inner);
        }
    }

    /// Remove all commands from this Ops sequence.
    ///
    /// :complexity: O(1) time, O(1) space
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Return index ranges for each subpath.
    ///
    /// :returns: A list of index lists, one per subpath.
    /// :complexity: O(n) time, O(n) space
    fn subpath_indices(&self) -> Vec<Vec<usize>> {
        self.inner.subpath_indices()
    }

    /// Split this Ops sequence into separate subpaths.
    ///
    /// :returns: A list of Ops sequences, one per subpath.
    /// :complexity: O(n) time, O(n) space
    fn split_into_subpaths(&self) -> Vec<PyOps> {
        self.inner
            .split_into_subpaths()
            .into_iter()
            .map(|o| PyOps { inner: o })
            .collect()
    }

    /// Split the sequence at paired markers of the given type.
    ///
    /// Returns a list of ``Ops`` sequences. Each matched start/end marker pair
    /// yields one ``Ops`` containing the markers and their content. Commands
    /// that fall outside any pair are returned as additional ``Ops`` segments,
    /// so concatenating all returned sequences reproduces the original.
    ///
    /// :param command_type: ``CommandType.LAYER_START``,
    ///     ``WORKPIECE_START``, ``OPS_SECTION_START``, or ``JOB_START``.
    /// :returns: A list of ``Ops`` sequences.
    /// :raises ValueError: If ``command_type`` is not a supported start marker.
    /// :complexity: O(n) time, O(n) space
    fn split_at(
        &self,
        command_type: &Bound<'_, PyAny>,
    ) -> PyResult<Vec<PyOps>> {
        let raw: u8 = command_type.getattr("value")?.extract()?;
        let ct = CommandType::try_from(raw).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "invalid CommandType value: {raw}"
            ))
        })?;
        let valid = matches!(
            ct,
            CommandType::LayerStart
                | CommandType::WorkpieceStart
                | CommandType::OpsSectionStart
                | CommandType::JobStart
        );
        if !valid {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unsupported marker type: {ct}. Use LAYER_START, WORKPIECE_START, OPS_SECTION_START, or JOB_START"
            )));
        }
        Ok(self
            .inner
            .split_at(ct)
            .into_iter()
            .map(|o| PyOps { inner: o })
            .collect())
    }

    /// Reverse the order of subpaths.
    ///
    /// :returns: A new Ops with subpath order reversed.
    /// :complexity: O(n) time, O(n) space
    fn flip_ops(&self) -> PyOps {
        PyOps {
            inner: self.inner.flip_ops(),
        }
    }

    /// Return a copy with all state commands removed.
    ///
    /// :returns: A new Ops containing only moving commands.
    /// :complexity: O(n) time, O(n) space
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
    /// :complexity: O(1) time, O(1) space
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
    /// :complexity: O(n) time, O(n) space
    fn sub_ops(&self, indices: Vec<usize>) -> PyOps {
        PyOps {
            inner: self.inner.sub_ops(&indices),
        }
    }

    /// Replace all commands in this sequence with those from another.
    ///
    /// :param source: The source Ops sequence.
    /// :complexity: O(n) time, O(n) space
    fn replace_all(&mut self, source: &PyOps) {
        self.inner.replace_all(&source.inner);
    }

    /// Replace the internal buffer of this sequence with a copy from another.
    ///
    /// :param source: The source Ops sequence.
    /// :complexity: O(n) time, O(n) space
    fn replace_with(&mut self, source: &PyOps) {
        self.inner.replace_with(&source.inner);
    }

    /// Create an Ops sequence from a Geometry.
    ///
    /// :param geometry: The geometry to convert.
    /// :returns: A new ``Ops`` instance.
    /// :complexity: O(n) time, O(n) space
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

    /// Build an Ops sequence from a 3-D polyline.
    ///
    /// When *move_first* is ``True`` the first point is emitted as a
    /// MoveTo and subsequent points as LineTo.  When *move_first* is
    /// ``False`` every point is emitted as a LineTo (useful for
    /// appending a polyline to an in-progress cut).
    ///
    /// When *state* is provided, its commands (power, feed rate, etc.)
    /// are applied before the polyline points.
    ///
    /// :param points: List of ``(x, y, z)`` tuples.
    /// :param move_first: Whether to emit the first point as a MoveTo.
    /// :param state: Optional machine state to apply before the path.
    /// :returns: A new ``Ops`` instance.
    /// :complexity: O(n) where n = number of points
    #[staticmethod]
    #[pyo3(signature = (points, move_first=true, state=None))]
    fn from_polyline(
        points: Vec<(f64, f64, f64)>,
        move_first: bool,
        state: Option<Bound<'_, PyState>>,
    ) -> Self {
        let pts: Vec<Point3D> = points
            .into_iter()
            .map(|(x, y, z)| Point3D::new(x, y, z))
            .collect();
        let ops = if let Some(ref s) = state {
            crate::ops::Ops::from_polyline(
                &pts,
                move_first,
                Some(&s.borrow().0),
            )
        } else {
            crate::ops::Ops::from_polyline(&pts, move_first, None)
        };
        PyOps { inner: ops }
    }

    /// Convert this Ops sequence back into a Geometry.
    ///
    /// :returns: A Geometry representing the same paths.
    /// :complexity: O(n) time, O(n) space
    fn to_geometry(&self) -> PyGeometry {
        PyGeometry {
            inner: self.inner.to_geometry(),
        }
    }

    /// Pre-compute and store the accumulated state at each moving command.
    ///
    /// :complexity: O(n) time, O(n) space
    fn preload_state(&mut self) {
        self.inner.preload_state();
    }

    /// Apply a state to all moving commands without an explicit state.
    ///
    /// :param state: The state to apply.
    /// :complexity: O(n) time, O(1) space
    fn set_state_on_moving(&mut self, state: &PyState) {
        self.inner.set_state_on_moving(&state.0);
    }

    /// Overwrite the state at a specific command index.
    ///
    /// :param idx: The command index.
    /// :param state: The new state.
    /// :complexity: O(1) time, O(1) space
    fn set_state_at(&mut self, idx: usize, state: &PyState) {
        self.inner.set_state_at(idx, &state.0);
    }

    /// Print a human-readable dump of all commands.
    ///
    /// :complexity: O(n) time, O(n) space
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
    /// :complexity: O(1) time, O(1) space
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
            feed_rate: None,
            rapid_rate: None,
            frequency: None,
            pulse_width: None,
            head_uid: None,
            spindle_rpm: None,
            coolant: None,
            air_assist: None,
            head_coolant: None,
            duration_ms: None,
            layer_uid: None,
            workpiece_uid: None,
            section_type: None,
        };

        if inner.commands[idx].is_moving() {
            info.end = Some(point3d_to_tuple(inner.commands[idx].end_point()));
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
                    info.center_offset = Some((center.x, center.y));
                    info.clockwise = Some(*cw);
                }
                MoveCmd::BezierTo { control1, control2 } => {
                    info.control1 = Some(point3d_to_tuple(*control1));
                    info.control2 = Some(point3d_to_tuple(*control2));
                }
                MoveCmd::QuadraticBezierTo { control } => {
                    info.control = Some(point3d_to_tuple(*control));
                }
                MoveCmd::ScanLine { power_values } => {
                    info.power_values =
                        Some(PyBytes::new(py, power_values.as_ref()).unbind());
                }
                _ => {}
            },
            OpCategory::State(cmd) => match cmd {
                StateCmd::SetPower(p) => info.power = Some(*p),
                StateCmd::SetFeedRate(s) => info.feed_rate = Some(*s),
                StateCmd::SetRapidRate(s) => info.rapid_rate = Some(*s),
                StateCmd::SetFrequency(f) => info.frequency = Some(*f),
                StateCmd::SetPulseWidth(pw) => info.pulse_width = Some(*pw),
                StateCmd::SetSpindleRpm(s) => info.spindle_rpm = Some(*s),
                StateCmd::SetCoolant(mode) => {
                    info.coolant = Some(PyCoolantMode(*mode))
                }
                StateCmd::SetAirAssist(mode) => {
                    info.air_assist = Some(PyAirAssistMode(*mode))
                }
                StateCmd::SetHeadCoolant(mode) => {
                    info.head_coolant = Some(PyHeadCoolantMode(*mode))
                }
                StateCmd::SetHead(uid) => info.head_uid = Some(uid.to_string()),
                StateCmd::Dwell(d) => info.duration_ms = Some(*d),
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
                    ..
                } => {
                    info.section_type = Some(PySectionType(*section_type));
                    if let Some(wp) = workpiece_uid {
                        info.workpiece_uid = Some(wp.to_string());
                    }
                }
                MarkerCmd::OpsSectionEnd { section_type, .. } => {
                    info.section_type = Some(PySectionType(*section_type));
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
    /// :complexity: O(n) time, O(1) space
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
    /// :complexity: O(n) time, O(1) space
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
    /// :complexity: O(n) time, O(1) space
    fn rotate(&mut self, angle_deg: f64, cx: f64, cy: f64) -> PyResult<()> {
        self.inner.rotate(angle_deg, cx, cy);
        Ok(())
    }

    #[gen_stub(skip)]
    fn transform(&mut self, matrix: &Bound<'_, PyAny>) -> PyResult<()> {
        let m = if let Ok(py_m) = matrix.extract::<PyMatrix>() {
            py_m.inner.to_4x4()
        } else if let Ok(rows) = matrix.extract::<Vec<Vec<f64>>>() {
            if rows.len() != 4 || rows.iter().any(|r| r.len() != 4) {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "transform requires a 4x4 matrix",
                ));
            }
            DMat4::from_cols(
                DVec4::new(rows[0][0], rows[1][0], rows[2][0], rows[3][0]),
                DVec4::new(rows[0][1], rows[1][1], rows[2][1], rows[3][1]),
                DVec4::new(rows[0][2], rows[1][2], rows[2][2], rows[3][2]),
                DVec4::new(rows[0][3], rows[1][3], rows[2][3], rows[3][3]),
            )
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "expected a Matrix or a 4x4 list of lists",
            ));
        };
        self.inner.transform(m);
        Ok(())
    }

    // --- Clipping ---

    /// Clip this sequence to a rectangle, keeping only commands inside.
    ///
    /// :param rect: ``(x_min, y_min, x_max, y_max)``.
    /// :returns: A new Ops sequence containing the clipped commands.
    /// :complexity: O(n) time, O(n) space
    fn clip_rect(&self, rect: (f64, f64, f64, f64)) -> PyOps {
        PyOps {
            inner: self
                .inner
                .clip_rect(Rect::new(rect.0, rect.1, rect.2, rect.3)),
        }
    }

    /// Subtract polygonal regions from the cutting paths.
    ///
    /// :param regions: List of polygons, each being a list of ``(x, y)`` vertices.
    /// :complexity: O(n * m) time, O(n) space where m is the number of polygon vertices
    fn subtract_regions(
        &mut self,
        regions: Vec<Vec<(f64, f64)>>,
    ) -> PyResult<()> {
        let regions = polygons_from_tuples(regions);
        self.inner.subtract_regions(&regions);
        Ok(())
    }

    /// Clip paths to the given polygonal regions, keeping only what is inside.
    ///
    /// :param regions: List of polygons, each being a list of ``(x, y)`` vertices.
    /// :param tolerance: Approximation tolerance (default 0.3).
    /// :complexity: O(n * m) time, O(n) space where m is the number of polygon vertices
    #[pyo3(signature = (regions, tolerance = 0.3))]
    fn clip_to_regions(
        &mut self,
        regions: Vec<Vec<(f64, f64)>>,
        tolerance: f64,
    ) -> PyResult<()> {
        let regions: Vec<Vec<Point>> = regions
            .into_iter()
            .map(|r| r.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        self.inner.clip_to_regions(&regions, tolerance);
        Ok(())
    }

    /// Clip paths using polygonal regions as boundaries; keeps what is inside.
    ///
    /// :param regions: List of polygons, each being a list of ``(x, y)`` vertices.
    /// :param tolerance: Approximation tolerance (default 0.3).
    /// :complexity: O(n * m) time, O(n) space where m is the number of polygon vertices
    #[pyo3(signature = (regions, tolerance = 0.3))]
    fn clip_ops_to_regions(
        &mut self,
        regions: Vec<Vec<(f64, f64)>>,
        tolerance: f64,
    ) -> PyResult<()> {
        let regions = polygons_from_tuples(regions);
        self.inner.clip_ops_to_regions(&regions, tolerance);
        Ok(())
    }

    /// Clip at a single vertical swath, keeping commands that intersect the band.
    ///
    /// :param x: X coordinate of the left edge.
    /// :param y: Y coordinate (used to find the relevant segment).
    /// :param width: Width of the band.
    /// :returns: True if any commands were kept.
    /// :complexity: O(n) time, O(1) space
    fn clip_at(&mut self, x: f64, y: f64, width: f64) -> bool {
        self.inner.clip_at(x, y, width)
    }

    /// Translate each layer by its own offset, with a default fallback.
    ///
    /// :param default_offset: The ``(x, y, z)`` offset for layers not listed in layer_offsets.
    /// :param layer_offsets: Optional dict mapping layer UIDs to ``(x, y, z)`` offsets.
    /// :complexity: O(n) time, O(1) space
    #[pyo3(signature = (default_offset, layer_offsets = None))]
    #[allow(clippy::type_complexity)]
    fn translate_layers(
        &mut self,
        default_offset: (f64, f64, f64),
        layer_offsets: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let parsed_layer_offsets: Option<Vec<(String, (f64, f64, f64))>> =
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
            .translate_layers(default_offset, parsed_layer_offsets.as_deref());
        Ok(())
    }

    /// Transform each layer by calling a Python callback with the layer UID and ops.
    ///
    /// The callback receives ``(layer_uid: str, layer_ops: Ops)`` and should
    /// mutate the layer_ops in place.
    ///
    /// :param callback: A callable accepting ``(layer_uid, layer_ops)``.
    /// :complexity: O(n) time, O(n) space
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
            let new_cmds = layer_ops_ref.inner.commands.clone();
            let new_len = new_cmds.len();

            self.inner
                .cmds_mut()
                .splice(layer_start..layer_end, new_cmds.iter().cloned());
            i = layer_start + new_len;
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
    /// :complexity: O(n) time, O(1) space
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
            let end_list = vec![end.x, end.y, end.z];
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
                &mut self.inner.cmds_mut()[i].category
            {
                *e = Point3D::new(new_end[0], new_end[1], new_end[2]);
            }

            let ea_vec = py_to_axis_map(&ea_arg)?;
            if ea_vec.is_empty() {
                self.inner.cmds_mut()[i].clear_extra_axes();
            } else {
                self.inner.cmds_mut()[i]
                    .set_extra_axes(std::sync::Arc::from(ea_vec));
            }

            if let Some(ref aux_cb) = on_aux_point {
                if let OpCategory::Moving { cmd, .. } =
                    &mut self.inner.cmds_mut()[i].category
                {
                    match cmd {
                        MoveCmd::ArcTo { center, .. } => {
                            let off_list = vec![center.x, center.y];
                            let off_py_list = PyList::new(py, &off_list)?;
                            aux_cb.call1(py, (&off_py_list,))?;
                            let new_off: Vec<f64> = off_py_list.extract()?;
                            *center = Point::new(new_off[0], new_off[1]);
                        }
                        MoveCmd::BezierTo {
                            control1, control2, ..
                        } => {
                            for cp in [control1, control2].iter_mut() {
                                let cp_list = vec![cp.x, cp.y, cp.z];
                                let cp_py_list = PyList::new(py, &cp_list)?;
                                aux_cb.call1(py, (&cp_py_list,))?;
                                let new_cp: Vec<f64> = cp_py_list.extract()?;
                                **cp = Point3D::new(
                                    new_cp[0], new_cp[1], new_cp[2],
                                );
                            }
                        }
                        MoveCmd::QuadraticBezierTo { control, .. } => {
                            let cp_list = vec![control.x, control.y, control.z];
                            let cp_py_list = PyList::new(py, &cp_list)?;
                            aux_cb.call1(py, (&cp_py_list,))?;
                            let new_cp: Vec<f64> = cp_py_list.extract()?;
                            *control =
                                Point3D::new(new_cp[0], new_cp[1], new_cp[2]);
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
    /// :complexity: O(n) time, O(n) space
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
                inner: self.inner.linearize(idx, tuple_to_point3d(start_point)),
            }),
            _ => Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "Cannot linearize command at index {}: {:?}",
                idx, ct,
            ))),
        }
    }

    /// Replace all curved commands with linear approximations in-place.
    ///
    /// :complexity: O(n) time, O(n) space
    fn linearize_all(&mut self) {
        self.inner.linearize_all();
    }

    /// Replace only bezier and quadratic bezier curves with linear approximations.
    ///
    /// :complexity: O(n) time, O(n) space
    fn linearize_curves(&mut self) {
        self.inner.linearize_curves();
    }

    /// Replace only arc commands with linear approximations.
    ///
    /// :complexity: O(n) time, O(n) space
    fn linearize_arcs(&mut self) {
        self.inner.linearize_arcs();
    }

    /// Return index ranges for each contiguous cutting segment.
    ///
    /// :returns: A list of index lists, one per segment.
    /// :complexity: O(n) time, O(n) space
    fn segment_indices(&self) -> Vec<Vec<usize>> {
        self.inner.segment_indices()
    }

    /// Group contiguous commands with the same auxiliary state into separate Ops sequences.
    ///
    /// Groups by continuity of auxiliary state (coolant, air_assist,
    /// head_coolant) only. For full parameter-regime grouping, use
    /// :meth:`OpsSection.state_blocks` with ``StateBlockStart``/``StateBlockEnd`` markers.
    ///
    /// :returns: A list of Ops sequences grouped by auxiliary state continuity.
    /// :complexity: O(n) time, O(n) space
    fn group_by_auxiliary_state(&self) -> Vec<PyOps> {
        self.inner
            .group_by_auxiliary_state()
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
    /// :complexity: O(n) time, O(n) space
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
    /// :complexity: O(n) time, O(n) space
    fn section_ranges(&self) -> Vec<PyOpsSectionRange> {
        self.inner
            .iter_section_ranges()
            .into_iter()
            .map(PyOpsSectionRange)
            .collect()
    }

    /// Extract the commands belonging to a section.
    ///
    /// :param section: The OpsSection to extract.
    /// :returns: A new Ops containing only the content of that section.
    /// :complexity: O(n) time, O(n) space
    fn section_content(&self, section: &PyOpsSection) -> PyOps {
        PyOps {
            inner: self.inner.section_ops(&section.0),
        }
    }

    /// Return sections matching a given section type.
    ///
    /// :param section_type: The SectionType to filter by.
    /// :returns: List of matching OpsSection objects.
    /// :complexity: O(n) time, O(n) space
    fn sections_by_type(
        &self,
        section_type: &PySectionType,
    ) -> Vec<PyOpsSection> {
        self.inner
            .sections_by_type(section_type.0)
            .into_iter()
            .map(PyOpsSection)
            .collect()
    }

    /// Return sections matching a given raster mode.
    ///
    /// :param raster_mode: The RasterMode to filter by.
    /// :returns: List of matching OpsSection objects.
    /// :complexity: O(n) time, O(n) space
    fn sections_by_mode(
        &self,
        raster_mode: &PyRasterMode,
    ) -> Vec<PyOpsSection> {
        self.inner
            .sections_by_mode(raster_mode.0)
            .into_iter()
            .map(PyOpsSection)
            .collect()
    }

    /// Return all state blocks across all sections.
    ///
    /// :returns: List of StateBlock objects.
    /// :raises RuntimeError: If state block nesting is invalid.
    /// :complexity: O(n) time, O(n) space
    fn state_blocks(&self) -> PyResult<Vec<PyStateBlock>> {
        self.inner
            .state_blocks_all()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
            })
            .map(|blocks| {
                blocks
                    .into_iter()
                    .map(|b| PyStateBlock {
                        name: b.name.as_ref().map(|s| s.to_string()),
                        marker_indices: b.marker_indices,
                        content_indices: b.content_indices,
                    })
                    .collect()
            })
    }

    /// Compute the bounding rectangle of all commands.
    ///
    /// :param include_travel: Whether to include travel moves (default False).
    /// :returns: ``(x_min, y_min, x_max, y_max)``.
    /// :complexity: O(n) time, O(1) space
    #[pyo3(signature = (include_travel = false))]
    fn rect(&self, include_travel: bool) -> (f64, f64, f64, f64) {
        match self.inner.rect(include_travel) {
            Some(r) => (r.min.x, r.min.y, r.max.x, r.max.y),
            None => (0.0, 0.0, 0.0, 0.0),
        }
    }

    /// Extract a frame (first and last endpoints) from the sequence.
    ///
    /// :param power: Laser power value (0.0 to 1.0).
    /// :param feed_rate: Optional feed rate to set on the frame commands.
    /// :returns: A new Ops containing only the frame endpoints.
    /// :complexity: O(n) time, O(n) space
    #[pyo3(signature = (power = None, feed_rate = None))]
    fn get_frame(&self, power: Option<f64>, feed_rate: Option<f64>) -> PyOps {
        PyOps {
            inner: self.inner.get_frame(power, feed_rate),
        }
    }

    /// Estimate the total processing time for this sequence.
    ///
    /// :param default_feed_rate: Default feed rate (default 1000.0).
    /// :param default_rapid_rate: Default rapid rate (default 3000.0).
    /// :param acceleration: Acceleration value (default 1000.0).
    /// :returns: Estimated time in seconds.
    /// :complexity: O(n) time, O(1) space
    #[pyo3(signature = (default_feed_rate = 1000.0, default_rapid_rate = 3000.0, acceleration = 1000.0))]
    fn estimate_time(
        &mut self,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> f64 {
        self.inner.estimate_time(
            default_feed_rate,
            default_rapid_rate,
            acceleration,
        )
    }

    /// Estimate the time of each individual command in the sequence.
    ///
    /// Returns a list with one entry per command. Moving commands
    /// (MoveTo, LineTo, ArcTo, etc.) yield their estimated execution
    /// time in seconds. Dwell commands yield their dwell duration in
    /// seconds. Other non-moving commands (state changes, markers)
    /// yield 0.0.
    ///
    /// :param default_feed_rate: Default feed rate (default 1000.0).
    /// :param default_rapid_rate: Default rapid rate (default 3000.0).
    /// :param acceleration: Acceleration value (default 1000.0).
    /// :returns: List of estimated times in seconds, one per command.
    /// :complexity: O(n) time, O(n) space
    #[pyo3(signature = (default_feed_rate = 1000.0, default_rapid_rate = 3000.0, acceleration = 1000.0))]
    fn estimate_command_times(
        &mut self,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> Vec<f64> {
        self.inner.estimate_command_times(
            default_feed_rate,
            default_rapid_rate,
            acceleration,
        )
    }

    /// Cumulative execution time (seconds) of every command in the
    /// sequence.
    ///
    /// Returns a list with one entry per command, where entry *i* is
    /// the total simulated time elapsed once command *i* has executed.
    /// State changes (except dwells) and markers contribute zero time.
    /// The result is cached per parameter set and invalidated when the
    /// ops are mutated.
    ///
    /// :param default_feed_rate: Default feed rate (default 1000.0).
    /// :param default_rapid_rate: Default rapid rate (default 3000.0).
    /// :param acceleration: Acceleration value (default 1000.0).
    /// :returns: List of cumulative times in seconds, one per command.
    /// :complexity: O(n) time, O(1) space
    #[pyo3(signature = (default_feed_rate = 1000.0, default_rapid_rate = 3000.0, acceleration = 1000.0))]
    fn build_cumulative_time_index(
        &mut self,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> Vec<f64> {
        self.inner
            .build_cumulative_time_index(
                default_feed_rate,
                default_rapid_rate,
                acceleration,
            )
            .to_vec()
    }

    /// Find the command index in effect at simulated time *t* (seconds).
    ///
    /// Returns the largest index whose cumulative execution time is
    /// <= *t*, clamped to ``[0, len-1]``. Returns 0 for an empty ops
    /// or for times before the first command's completion.
    ///
    /// :param t: Simulated time in seconds.
    /// :param default_feed_rate: Default feed rate (default 1000.0).
    /// :param default_rapid_rate: Default rapid rate (default 3000.0).
    /// :param acceleration: Acceleration value (default 1000.0).
    /// :returns: The command index in effect at time *t*.
    /// :complexity: O(n) time, O(1) space
    #[pyo3(signature = (t, default_feed_rate = 1000.0, default_rapid_rate = 3000.0, acceleration = 1000.0))]
    fn find_index_at_time(
        &mut self,
        t: f64,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> usize {
        self.inner.find_index_at_time(
            t,
            default_feed_rate,
            default_rapid_rate,
            acceleration,
        )
    }

    /// Cumulative simulated time (seconds) up to and including command *idx*.
    ///
    /// Out-of-range indices clamp to the nearest valid command; empty
    /// ops yield 0.0.
    ///
    /// :param idx: Command index.
    /// :param default_feed_rate: Default feed rate (default 1000.0).
    /// :param default_rapid_rate: Default rapid rate (default 3000.0).
    /// :param acceleration: Acceleration value (default 1000.0).
    /// :returns: Cumulative simulated time in seconds.
    /// :complexity: O(n) time, O(1) space
    #[pyo3(signature = (idx, default_feed_rate = 1000.0, default_rapid_rate = 3000.0, acceleration = 1000.0))]
    fn get_cumulative_time_at(
        &mut self,
        idx: usize,
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> f64 {
        self.inner.get_cumulative_time_at(
            idx,
            default_feed_rate,
            default_rapid_rate,
            acceleration,
        )
    }

    // --- Properties ---

    /// The last ``(x, y, z)`` endpoint from a MoveTo command.
    ///
    /// :complexity: O(1) time, O(1) space
    #[getter]
    fn get_last_move_to(&self) -> (f64, f64, f64) {
        point3d_to_tuple(self.inner.last_move_to)
    }

    /// Set the last move-to position.
    ///
    /// :complexity: O(1) time, O(1) space
    #[setter]
    fn set_last_move_to(&mut self, val: (f64, f64, f64)) {
        self.inner.last_move_to = tuple_to_point3d(val);
    }

    /// Serialize this Ops sequence to a dict suitable for JSON export.
    ///
    /// :returns: A Python dict representation.
    /// :complexity: O(n) time, O(n) space
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        super::convert::dict::ops_to_dict(py, &self.inner)
    }

    /// Create an Ops sequence from a dictionary.
    ///
    /// :param data: Dictionary as produced by to_dict.
    /// :returns: A new ``Ops`` instance.
    /// :complexity: O(n) time, O(n) space
    #[classmethod]
    fn from_dict(
        _cls: &Bound<'_, PyType>,
        data: &Bound<'_, PyDict>,
    ) -> PyResult<Self> {
        let inner = super::convert::dict::ops_from_dict(data)?;
        Ok(PyOps { inner })
    }

    /// Serialize this Ops sequence to numpy arrays.
    ///
    /// :returns: A Python dict of numpy arrays.
    /// :complexity: O(n) time, O(n) space
    fn to_numpy_arrays(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        super::convert::numpy::ops_to_numpy_arrays(py, &self.inner)
    }

    /// Create an Ops sequence from numpy arrays.
    ///
    /// :param arrays: Dictionary as produced by to_numpy_arrays.
    /// :returns: A new ``Ops`` instance.
    /// :complexity: O(n) time, O(n) space
    #[classmethod]
    fn from_numpy_arrays(
        _cls: &Bound<'_, PyType>,
        arrays: &Bound<'_, PyDict>,
    ) -> PyResult<Self> {
        let inner = super::convert::numpy::ops_from_numpy_arrays(arrays)?;
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
    /// :complexity: O(n * k) time, O(1) space where k is the number of tab clips
    fn apply_tab_gaps(&mut self, clips: Vec<(f64, f64, f64)>) {
        let clip_points: Vec<crate::ops::transform::tabs::ClipPoint> = clips
            .into_iter()
            .map(|(x, y, width)| crate::ops::transform::tabs::ClipPoint {
                x,
                y,
                width,
            })
            .collect();
        crate::ops::transform::tabs::apply_tab_gaps(
            &mut self.inner,
            &clip_points,
        );
    }

    /// Apply holding tabs by reducing power in tab regions.
    ///
    /// Instead of cutting a gap, the power is lowered in the tab
    /// area so the material stays connected but weaker. Only
    /// ``VECTOR_OUTLINE`` sections are modified.
    ///
    /// :param clips: List of ``(x, y, width)`` tuples defining tab positions.
    /// :param tab_power: Power level inside tab regions (0.0–1.0).
    /// :param original_power: Normal cutting power to restore after the tab.
    /// :complexity: O(n * k) time, O(1) space where k is the number of tab clips
    fn apply_tab_power(
        &mut self,
        clips: Vec<(f64, f64, f64)>,
        tab_power: f64,
        original_power: f64,
    ) {
        let clip_points: Vec<crate::ops::transform::tabs::ClipPoint> = clips
            .into_iter()
            .map(|(x, y, width)| crate::ops::transform::tabs::ClipPoint {
                x,
                y,
                width,
            })
            .collect();
        crate::ops::transform::tabs::apply_tab_power(
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
    /// :complexity: O(n log n) average time, O(n) space
    fn merge_overlapping_lines(&mut self, tolerance: f64) {
        crate::ops::transform::merge_lines::merge_overlapping_lines(
            &mut self.inner,
            tolerance,
        );
    }

    /// Smooth all line-only segments using a Gaussian filter.
    ///
    /// Arcs are linearized first.  Segments containing curves are
    /// transferred unchanged.  The smoothing operates in place.
    ///
    /// :param amount: Smoothing strength (0-100).  0 is a no-op.
    /// :param corner_angle_threshold: Corners with an internal angle
    ///     (in degrees) smaller than this are preserved.
    /// :complexity: O(n * k) time, O(n) space where k is the kernel size
    fn smooth(&mut self, amount: u32, corner_angle_threshold: f64) {
        self.inner.smooth(amount, corner_angle_threshold);
    }

    /// Apply overscan to raster lines.
    ///
    /// Extends raster line start/end points by ``distance_mm`` along
    /// the line direction, adding zero-power lead-in and lead-out
    /// segments for constant engraving velocity.
    ///
    /// :param distance_mm: Overscan distance in millimeters.
    /// :complexity: O(n) time, O(n) space
    fn apply_overscan(&mut self, distance_mm: f64) {
        crate::ops::transform::overscan::apply_overscan(
            &mut self.inner,
            distance_mm,
        );
    }

    /// Correct X misalignment between left-to-right and right-to-left
    /// raster passes.
    ///
    /// For every raster pass (a ``MoveTo`` followed by a ``ScanLine``),
    /// if the pass runs right-to-left, both the entry ``MoveTo`` and
    /// the ``ScanLine`` endpoint are shifted along X by ``offset_mm``.
    /// Left-to-right passes are left untouched.
    ///
    /// :param offset_mm: Offset in millimeters to apply to RTL passes.
    /// :complexity: O(n) time, O(n) space
    fn apply_bidir_scan_offset(&mut self, offset_mm: f64) {
        crate::ops::transform::bidir_scan_offset::apply_bidir_scan_offset(
            &mut self.inner,
            offset_mm,
        );
    }

    /// Repeats the ops sequence multiple times, optionally stepping
    /// down the Z axis after each pass.
    ///
    /// :param passes: Total number of passes (must be >= 1).
    /// :param z_step_down: Z distance to move down after each pass.
    /// :complexity: O(n * passes) time, O(n * passes) space
    fn apply_multipass(&mut self, passes: u32, z_step_down: f64) {
        crate::ops::transform::multipass::apply_multipass(
            &mut self.inner,
            passes,
            z_step_down,
        );
    }

    /// Apply lead-in and lead-out to vector contour paths.
    ///
    /// For each contour within a VECTOR_OUTLINE section, extends the
    /// toolpath with zero-power lead-in and lead-out segments along
    /// the tangent direction at the path start and end.
    ///
    /// :param lead_in_mm: Lead-in distance in millimeters.
    /// :param lead_out_mm: Lead-out distance in millimeters.
    /// :complexity: O(n) time, O(n) space
    fn apply_lead_in_out(&mut self, lead_in_mm: f64, lead_out_mm: f64) {
        crate::ops::transform::lead_in_out::apply_lead_in_out(
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
    /// :complexity: O(n²) average time, O(n) space
    #[pyo3(signature = (allow_flip=true, preserve_first=false, preserve_order=Vec::new(), progress_cb=None))]
    fn optimize_travel(
        &mut self,
        allow_flip: bool,
        preserve_first: bool,
        preserve_order: Vec<String>,
        progress_cb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let py_callbacks =
            PyCallableCallbacks::new(progress_cb.map(|b| b.clone().unbind()));
        crate::ops::transform::optimize::optimize_travel(
            &mut self.inner,
            allow_flip,
            preserve_first,
            preserve_order,
            &py_callbacks,
        );
        Ok(())
    }

    /// Apply a batch of transformers in a single call.
    ///
    /// The *transformers* list may contain any of the typed spec
    /// objects defined in :mod:`raygeo.ops.transform` (``SmoothSpec``,
    /// ``OptimizeSpec``, ``MergeLinesSpec``, ``OverscanSpec``,
    /// ``LeadInOutSpec``, ``MultiPassSpec``, ``CropSpec``,
    /// ``TabsSpec``, ``BidirScanOffsetSpec``). The transformers are
    /// sorted by execution phase (geometry refinement -> path
    /// interruption -> post-processing) and applied in order.
    ///
    /// :param transformers: List of typed spec objects.
    /// :param progress_cb: Optional callable ``(progress, message)``
    ///     that also exposes an ``is_cancelled()`` method; called
    ///     before each transformer. If ``is_cancelled()`` returns
    ///     ``True`` the loop aborts before the next transformer.
    /// :raises TypeError: If any element is not a known spec type.
    /// :raises RuntimeError: If the loop was cancelled.
    #[pyo3(signature = (transformers, progress_cb=None))]
    fn apply_transformers(
        &mut self,
        transformers: Vec<Bound<'_, PyAny>>,
        progress_cb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let mut specs: Vec<Box<dyn crate::ops::transform::Transformer>> =
            Vec::with_capacity(transformers.len());
        for ob in transformers.iter() {
            specs.push(extract_transformer(ob)?);
        }

        let py_callbacks =
            PyCallableCallbacks::new(progress_cb.map(|b| b.clone().unbind()));

        crate::ops::transform::apply_transformers(
            &mut self.inner,
            &mut specs,
            &py_callbacks,
        )
        .map_err(|_| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "apply_transformers cancelled",
            )
        })?;
        Ok(())
    }

    /// Compile this Ops into GPU-ready 3D scene data.
    ///
    /// :param world_to_visual: 4x4 transform matrix as a list of lists.
    /// :param layer_configs: Dict mapping layer UID to
    ///     ``{"rotary_enabled": bool, "rotary_diameter": float,
    ///       "axis_position": float, "reverse": bool}``.
    /// :returns: A :class:`CompiledScene3D` containing vertex groups,
    ///     layer infos, and laser UID order.
    fn compile_scene_3d<'py>(
        &self,
        py: Python<'py>,
        world_to_visual: &Bound<'py, PyAny>,
        layer_configs: &Bound<'py, PyDict>,
    ) -> PyResult<Py<PyCompiledScene3D>> {
        use crate::ops::convert::scene::{LayerConfig, SceneSpec};

        let mut w2v = [[0.0f32; 4]; 4];
        let rows: Vec<Vec<f32>> = world_to_visual.extract()?;
        for (i, row) in rows.iter().enumerate().take(4) {
            for (j, &val) in row.iter().enumerate().take(4) {
                w2v[i][j] = val;
            }
        }

        let mut configs = std::collections::HashMap::new();
        for (key, val) in layer_configs.iter() {
            let key_str: String = key.extract()?;
            let d = val.cast::<pyo3::types::PyDict>()?;
            let cfg = LayerConfig {
                rotary_enabled: d
                    .get_item("rotary_enabled")?
                    .and_then(|v| v.extract().ok())
                    .unwrap_or(false),
                rotary_diameter: d
                    .get_item("rotary_diameter")?
                    .and_then(|v| v.extract().ok())
                    .unwrap_or(0.0),
                axis_position: d
                    .get_item("axis_position")?
                    .and_then(|v| v.extract().ok())
                    .unwrap_or(0.0),
                reverse: d
                    .get_item("reverse")?
                    .and_then(|v| v.extract().ok())
                    .unwrap_or(false),
            };
            configs.insert(key_str, cfg);
        }

        let spec = SceneSpec {
            world_to_visual: w2v,
            layer_configs: configs,
        };

        let data = self.inner.compile_scene_3d(&spec);
        py_scene_data_to_object(py, data)
    }

    /// Compute the 2D bounding box of all ScanLine commands.
    ///
    /// Returns ``None`` if there are no scanlines. Otherwise returns
    /// ``(min_x, min_y, width, height)`` using visual endpoints.
    fn scanline_bbox(&self) -> Option<(f64, f64, f64, f64)> {
        self.inner.scanline_bbox()
    }

    /// Bake visual positions into a new Ops.
    ///
    /// For every moving command, replaces Y with the rotary degrees
    /// value from extra_axes. Non-moving commands are copied as-is.
    fn bake_visual_positions(&self, py: Python<'_>) -> PyResult<Py<PyOps>> {
        let baked = self.inner.bake_visual_positions();
        Py::new(py, PyOps { inner: baked })
    }

    /// Extract commands `[start, end)` into a new Ops.
    fn extract_range(
        &self,
        py: Python<'_>,
        start: usize,
        end: usize,
    ) -> PyResult<Py<PyOps>> {
        let sub = self.inner.extract_range(start, end);
        Py::new(py, PyOps { inner: sub })
    }

    /// Encode this Ops sequence into G-code text.
    ///
    /// Takes a typed dialect specification and an encoding context as a
    /// plain dict (JSON-serialisable). Returns a dict with the G-code
    /// text and bidirectional op-to-line index maps.
    ///
    /// :param dialect: A :class:`raygeo.ops.convert.GcodeDialectSpec` instance.
    /// :param context_dict: JSON-serialisable dict matching the Rust
    ///     ``EncodeContext`` schema.
    /// :returns: ``{"text": str, "op_to_machine_code": {int: [int]},
    ///     "machine_code_to_op": {int: int}}``
    /// :raises ValueError: If deserialization fails.
    fn to_gcode<'py>(
        &self,
        py: Python<'py>,
        dialect: &super::convert::PyGcodeDialectSpec,
        context_dict: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyDict>> {
        use crate::ops::convert::gcode::encode_gcode as encode_gcode_inner;
        use crate::ops::convert::gcode_types::EncodeContext;

        let context: EncodeContext = from_pydict(py, context_dict)?;
        let ops_clone = self.inner.clone();
        let dialect_clone = dialect.0.clone();
        let result = py
            .detach(move || {
                encode_gcode_inner(
                    &ops_clone,
                    &dialect_clone,
                    &context,
                    &crate::ops::callbacks::NoCallbacks,
                )
            })
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        let dict = PyDict::new(py);
        dict.set_item("text", &result.text)?;
        dict.set_item(
            "op_to_machine_code",
            usize_hashmap_to_pydict(py, &result.op_to_machine_code)?,
        )?;
        dict.set_item(
            "machine_code_to_op",
            usize_usize_hashmap_to_pydict(py, &result.machine_code_to_op)?,
        )?;
        Ok(dict)
    }

    /// Rasterise a grayscale image with power-modulated scans.
    ///
    /// Samples the image along scan lines and computes per-pixel power
    /// values from the grayscale intensity and alpha channel, then
    /// emits move-to/scan-to commands with the modulated power.
    ///
    /// :param gray_image: 2-D grayscale image (0 = black, 255 = white).
    /// :param alpha: 2-D alpha mask (0 = transparent/no emission).
    /// :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
    /// :param offset_x_mm: Global X offset in mm.
    /// :param offset_y_mm: Global Y offset in mm.
    /// :param line_interval_mm: Spacing between scan lines in mm.
    /// :param sample_interval_mm: Output sample spacing in mm.
    /// :param min_power: Minimum power fraction (for white pixels).
    /// :param max_power: Maximum power fraction (for black pixels).
    /// :param step_power: Global power multiplier.
    /// :param num_power_levels: Number of quantised power levels.
    /// :param angle: Scan angle in degrees.
    /// :param scan_mode: ``ScanMode.SEGMENTED`` or ``ScanMode.FULL_SWEEP``.
    /// :param dot_width_correction_mm: Shortens laser firing by this
    ///     distance at each end of every engraved run, compensating
    ///     for the laser spot's physical width. Geometry is unaffected.
    /// :returns: A new :class:`Ops` container.
    /// :complexity: O(h * w + n * p) where h, w = image dimensions, n = scan lines, p = pixels per line
    #[staticmethod]
    #[pyo3(signature = (gray_image, alpha, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, sample_interval_mm, min_power=0.0, max_power=1.0, step_power=1.0, num_power_levels=256, angle=0.0, scan_mode=PyScanMode::Segmented, dot_width_correction_mm=0.0))]
    #[allow(clippy::too_many_arguments)]
    fn from_power_modulated_image(
        py: Python<'_>,
        gray_image: &Bound<'_, PyAny>,
        alpha: &Bound<'_, PyAny>,
        pixels_per_mm: (f64, f64),
        offset_x_mm: f64,
        offset_y_mm: f64,
        line_interval_mm: f64,
        sample_interval_mm: f64,
        min_power: f64,
        max_power: f64,
        step_power: f64,
        num_power_levels: usize,
        angle: f64,
        scan_mode: PyScanMode,
        dot_width_correction_mm: f64,
    ) -> PyResult<Self> {
        let (gray, h, w) = extract_flat_u8(py, gray_image)?;
        let (alp, h2, w2) = extract_flat_u8(py, alpha)?;
        debug_assert_eq!(h, h2);
        debug_assert_eq!(w, w2);
        let ops = crate::ops::Ops::from_power_modulated_image(
            &gray,
            &alp,
            h,
            w,
            pixels_per_mm,
            offset_x_mm,
            offset_y_mm,
            line_interval_mm,
            sample_interval_mm,
            min_power,
            max_power,
            step_power,
            num_power_levels,
            angle,
            scan_mode.into(),
            dot_width_correction_mm,
        );
        Ok(PyOps { inner: ops })
    }

    /// Rasterise a binary mask into scan-to commands.
    ///
    /// Generates scan lines covering the mask's bounding box, samples
    /// the mask along each line, and emits move-to/scan-to commands
    /// for each non-zero segment (or the full sweep).
    ///
    /// :param mask: 2-D binary mask array.
    /// :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
    /// :param offset_x_mm: Global X offset in mm.
    /// :param offset_y_mm: Global Y offset in mm.
    /// :param line_interval_mm: Spacing between scan lines in mm.
    /// :param step_power: Power value (0-1) for exposed pixels.
    /// :param angle: Scan angle in degrees.
    /// :param scan_mode: ``ScanMode.SEGMENTED`` or ``ScanMode.FULL_SWEEP``.
    /// :param dot_width_correction_mm: Shortens laser firing by this
    ///     distance at each end of every engraved run, compensating
    ///     for the laser spot's physical width. Geometry is unaffected.
    /// :returns: A new :class:`Ops` container.
    /// :complexity: O(h * w + n * p) where h, w = image dimensions, n = scan lines, p = pixels per line
    #[staticmethod]
    #[pyo3(signature = (mask, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, step_power=1.0, angle=0.0, scan_mode=PyScanMode::Segmented, dot_width_correction_mm=0.0))]
    #[allow(clippy::too_many_arguments)]
    fn from_mask_scan(
        py: Python<'_>,
        mask: &Bound<'_, PyAny>,
        pixels_per_mm: (f64, f64),
        offset_x_mm: f64,
        offset_y_mm: f64,
        line_interval_mm: f64,
        step_power: f64,
        angle: f64,
        scan_mode: PyScanMode,
        dot_width_correction_mm: f64,
    ) -> PyResult<Self> {
        let (m, h, w) = extract_flat_u8(py, mask)?;
        let ops = crate::ops::Ops::from_mask_scan(
            &m,
            h,
            w,
            pixels_per_mm,
            offset_x_mm,
            offset_y_mm,
            line_interval_mm,
            step_power,
            angle,
            scan_mode.into(),
            dot_width_correction_mm,
        );
        Ok(PyOps { inner: ops })
    }

    /// Rasterise a binary mask into line-to commands (no power).
    ///
    /// Similar to :meth:`from_mask_scan` but emits move-to/line-to
    /// commands with a Z offset instead of scan-to with power values.
    /// Useful for simple contour or hatch patterns.
    ///
    /// :param mask: 2-D binary mask array.
    /// :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
    /// :param offset_x_mm: Global X offset in mm.
    /// :param offset_y_mm: Global Y offset in mm.
    /// :param line_interval_mm: Spacing between scan lines in mm.
    /// :param z: Z offset for the lines in mm.
    /// :param angle: Scan angle in degrees.
    /// :param scan_mode: ``ScanMode.SEGMENTED`` or ``ScanMode.FULL_SWEEP``.
    /// :returns: A new :class:`Ops` container.
    /// :complexity: O(h * w + n * p) where h, w = image dimensions, n = scan lines, p = pixels per line
    #[staticmethod]
    #[pyo3(signature = (mask, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, z=0.0, angle=0.0, scan_mode=PyScanMode::Segmented))]
    #[allow(clippy::too_many_arguments)]
    fn from_mask_lines(
        py: Python<'_>,
        mask: &Bound<'_, PyAny>,
        pixels_per_mm: (f64, f64),
        offset_x_mm: f64,
        offset_y_mm: f64,
        line_interval_mm: f64,
        z: f64,
        angle: f64,
        scan_mode: PyScanMode,
    ) -> PyResult<Self> {
        let (m, h, w) = extract_flat_u8(py, mask)?;
        let ops = crate::ops::Ops::from_mask_lines(
            &m,
            h,
            w,
            pixels_per_mm,
            offset_x_mm,
            offset_y_mm,
            line_interval_mm,
            z,
            angle,
            scan_mode.into(),
        );
        Ok(PyOps { inner: ops })
    }

    /// Rasterise a grayscale image as multiple Z-depth passes.
    ///
    /// Decomposes the grayscale image into *num_depth_levels* layers
    /// by depth-slicing, then rasterises each layer with a progressive
    /// Z offset and optional per-pass angle increment.
    ///
    /// :param gray_image: 2-D grayscale image (0 = black, 255 = white).
    /// :param pixels_per_mm: ``(x, y)`` pixel density in px/mm.
    /// :param offset_x_mm: Global X offset in mm.
    /// :param offset_y_mm: Global Y offset in mm.
    /// :param line_interval_mm: Spacing between scan lines in mm.
    /// :param num_depth_levels: Number of depth layers to produce.
    /// :param z_step_down: Z decrement per depth layer in mm.
    /// :param angle: Initial scan angle in degrees.
    /// :param angle_increment: Angle added per depth layer in degrees.
    /// :param scan_mode: ``ScanMode.SEGMENTED`` or ``ScanMode.FULL_SWEEP``.
    /// :returns: A new :class:`Ops` container.
    /// :complexity: O(d * (h * w + n * p)) where d = depth levels, h, w = image dims, n = scan lines, p = pixels per line
    #[staticmethod]
    #[pyo3(signature = (gray_image, pixels_per_mm, offset_x_mm, offset_y_mm, line_interval_mm, num_depth_levels, z_step_down, angle=0.0, angle_increment=0.0, scan_mode=PyScanMode::Segmented))]
    #[allow(clippy::too_many_arguments)]
    fn from_multi_pass_image(
        py: Python<'_>,
        gray_image: &Bound<'_, PyAny>,
        pixels_per_mm: (f64, f64),
        offset_x_mm: f64,
        offset_y_mm: f64,
        line_interval_mm: f64,
        num_depth_levels: usize,
        z_step_down: f64,
        angle: f64,
        angle_increment: f64,
        scan_mode: PyScanMode,
    ) -> PyResult<Self> {
        let (gray, h, w) = extract_flat_u8(py, gray_image)?;
        let ops = crate::ops::Ops::from_multi_pass_image(
            &gray,
            h,
            w,
            pixels_per_mm,
            offset_x_mm,
            offset_y_mm,
            line_interval_mm,
            num_depth_levels,
            z_step_down,
            angle,
            angle_increment,
            scan_mode.into(),
        );
        Ok(PyOps { inner: ops })
    }
}

fn extract_flat_u8(
    py: Python<'_>,
    arr: &Bound<'_, PyAny>,
) -> PyResult<(Vec<u8>, usize, usize)> {
    let numpy = py.import("numpy")?;
    let a = numpy.call_method1("asarray", (arr,))?;
    let shape: (usize, usize) = a.getattr("shape")?.extract()?;
    let flat: Vec<u8> = a
        .call_method0("flatten")?
        .call_method0("tolist")?
        .extract()?;
    Ok((flat, shape.0, shape.1))
}
