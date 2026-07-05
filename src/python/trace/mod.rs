//! PyO3 bindings for trace-file reading and the `MoveKind` enum.
//!
//! Exposes `MoveKind`, `TraceFile`, and per-record dict access to Python.

use std::path::Path;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::IntoPyObjectExt;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::trace_types::{MoveKind as RMoveKind, TraceFileData};

// ── Python module registration ─────────────────────────────────────

pyo3_stub_gen::module_doc!("raygeo.trace", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Binary trace-file reader and shared move-type classification.

MoveKind — standard move-type classification shared by all operations.
TraceFile — read a .bin trace file with random access to records.
TraceRecord — one per-step record with dot-accessible fields.
";

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let trace_mod = PyModule::new(py, "trace")?;
    trace_mod.setattr("__doc__", MODULE_DOC)?;
    trace_mod.add_class::<PyMoveKind>()?;
    trace_mod.add_class::<PyTraceFile>()?;
    trace_mod.add_class::<PyTraceRecord>()?;

    trace_mod.add_class::<PyTraceKind>()?;
    trace_mod.add_class::<PyStepStatus>()?;
    trace_mod.add_class::<PyResumeSource>()?;
    trace_mod.add_class::<PyRouteSource>()?;
    trace_mod
        .add_function(wrap_pyfunction!(get_route_detail_name, &trace_mod)?)?;

    m.add_submodule(&trace_mod)?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.trace", &trace_mod)?;
    Ok(())
}

// ── MoveKind ───────────────────────────────────────────────────────

/// Standard move-type classification shared by all operations.
///
/// Every toolpath point is tagged with one of these so that renderers
/// can colour and categorise moves generically.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    skip_from_py_object,
    module = "raygeo.trace",
    name = "MoveKind"
)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyMoveKind(pub(crate) RMoveKind);

#[gen_stub_pymethods]
#[pymethods]
impl PyMoveKind {
    #[classattr]
    pub const CUT: PyMoveKind = PyMoveKind(RMoveKind::Cut);
    #[classattr]
    pub const TRAVEL: PyMoveKind = PyMoveKind(RMoveKind::Travel);
    #[classattr]
    pub const PLUNGE: PyMoveKind = PyMoveKind(RMoveKind::Plunge);
    #[classattr]
    pub const LEAD_IN: PyMoveKind = PyMoveKind(RMoveKind::LeadIn);
    #[classattr]
    pub const LEAD_OUT: PyMoveKind = PyMoveKind(RMoveKind::LeadOut);
    #[classattr]
    pub const LINK: PyMoveKind = PyMoveKind(RMoveKind::Link);
    #[classattr]
    pub const RESUME: PyMoveKind = PyMoveKind(RMoveKind::Resume);
    #[classattr]
    pub const ROUTE: PyMoveKind = PyMoveKind(RMoveKind::Route);

    fn __repr__(&self) -> &'static str {
        match self.0 {
            RMoveKind::Cut => "MoveKind.CUT",
            RMoveKind::Travel => "MoveKind.TRAVEL",
            RMoveKind::Plunge => "MoveKind.PLUNGE",
            RMoveKind::LeadIn => "MoveKind.LEAD_IN",
            RMoveKind::LeadOut => "MoveKind.LEAD_OUT",
            RMoveKind::Link => "MoveKind.LINK",
            RMoveKind::Resume => "MoveKind.RESUME",
            RMoveKind::Route => "MoveKind.ROUTE",
        }
    }

    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }

    #[getter]
    fn name(&self) -> &'static str {
        match self.0 {
            RMoveKind::Cut => "CUT",
            RMoveKind::Travel => "TRAVEL",
            RMoveKind::Plunge => "PLUNGE",
            RMoveKind::LeadIn => "LEAD_IN",
            RMoveKind::LeadOut => "LEAD_OUT",
            RMoveKind::Link => "LINK",
            RMoveKind::Resume => "RESUME",
            RMoveKind::Route => "ROUTE",
        }
    }
}

// ── Import canonical Rust enums for PyO3 wrappers ────────────────

use crate::ops::assembly::adaptive::resume::ResumeSource as RResumeSource;
use crate::ops::assembly::adaptive::routing::RouteSource as RRouteSource;
use crate::ops::cut::stepper::StepStatus as RStepStatus;
use crate::trace_types::TraceKind as RTraceKind;

/// Record-kind enum for trace events.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    skip_from_py_object,
    module = "raygeo.trace",
    name = "TraceKind"
)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyTraceKind(pub(crate) RTraceKind);

#[gen_stub_pymethods]
#[pymethods]
impl PyTraceKind {
    #[classattr]
    pub const INIT: PyTraceKind = PyTraceKind(RTraceKind::Init);
    #[classattr]
    pub const CUT: PyTraceKind = PyTraceKind(RTraceKind::Cut);
    #[classattr]
    pub const RESUME_STALL: PyTraceKind = PyTraceKind(RTraceKind::ResumeStall);
    #[classattr]
    pub const RESUME_STUCK: PyTraceKind = PyTraceKind(RTraceKind::ResumeStuck);
    #[classattr]
    pub const EXIT: PyTraceKind = PyTraceKind(RTraceKind::Exit);

    fn __repr__(&self) -> &'static str {
        self.name()
    }
    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }
    #[getter]
    fn name(&self) -> &'static str {
        match self.0 {
            RTraceKind::Init => "init",
            RTraceKind::Cut => "cut",
            RTraceKind::ResumeStall => "resume_stall",
            RTraceKind::ResumeStuck => "resume_stuck",
            RTraceKind::Exit => "exit",
        }
    }
}

// ── StepStatus (wraps ops/cut/stepper.rs StepStatus) ──────────────

/// Step-status enum for trace records.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    skip_from_py_object,
    module = "raygeo.trace",
    name = "StepStatus"
)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyStepStatus(pub(crate) RStepStatus);

#[gen_stub_pymethods]
#[pymethods]
impl PyStepStatus {
    #[classattr]
    pub const OK: PyStepStatus = PyStepStatus(RStepStatus::Ok);
    #[classattr]
    pub const BOUNDARY_HIT: PyStepStatus =
        PyStepStatus(RStepStatus::BoundaryHit);
    #[classattr]
    pub const LOST_ENGAGEMENT: PyStepStatus =
        PyStepStatus(RStepStatus::LostEngagement);
    #[classattr]
    pub const NO_CONVERGENCE: PyStepStatus =
        PyStepStatus(RStepStatus::NoConvergence);

    fn __repr__(&self) -> &'static str {
        self.name()
    }
    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }
    #[getter]
    fn name(&self) -> &'static str {
        match self.0 {
            RStepStatus::Ok => "Ok",
            RStepStatus::BoundaryHit => "BoundaryHit",
            RStepStatus::LostEngagement => "LostEngagement",
            RStepStatus::NoConvergence => "NoConvergence",
        }
    }
}

// ── ResumeSource (wraps ops/assembly/adaptive/resume.rs ResumeSource)

/// Resume-strategy enum for trace records.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    skip_from_py_object,
    module = "raygeo.trace",
    name = "ResumeSource"
)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyResumeSource(pub(crate) RResumeSource);

#[gen_stub_pymethods]
#[pymethods]
impl PyResumeSource {
    #[classattr]
    pub const NONE: PyResumeSource = PyResumeSource(RResumeSource::None);
    #[classattr]
    pub const WALL_HUG: PyResumeSource =
        PyResumeSource(RResumeSource::ResumeWallHug);
    #[classattr]
    pub const SEGMENT: PyResumeSource =
        PyResumeSource(RResumeSource::ResumeSegment);
    #[classattr]
    pub const MAT: PyResumeSource = PyResumeSource(RResumeSource::ResumeMat);
    #[classattr]
    pub const FRONTIER: PyResumeSource =
        PyResumeSource(RResumeSource::ResumeFrontier);
    #[classattr]
    pub const ENVELOPE: PyResumeSource =
        PyResumeSource(RResumeSource::ResumeEnvelope);
    #[classattr]
    pub const ISLAND: PyResumeSource =
        PyResumeSource(RResumeSource::ResumeIsland);

    fn __repr__(&self) -> &'static str {
        self.name()
    }
    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }
    #[getter]
    fn name(&self) -> &'static str {
        match self.0 {
            RResumeSource::None => "none",
            RResumeSource::ResumeWallHug => "wall_hug",
            RResumeSource::ResumeSegment => "segment",
            RResumeSource::ResumeMat => "mat",
            RResumeSource::ResumeFrontier => "frontier",
            RResumeSource::ResumeEnvelope => "envelope",
            RResumeSource::ResumeIsland => "island",
        }
    }
}

// ── RouteSource (wraps ops/assembly/adaptive/routing.rs RouteSource)

/// Route-strategy enum for trace records.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    eq,
    skip_from_py_object,
    module = "raygeo.trace",
    name = "RouteSource"
)]
#[derive(Clone, PartialEq)]
pub(crate) struct PyRouteSource(pub(crate) RRouteSource);

#[gen_stub_pymethods]
#[pymethods]
impl PyRouteSource {
    #[classattr]
    pub const NONE: PyRouteSource = PyRouteSource(RRouteSource::None);
    #[classattr]
    pub const DIRECT: PyRouteSource =
        PyRouteSource(RRouteSource::RoutingDirect);
    #[classattr]
    pub const FRONTIER: PyRouteSource =
        PyRouteSource(RRouteSource::RoutingFrontier);
    #[classattr]
    pub const MAT: PyRouteSource = PyRouteSource(RRouteSource::RoutingMat);
    #[classattr]
    pub const ZHOP: PyRouteSource = PyRouteSource(RRouteSource::RoutingZHop);

    fn __repr__(&self) -> &'static str {
        self.name()
    }
    #[getter]
    fn value(&self) -> u8 {
        self.0 as u8
    }
    #[getter]
    fn name(&self) -> &'static str {
        match self.0 {
            RRouteSource::None => "none",
            RRouteSource::RoutingDirect => "direct",
            RRouteSource::RoutingFrontier => "frontier",
            RRouteSource::RoutingMat => "mat",
            RRouteSource::RoutingZHop => "zhop",
        }
    }
}

// ── TraceRecord ────────────────────────────────────────────────────

/// One per-step trace record.
///
/// Wraps the decoded msgpack map and exposes fields as attributes.
/// Supports both dot access (``rec.kind``) and dict access (``rec["kind"]``).
#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.trace", name = "TraceRecord")]
pub struct PyTraceRecord {
    inner: Py<PyDict>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTraceRecord {
    #[new]
    fn new(d: Py<PyDict>) -> Self {
        Self { inner: d }
    }

    fn __getattr__(&self, name: &str, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let d = self.inner.bind(py);
        // Look up the value: root dict first, then the payload sub-dict.
        let val: Option<Bound<'_, PyAny>> = match d.get_item(name) {
            Ok(Some(v)) => Some(v),
            _ => None,
        };
        let val = val.or_else(|| {
            if let Ok(Some(p)) = d.get_item("payload") {
                if let Ok(pd) = p.cast::<PyDict>() {
                    return pd.get_item(name).ok().flatten();
                }
            }
            None
        });
        match val {
            Some(v) => {
                let obj: Py<PyAny> = match name {
                    "status" => {
                        let val: u8 = v.extract().unwrap_or(0);
                        let r =
                            num_enum::TryFromPrimitive::try_from_primitive(val)
                                .unwrap_or(RStepStatus::Ok);
                        PyStepStatus(r).into_pyobject(py)?.unbind().into()
                    }
                    "resume_source" => {
                        let val: u8 = v.extract().unwrap_or(0);
                        let r =
                            num_enum::TryFromPrimitive::try_from_primitive(val)
                                .unwrap_or(RResumeSource::None);
                        PyResumeSource(r).into_pyobject(py)?.unbind().into()
                    }
                    "route_source" => {
                        let val: u8 = v.extract().unwrap_or(0);
                        let r =
                            num_enum::TryFromPrimitive::try_from_primitive(val)
                                .unwrap_or(RRouteSource::None);
                        PyRouteSource(r).into_pyobject(py)?.unbind().into()
                    }
                    _ => v.unbind(),
                };
                Ok(obj)
            }
            None => Ok(py.None().into_py_any(py)?),
        }
    }

    fn __getitem__(&self, name: &str, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.__getattr__(name, py)
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let d = self.inner.bind(py);
        Ok(format!("TraceRecord({})", d.repr()?))
    }

    fn __contains__(&self, name: &str, py: Python<'_>) -> bool {
        self.inner.bind(py).contains(name).unwrap_or(false)
    }

    fn keys(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        let d = self.inner.bind(py);
        let mut keys = Vec::new();
        for k in d.keys() {
            keys.push(k.extract::<String>()?);
        }
        Ok(keys)
    }
}

// ── TraceFile ──────────────────────────────────────────────────────

/// Binary trace file with random access to records.
///
/// Usage:
///
/// ```python
/// >>> from raygeo.trace import TraceFile
/// >>> t = TraceFile("path/to/trace.bin")
/// >>> len(t)          # number of records
/// >>> t[0]            # first record (TraceRecord with dot access)
/// >>> t.toolpath      # list of (x, y, move_kind) tuples
/// >>> t.geometry      # dict with tool_radius, boundary, islands, seeds
/// >>> t.mat_nodes     # MAT nodes or empty list
/// ```
#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.trace", name = "TraceFile")]
pub struct PyTraceFile {
    data: TraceFileData,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTraceFile {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let data = TraceFileData::open(Path::new(path)).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("{e}"))
        })?;
        Ok(Self { data })
    }

    fn __len__(&self) -> usize {
        self.data.record_count as usize
    }

    fn __getitem__(
        &self,
        idx: usize,
        py: Python<'_>,
    ) -> PyResult<PyTraceRecord> {
        if idx >= self.data.records.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "record index out of range",
            ));
        }
        let bytes = &self.data.records[idx];
        let msgpack_mod = py.import("msgpack")?;
        let d: Py<PyDict> = msgpack_mod
            .call_method1("unpackb", (bytes.to_vec(),))?
            .extract()?;
        Ok(PyTraceRecord { inner: d })
    }

    #[getter]
    fn ver(&self) -> u16 {
        self.data.ver
    }

    /// Decoded geometry dict (tool_radius, boundary, islands, seeds) from the
    /// first record with ``kind == "geometry"``, or an empty dict.
    #[getter]
    fn geometry(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        for bytes in &self.data.records {
            let msgpack_mod = py.import("msgpack")?;
            let d: Py<PyDict> = msgpack_mod
                .call_method1("unpackb", (bytes.to_vec(),))?
                .extract()?;
            let d_ref = d.bind(py);
            if let Ok(Some(kind)) = d_ref.get_item("kind") {
                if let Ok(s) = kind.extract::<String>() {
                    if s == "geometry" {
                        return Ok(d);
                    }
                }
            }
        }
        Ok(PyDict::new(py).unbind())
    }

    /// Toolpath points extracted from all motion records (those with
    /// ``pos_x`` / ``pos_y``).  Returns a list of ``(x, y, move_kind)``
    /// where ``move_kind`` is a ``MoveKind`` value.
    #[getter]
    fn toolpath(&self, py: Python<'_>) -> PyResult<Vec<(f64, f64, u8)>> {
        let mut out = Vec::new();
        for bytes in &self.data.records {
            let msgpack_mod = py.import("msgpack")?;
            let d: Py<PyDict> = msgpack_mod
                .call_method1("unpackb", (bytes.to_vec(),))?
                .extract()?;
            let d_ref = d.bind(py);
            let kind: String = d_ref
                .get_item("kind")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<String>().ok())
                .unwrap_or_default();
            if kind == "geometry" || kind == "mat" {
                continue;
            }
            let x = d_ref
                .get_item("pos_x")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<f64>().ok())
                .unwrap_or(0.0);
            let y = d_ref
                .get_item("pos_y")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<f64>().ok())
                .unwrap_or(0.0);
            let move_kind = match kind.as_str() {
                "cut" | "init" => RMoveKind::Cut as u8,
                "exit" => RMoveKind::Travel as u8,
                "resume_stall" | "resume_stuck" | "resume" => {
                    RMoveKind::Resume as u8
                }
                _ => RMoveKind::Travel as u8,
            };
            out.push((x, y, move_kind));
        }
        Ok(out)
    }

    /// MAT nodes from the first record with ``kind == "mat"``, or an
    /// empty list.
    #[getter]
    fn mat_nodes(&self, py: Python<'_>) -> PyResult<Vec<(f64, f64)>> {
        if let Some(mat) = find_mat_record(&self.data.records, py)? {
            let d = mat.bind(py);
            let raw: Option<Vec<Vec<f64>>> = d
                .get_item("nodes")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<Vec<Vec<f64>>>().ok());
            Ok(raw.map_or_else(Vec::new, |v| {
                v.into_iter().map(|p| (p[0], p[1])).collect()
            }))
        } else {
            Ok(Vec::new())
        }
    }

    #[getter]
    fn mat_clearances(&self, py: Python<'_>) -> PyResult<Vec<f64>> {
        if let Some(mat) = find_mat_record(&self.data.records, py)? {
            let d = mat.bind(py);
            let vals: Vec<f64> = d
                .get_item("clearances")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<Vec<f64>>().ok())
                .unwrap_or_default();
            Ok(vals)
        } else {
            Ok(Vec::new())
        }
    }

    #[getter]
    fn mat_edges(&self, py: Python<'_>) -> PyResult<Vec<(u32, u32)>> {
        if let Some(mat) = find_mat_record(&self.data.records, py)? {
            let d = mat.bind(py);
            let raw: Option<Vec<Vec<u32>>> = d
                .get_item("edges")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<Vec<Vec<u32>>>().ok());
            Ok(raw.map_or_else(Vec::new, |v| {
                v.into_iter().map(|p| (p[0], p[1])).collect()
            }))
        } else {
            Ok(Vec::new())
        }
    }

    #[getter]
    fn mat_root(&self, py: Python<'_>) -> PyResult<u32> {
        if let Some(mat) = find_mat_record(&self.data.records, py)? {
            let d = mat.bind(py);
            let root: u32 = d
                .get_item("root")
                .ok()
                .flatten()
                .and_then(|v| v.extract::<u32>().ok())
                .unwrap_or(0);
            Ok(root)
        } else {
            Ok(0)
        }
    }
}

fn find_mat_record<'a>(
    records: &[Vec<u8>],
    py: Python<'a>,
) -> PyResult<Option<Py<PyDict>>> {
    for bytes in records {
        let msgpack_mod = py.import("msgpack")?;
        let d: Py<PyDict> = msgpack_mod
            .call_method1("unpackb", (bytes.to_vec(),))?
            .extract()?;
        let d_ref = d.bind(py);
        if let Ok(Some(kind)) = d_ref.get_item("kind") {
            if let Ok(s) = kind.extract::<String>() {
                if s == "mat" {
                    return Ok(Some(d));
                }
            }
        }
    }
    Ok(None)
}

// ── Route detail label function ───────────────────────────────────

/// Return a human-readable label for a route-strategy detail code.
#[gen_stub_pyfunction(
    python = r#"
    def get_route_detail_name(detail: int) -> str:
        """Return a human-readable label for a route-strategy detail code."""
        ...
    "#,
    module = "raygeo.trace"
)]
#[pyfunction]
fn get_route_detail_name(detail: u8) -> &'static str {
    crate::ops::assembly::adaptive::routing::route_detail_label(detail)
}

// ── Helpers ────────────────────────────────────────────────────────
