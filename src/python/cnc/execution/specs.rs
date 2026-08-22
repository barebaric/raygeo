pyo3_stub_gen::module_doc!("raygeo.cnc.execution.specs", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "CNC execution spec types.";

use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

use crate::cnc::execution::specs::AggregateOutput as CoreAggregateOutput;
use crate::ops::state::AirAssistMode;
use crate::python::ops::container::PyOps;
use crate::python::ops::convert::PyEncoder;
use crate::python::ops::state::PyAirAssistMode;

// ═══════════════════════════════════════════════════════════════════
// AggregateOutput  (result of the Aggregate stage)
// ═══════════════════════════════════════════════════════════════════

/// Result of the ``Aggregate`` stage.
///
/// Produced when the pipeline executes an ``AggregateSpec`` and contains
/// the concatenated/transformed Ops together with an optional time
/// estimate.
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "AggregateOutput",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyAggregateOutput {
    /// The concatenated Ops from all inputs in the aggregate.
    #[pyo3(get)]
    pub ops: Py<PyOps>,
    /// Estimated machining time in seconds, or ``None`` when the
    /// ``MachineParams`` rates are all zero.
    #[pyo3(get)]
    pub time_estimate: Option<f64>,
}

impl PyAggregateOutput {
    pub fn from_core(core: CoreAggregateOutput, py: Python<'_>) -> Self {
        PyAggregateOutput {
            ops: Py::new(py, PyOps { inner: core.ops }).unwrap(),
            time_estimate: core.time_estimate,
        }
    }

    pub fn from_arc(
        arc: std::sync::Arc<dyn std::any::Any + Send + Sync>,
        py: Python<'_>,
    ) -> Self {
        let agg = arc
            .downcast_ref::<CoreAggregateOutput>()
            .expect("PyAggregateOutput holds non-AggregateOutput");
        Self::from_core(agg.clone(), py)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAggregateOutput {
    /// Number of commands in the aggregated Ops.
    #[getter]
    fn ops_len(&self, py: Python<'_>) -> usize {
        self.ops.bind(py).borrow().inner.len()
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "AggregateOutput(ops_len={}, time_estimate={:?})",
            self.ops.bind(py).borrow().inner.len(),
            self.time_estimate,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Marker  (declarative markers for aggregate groups)
// ═══════════════════════════════════════════════════════════════════

/// Declarative markers for aggregate groups.
///
/// Each variant has a ``_tag`` field (always ``True``) required by the
/// stub generator; pass it as keyword argument:
///
///   Marker.JobStart(_tag=True)
///   Marker.LayerStart(uid="my-layer", _tag=True)
///   Marker.WorkpieceEnd(uid="my-wp", _tag=True)
#[gen_stub_pyclass_complex_enum]
#[pyclass(
    name = "Marker",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyMarker {
    /// Marks the beginning of a job.
    JobStart { _tag: bool },
    /// Marks the end of a job.
    JobEnd { _tag: bool },
    /// Marks the start of a layer with the given UID.
    LayerStart { uid: String, _tag: bool },
    /// Marks the end of a layer with the given UID.
    LayerEnd { uid: String, _tag: bool },
    /// Marks the start of a workpiece with the given UID.
    WorkpieceStart { uid: String, _tag: bool },
    /// Marks the end of a workpiece with the given UID.
    WorkpieceEnd { uid: String, _tag: bool },
}

impl PyMarker {
    pub fn to_core(&self) -> crate::cnc::execution::specs::Marker {
        match self {
            PyMarker::JobStart { .. } => {
                crate::cnc::execution::specs::Marker::JobStart
            }
            PyMarker::JobEnd { .. } => {
                crate::cnc::execution::specs::Marker::JobEnd
            }
            PyMarker::LayerStart { uid, .. } => {
                crate::cnc::execution::specs::Marker::LayerStart {
                    uid: uid.clone(),
                }
            }
            PyMarker::LayerEnd { uid, .. } => {
                crate::cnc::execution::specs::Marker::LayerEnd {
                    uid: uid.clone(),
                }
            }
            PyMarker::WorkpieceStart { uid, .. } => {
                crate::cnc::execution::specs::Marker::WorkpieceStart {
                    uid: uid.clone(),
                }
            }
            PyMarker::WorkpieceEnd { uid, .. } => {
                crate::cnc::execution::specs::Marker::WorkpieceEnd {
                    uid: uid.clone(),
                }
            }
        }
    }
}

impl From<crate::cnc::execution::specs::Marker> for PyMarker {
    fn from(m: crate::cnc::execution::specs::Marker) -> Self {
        match m {
            crate::cnc::execution::specs::Marker::JobStart => {
                PyMarker::JobStart { _tag: true }
            }
            crate::cnc::execution::specs::Marker::JobEnd => {
                PyMarker::JobEnd { _tag: true }
            }
            crate::cnc::execution::specs::Marker::LayerStart { uid } => {
                PyMarker::LayerStart { uid, _tag: true }
            }
            crate::cnc::execution::specs::Marker::LayerEnd { uid } => {
                PyMarker::LayerEnd { uid, _tag: true }
            }
            crate::cnc::execution::specs::Marker::WorkpieceStart { uid } => {
                PyMarker::WorkpieceStart { uid, _tag: true }
            }
            crate::cnc::execution::specs::Marker::WorkpieceEnd { uid } => {
                PyMarker::WorkpieceEnd { uid, _tag: true }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// MachineParams  (feed/speed rates for time estimation)
// ═══════════════════════════════════════════════════════════════════

/// Feed/speed rates used for time estimation.
///
/// All three fields default to ``0.0``, which disables the time
/// estimate (returning ``None``).
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "MachineParams",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy)]
pub struct PyMachineParams {
    /// Default feed rate (mm/min) for cutting moves.
    #[pyo3(get)]
    pub default_feed_rate: f64,
    /// Default rapid rate (mm/min) for travel moves.
    #[pyo3(get)]
    pub default_rapid_rate: f64,
    /// Acceleration (mm/s²) for time estimation.
    #[pyo3(get)]
    pub acceleration: f64,
}

impl PyMachineParams {
    pub fn to_core(self) -> crate::cnc::execution::specs::MachineParams {
        crate::cnc::execution::specs::MachineParams {
            default_feed_rate: self.default_feed_rate,
            default_rapid_rate: self.default_rapid_rate,
            acceleration: self.acceleration,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyMachineParams {
    #[new]
    #[pyo3(signature = (default_feed_rate = 0.0, default_rapid_rate = 0.0, acceleration = 0.0))]
    fn new(
        default_feed_rate: f64,
        default_rapid_rate: f64,
        acceleration: f64,
    ) -> Self {
        PyMachineParams {
            default_feed_rate,
            default_rapid_rate,
            acceleration,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// ComputePayload  (opaque parameter bundle for a Compute stage)
// ═══════════════════════════════════════════════════════════════════

/// Opaque parameter bundle for a ``Compute`` stage node.
///
/// The pipeline's ``StageSpec.Compute`` stores this as an opaque
/// ``Any`` — the CNC converter unpacks it during traversal.
///
/// :param assembler: The assembler spec that drives this compute.
/// :param transformers: Optional list of transformer specs.
/// :param state_source_keys: Keys of upstream nodes whose cleared-area
///     state should be threaded into this compute (CNC only).
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "ComputePayload",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyComputePayload {
    /// The assembler spec (e.g. ``ContourSpec``, ``AdaptiveClearingSpec``).
    #[pyo3(get)]
    pub assembler: Py<PyAny>,
    /// Optional list of transformer specs applied post-assembly.
    #[pyo3(get, set)]
    pub transformers: Vec<Py<PyAny>>,
    /// Source keys for cleared-area state threading (CNC only).
    #[pyo3(get, set)]
    pub state_source_keys: Vec<String>,
    /// Laser power fraction (0.0 – 1.0) injected as ``SetPower``.
    #[pyo3(get, set)]
    pub power: f64,
    /// Cut speed (mm/min) injected as ``SetFeedRate``.
    #[pyo3(get, set)]
    pub cut_speed: i32,
    /// Active head/laser UID injected as ``SetHead``.
    #[pyo3(get, set)]
    pub head_uid: Option<String>,
    /// Air assist mode injected as ``SetAirAssist`` (default ``None``).
    pub air_assist: Option<AirAssistMode>,
    /// Print a profiling report to stdout after this node's faces have
    /// been assembled (default False).
    #[pyo3(get, set)]
    pub profile: bool,
    /// Laser emission wavelength in nm for the burn fluence model
    /// (default 0 → renderer falls back to full absorption).
    #[pyo3(get, set)]
    pub wavelength_nm: f64,
    /// Optical output power in watts at full power for the burn
    /// fluence model (default 0 → neutral fallback).
    #[pyo3(get, set)]
    pub max_power_watts: f64,
    /// Beam spot size ``(x, y)`` in mm for the burn fluence model
    /// (default `(0.1, 0.1)`).
    #[pyo3(get, set)]
    pub spot_size_mm: (f64, f64),
    /// Scan speed in mm/s for the burn fluence model (default 100).
    #[pyo3(get, set)]
    pub scan_speed_mm_per_s: f64,
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyComputePayload {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (assembler, transformers=vec![], state_source_keys=vec![], power=0.0, cut_speed=0, head_uid=None, air_assist=None, profile=false, wavelength_nm=0.0, max_power_watts=0.0, spot_size_mm=(0.1, 0.1), scan_speed_mm_per_s=100.0))]
    fn new(
        assembler: Py<PyAny>,
        transformers: Vec<Py<PyAny>>,
        state_source_keys: Vec<String>,
        power: f64,
        cut_speed: i32,
        head_uid: Option<String>,
        air_assist: Option<Bound<'_, PyAirAssistMode>>,
        profile: bool,
        wavelength_nm: f64,
        max_power_watts: f64,
        spot_size_mm: (f64, f64),
        scan_speed_mm_per_s: f64,
    ) -> Self {
        PyComputePayload {
            assembler,
            transformers,
            state_source_keys,
            power,
            cut_speed,
            head_uid,
            air_assist: air_assist.map(|a| a.borrow().0),
            profile,
            wavelength_nm,
            max_power_watts,
            spot_size_mm,
            scan_speed_mm_per_s,
        }
    }

    /// Air assist mode injected as ``SetAirAssist``.
    #[getter]
    fn air_assist(&self) -> Option<PyAirAssistMode> {
        self.air_assist.map(PyAirAssistMode)
    }

    #[setter]
    fn set_air_assist(&mut self, value: Option<Bound<'_, PyAirAssistMode>>) {
        self.air_assist = value.map(|a| a.borrow().0);
    }
}

// ═══════════════════════════════════════════════════════════════════
// AggregateSpec  (configuration for the Aggregate stage)
// ═══════════════════════════════════════════════════════════════════

/// Configuration for the ``Aggregate`` stage.
///
/// An aggregate concatenates Ops from multiple upstream nodes, wraps
/// them in markers (job/layer/workpiece start/end), applies per-input
/// placement and scaling, optionally links consecutive inputs with
/// travel moves, and applies batch transformers.
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "AggregateSpec",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyAggregateSpec {
    /// Markers emitted before all groups (e.g. ``JobStart``).
    #[pyo3(get)]
    pub wrap_start: Vec<PyMarker>,
    /// Groups of inputs, each processed in order.
    #[pyo3(get)]
    pub groups: Vec<PyAggregateGroup>,
    /// Markers emitted after all groups (e.g. ``JobEnd``).
    #[pyo3(get)]
    pub wrap_end: Vec<PyMarker>,
    /// Machine parameters for time estimation.
    #[pyo3(get)]
    pub machine: PyMachineParams,
    /// Batch transformers applied to the aggregated Ops.
    #[pyo3(get, set)]
    pub transformers: Vec<Py<PyAny>>,
}

impl PyAggregateSpec {
    pub fn to_core(
        &self,
        py: Python<'_>,
    ) -> PyResult<crate::cnc::execution::specs::AggregateSpec> {
        let transformers = self
            .transformers
            .iter()
            .map(|t| {
                crate::python::ops::transform::extract_transformer(
                    t.bind(py),
                )
            })
            .collect::<PyResult<Vec<Box<dyn crate::ops::transform::Transformer>>>>()?;
        Ok(crate::cnc::execution::specs::AggregateSpec {
            wrap_start: self
                .wrap_start
                .iter()
                .map(|m| m.clone().to_core())
                .collect(),
            groups: self.groups.iter().map(|g| g.to_core()).collect(),
            wrap_end: self
                .wrap_end
                .iter()
                .map(|m| m.clone().to_core())
                .collect(),
            machine: self.machine.to_core(),
            transformers,
        })
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyAggregateSpec {
    #[new]
    #[pyo3(signature = (wrap_start, groups, wrap_end, machine, transformers=vec![]))]
    fn new(
        py: Python<'_>,
        wrap_start: Vec<Py<PyMarker>>,
        groups: Vec<Py<PyAggregateGroup>>,
        wrap_end: Vec<Py<PyMarker>>,
        machine: Py<PyMachineParams>,
        transformers: Vec<Py<PyAny>>,
    ) -> Self {
        PyAggregateSpec {
            wrap_start: wrap_start
                .iter()
                .map(|m| m.borrow(py).clone())
                .collect(),
            groups: groups.iter().map(|g| g.borrow(py).clone()).collect(),
            wrap_end: wrap_end.iter().map(|m| m.borrow(py).clone()).collect(),
            machine: *machine.borrow(py),
            transformers,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// LinkMode  (inter-input linking control)
// ═══════════════════════════════════════════════════════════════════

/// Controls inter-input linking in an AggregateGroup.
///
/// Create via class methods:
///   LinkMode.none()
///   LinkMode.sequential(safe_z=2.0)
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "LinkMode",
    module = "raygeo.cnc.execution.specs",
    frozen,
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq)]
pub struct PyLinkMode {
    /// Discriminant: ``"none"`` or ``"sequential"``.
    #[pyo3(get)]
    tag: String,
    /// Safe Z height for retract/lift moves (only meaningful when
    /// ``tag == "sequential"``).
    #[pyo3(get)]
    safe_z: f64,
}

impl PyLinkMode {
    pub fn to_core(&self) -> crate::cnc::execution::specs::LinkMode {
        match self.tag.as_str() {
            "sequential" => {
                crate::cnc::execution::specs::LinkMode::Sequential {
                    safe_z: self.safe_z,
                }
            }
            _ => crate::cnc::execution::specs::LinkMode::None,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyLinkMode {
    /// Create a ``LinkMode`` with no inter-input linking.
    #[classmethod]
    fn none(_cls: &Bound<'_, PyType>) -> Self {
        PyLinkMode {
            tag: "none".to_string(),
            safe_z: 0.0,
        }
    }

    /// Create a ``LinkMode`` that emits travel moves (retract →
    /// XY travel → plunge) between consecutive inputs.
    ///
    /// :param safe_z: Z height for retract and final lift.
    #[classmethod]
    fn sequential(_cls: &Bound<'_, PyType>, safe_z: f64) -> Self {
        PyLinkMode {
            tag: "sequential".to_string(),
            safe_z,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// AggregateGroup  (a group of inputs in an AggregateSpec)
// ═══════════════════════════════════════════════════════════════════

/// A group of inputs within an ``AggregateSpec``.
///
/// Each group emits its own start/end markers, processes its inputs
/// in order (optionally linking consecutive inputs when
/// ``link_mode`` is ``Sequential``), and supports per-input placement
/// and scaling.
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "AggregateGroup",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyAggregateGroup {
    /// Markers emitted before this group's inputs.
    #[pyo3(get)]
    pub start_markers: Vec<PyMarker>,
    /// Upstream inputs to concatenate (in order).
    #[pyo3(get)]
    pub inputs: Vec<PyAggregateInput>,
    /// Markers emitted after this group's inputs.
    #[pyo3(get)]
    pub end_markers: Vec<PyMarker>,
    /// Inter-input linking control (default ``LinkMode.none()``).
    #[pyo3(get)]
    pub link_mode: PyLinkMode,
}

impl PyAggregateGroup {
    pub fn to_core(&self) -> crate::cnc::execution::specs::AggregateGroup {
        crate::cnc::execution::specs::AggregateGroup {
            start_markers: self
                .start_markers
                .iter()
                .map(|m| m.clone().to_core())
                .collect(),
            inputs: self.inputs.iter().map(|i| i.to_core()).collect(),
            end_markers: self
                .end_markers
                .iter()
                .map(|m| m.clone().to_core())
                .collect(),
            link_mode: self.link_mode.to_core(),
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyAggregateGroup {
    #[new]
    #[pyo3(signature = (start_markers, inputs, end_markers, link_mode=None))]
    fn new(
        py: Python<'_>,
        start_markers: Vec<Py<PyMarker>>,
        inputs: Vec<Py<PyAggregateInput>>,
        end_markers: Vec<Py<PyMarker>>,
        link_mode: Option<Py<PyLinkMode>>,
    ) -> Self {
        PyAggregateGroup {
            start_markers: start_markers
                .iter()
                .map(|m| m.borrow(py).clone())
                .collect(),
            inputs: inputs.iter().map(|i| i.borrow(py).clone()).collect(),
            end_markers: end_markers
                .iter()
                .map(|m| m.borrow(py).clone())
                .collect(),
            link_mode: link_mode
                .map(|lm| lm.borrow(py).clone())
                .unwrap_or_else(|| PyLinkMode {
                    tag: "none".to_string(),
                    safe_z: 0.0,
                }),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// AggregateInput  (a single source in an AggregateGroup)
// ═══════════════════════════════════════════════════════════════════

/// A single source input in an ``AggregateGroup``.
///
/// Identifies an upstream node by key and optionally applies a
/// placement matrix and uniform scaling.
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "AggregateInput",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyAggregateInput {
    /// Key of the upstream node whose Ops to consume.
    #[pyo3(get)]
    pub source_key: String,
    /// 4×4 placement matrix (row-major) applied to all points in
    /// the source Ops.
    #[pyo3(get)]
    pub placement_matrix: [[f64; 4]; 4],
    /// Optional UID carried through for marker correlation (not
    /// used by the pipeline core).
    #[pyo3(get)]
    pub uid: String,
    /// Target dimensions for uniform scaling ``(width, height)``.
    /// When ``(0, 0)`` (default), no scaling is applied.
    #[pyo3(get)]
    pub target_dimensions: (f64, f64),
}

impl PyAggregateInput {
    pub fn to_core(&self) -> crate::cnc::execution::specs::AggregateInput {
        crate::cnc::execution::specs::AggregateInput {
            source_key: self.source_key.clone(),
            placement_matrix: self.placement_matrix,
            uid: self.uid.clone(),
            target_dimensions: self.target_dimensions,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyAggregateInput {
    #[new]
    #[pyo3(signature = (source_key, placement_matrix, uid="", target_dimensions=(0.0, 0.0)))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        source_key: &str,
        placement_matrix: [[f64; 4]; 4],
        uid: &str,
        target_dimensions: (f64, f64),
    ) -> Self {
        PyAggregateInput {
            source_key: source_key.to_string(),
            placement_matrix,
            uid: uid.to_string(),
            target_dimensions,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// EncodeSpec  (opaque compute stage that wraps an encoder)
// ═══════════════════════════════════════════════════════════════════

/// An encode-only compute stage.
///
/// The pipeline's generic ``StageSpec`` has no ``Encode`` variant —
/// encoding is a CNC concern. Use ``EncodeSpec`` when you need to
/// run an encoder through the pipeline.
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "EncodeSpec",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyEncodeSpec {
    /// Key of the upstream node whose Ops to encode.
    #[pyo3(get)]
    pub source_key: String,
    /// The encoder to use (e.g. ``GcodeSpec``, ``VertexSpec``).
    #[pyo3(get)]
    pub encoder: Py<PyEncoder>,
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyEncodeSpec {
    #[new]
    fn new(source_key: String, encoder: Py<PyEncoder>) -> Self {
        PyEncodeSpec {
            source_key,
            encoder,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// MachineTransformSpec  (machine coordinate transform stage)
// ═══════════════════════════════════════════════════════════════════

/// Configuration for the machine-transform pipeline stage.
///
/// Converts world-space Ops into machine-space Ops by applying
/// curve linearization, per-layer rotary axis mapping, world→machine
/// coordinate transforms, WCS offsets, Z-flip, and AXIS_REPLACEMENT
/// downstream conversion.
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "MachineTransformSpec",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyMachineTransformSpec {
    /// Key of the upstream node whose Ops to transform.
    #[pyo3(get)]
    pub source_key: String,
    /// When true, linearize Bezier curves before other transforms.
    #[pyo3(get)]
    pub linearize_curves: bool,
    /// 4×4 world→machine matrix (row-major).
    #[pyo3(get)]
    pub world_to_machine: [[f64; 4]; 4],
    /// Default per-layer WCS command offset (x, y, z).
    #[pyo3(get)]
    pub default_wcs_offset: [f64; 3],
    /// Per-layer WCS offsets, keyed by layer UID.
    #[pyo3(get)]
    pub layer_wcs_offsets: Vec<(String, [f64; 3])>,
    /// When true, negate Z after transforms.
    #[pyo3(get)]
    pub reverse_z: bool,
    /// Per-layer rotary mapping configs.
    #[pyo3(get)]
    pub rotary_mappings: Vec<Py<PyRotaryMappingSpec>>,
}

impl PyMachineTransformSpec {
    pub fn to_core(
        &self,
        py: Python<'_>,
    ) -> crate::cnc::execution::specs::MachineTransformSpec {
        crate::cnc::execution::specs::MachineTransformSpec {
            source_key: self.source_key.clone(),
            linearize_curves: self.linearize_curves,
            world_to_machine: self.world_to_machine,
            default_wcs_offset: self.default_wcs_offset,
            layer_wcs_offsets: self.layer_wcs_offsets.clone(),
            reverse_z: self.reverse_z,
            rotary_mappings: self
                .rotary_mappings
                .iter()
                .map(|rm| rm.borrow(py).to_core())
                .collect(),
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyMachineTransformSpec {
    #[new]
    #[pyo3(signature = (
        source_key,
        linearize_curves,
        world_to_machine,
        default_wcs_offset,
        layer_wcs_offsets,
        reverse_z,
        rotary_mappings,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        _py: Python<'_>,
        source_key: String,
        linearize_curves: bool,
        world_to_machine: [[f64; 4]; 4],
        default_wcs_offset: [f64; 3],
        layer_wcs_offsets: Vec<(String, [f64; 3])>,
        reverse_z: bool,
        rotary_mappings: Vec<Py<PyRotaryMappingSpec>>,
    ) -> Self {
        PyMachineTransformSpec {
            source_key,
            linearize_curves,
            world_to_machine,
            default_wcs_offset,
            layer_wcs_offsets,
            reverse_z,
            rotary_mappings,
        }
    }
}

/// Per-layer rotary axis mapping configuration.
#[gen_stub_pyclass(module = "raygeo.cnc.execution.specs")]
#[pyclass(
    name = "RotaryMappingSpec",
    module = "raygeo.cnc.execution.specs",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyRotaryMappingSpec {
    /// UID of the layer this rotary config applies to.
    #[pyo3(get)]
    pub layer_uid: String,
    /// Rotary workpiece diameter (mm).
    #[pyo3(get)]
    pub diameter: f64,
    /// Gear ratio.
    #[pyo3(get)]
    pub gear_ratio: f64,
    /// When true, negate computed degree values.
    #[pyo3(get)]
    pub reverse: bool,
    /// Axis mount position (x, y, z).
    #[pyo3(get)]
    pub axis_position_3d: [f64; 3],
    /// Cylinder direction vector (x, y, z).
    #[pyo3(get)]
    pub cylinder_dir: [f64; 3],
    /// Rotary axis name (e.g. "A", "B", "C").
    #[pyo3(get)]
    pub rotary_axis: String,
    /// Replaced world axis name or None for TRUE_4TH_AXIS.
    #[pyo3(get)]
    pub replaced_axis: Option<String>,
    /// Millimetres per full rotation (0 = no conversion).
    #[pyo3(get)]
    pub mm_per_rotation: f64,
}

impl PyRotaryMappingSpec {
    pub fn to_core(&self) -> crate::cnc::execution::specs::RotaryMappingSpec {
        crate::cnc::execution::specs::RotaryMappingSpec {
            layer_uid: self.layer_uid.clone(),
            diameter: self.diameter,
            gear_ratio: self.gear_ratio,
            reverse: self.reverse,
            axis_position_3d: self.axis_position_3d,
            cylinder_dir: self.cylinder_dir,
            rotary_axis: self.rotary_axis.clone(),
            replaced_axis: self.replaced_axis.clone(),
            mm_per_rotation: self.mm_per_rotation,
        }
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyRotaryMappingSpec {
    #[new]
    #[pyo3(signature = (
        layer_uid,
        diameter,
        gear_ratio,
        reverse,
        axis_position_3d,
        cylinder_dir,
        rotary_axis,
        replaced_axis,
        mm_per_rotation,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        layer_uid: String,
        diameter: f64,
        gear_ratio: f64,
        reverse: bool,
        axis_position_3d: [f64; 3],
        cylinder_dir: [f64; 3],
        rotary_axis: String,
        replaced_axis: Option<String>,
        mm_per_rotation: f64,
    ) -> Self {
        PyRotaryMappingSpec {
            layer_uid,
            diameter,
            gear_ratio,
            reverse,
            axis_position_3d,
            cylinder_dir,
            rotary_axis,
            replaced_axis,
            mm_per_rotation,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Module registration
// ═══════════════════════════════════════════════════════════════════

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let specs_mod = PyModule::new(py, "specs")?;
    specs_mod.setattr("__doc__", MODULE_DOC)?;

    specs_mod.add_class::<PyAggregateOutput>()?;
    specs_mod.add_class::<PyComputePayload>()?;
    specs_mod.add_class::<PyMarker>()?;
    specs_mod.add_class::<PyMachineParams>()?;
    specs_mod.add_class::<PyAggregateSpec>()?;
    specs_mod.add_class::<PyLinkMode>()?;
    specs_mod.add_class::<PyAggregateGroup>()?;
    specs_mod.add_class::<PyAggregateInput>()?;
    specs_mod.add_class::<PyEncodeSpec>()?;
    specs_mod.add_class::<PyMachineTransformSpec>()?;
    specs_mod.add_class::<PyRotaryMappingSpec>()?;

    m.add_submodule(&specs_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.cnc.execution.specs", &specs_mod)?;

    Ok(())
}
