//! PyO3 bindings for the span/event trace file reader.
//!
//! Exposes `MoveKind`, `TraceFile`, `Span`, `Event`, `ToolSnapshot`,
//! and `ProgressSnapshot` to Python.

use std::collections::HashMap;
use std::path::Path;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PySet};
use pyo3::IntoPyObjectExt;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::trace_types::{MetaValue, MoveKind as RMoveKind, TraceFileData};

// ── Python module registration ─────────────────────────────────────

pyo3_stub_gen::module_doc!("raygeo.trace", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Binary trace-file reader and shared move-type classification.

MoveKind — standard move-type classification shared by all operations.
TraceFile — read a .bin trace file with span/event access.
Span — one span record from a trace file.
Event — one event record from a trace file.
ToolSnapshot — tool position and heading snapshot.
ProgressSnapshot — step progress snapshot.
";

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let trace_mod = PyModule::new(py, "trace")?;
    trace_mod.setattr("__doc__", MODULE_DOC)?;
    trace_mod.add_class::<PyMoveKind>()?;
    trace_mod.add_class::<PyTraceFile>()?;
    trace_mod.add_class::<PySpan>()?;
    trace_mod.add_class::<PyEvent>()?;
    trace_mod.add_class::<PyToolSnapshot>()?;
    trace_mod.add_class::<PyProgressSnapshot>()?;
    trace_mod
        .add_function(wrap_pyfunction!(get_route_detail_name, &trace_mod)?)?;
    m.add_submodule(&trace_mod)?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.trace", &trace_mod)?;
    Ok(())
}

/// Map a routing-strategy detail code to a human-readable label.
#[pyo3_stub_gen::derive::gen_stub_pyfunction(
    python = r#"
    def get_route_detail_name(detail: int) -> str:
        """Return a human-readable label for a route-strategy detail code."""
        ...
    "#,
    module = "raygeo.trace"
)]
#[pyfunction]
pub(crate) fn get_route_detail_name(detail: u8) -> &'static str {
    crate::ops::assembly::adaptive::routing::route_detail_label(detail)
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
    fn name(&self) -> &str {
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

// ── Enum → string mappings ────────────────────────────────────────

pub(crate) fn event_kind_str(kind: u8) -> &'static str {
    match kind {
        12 => "init",
        13 => "move",
        14 => "resume",
        15 => "exit",
        _ => "unknown",
    }
}

pub(crate) fn move_kind_str(kind: u8) -> &'static str {
    match kind {
        0 => "cut",
        1 => "travel",
        2 => "plunge",
        3 => "lead_in",
        4 => "lead_out",
        5 => "link",
        6 => "resume",
        7 => "route",
        _ => "unknown",
    }
}

// ── MetaValue → Python object ─────────────────────────────────────

pub(crate) fn metavalue_to_py(
    py: Python<'_>,
    v: &MetaValue,
) -> PyResult<Py<PyAny>> {
    match v {
        MetaValue::F64(f) => Ok((*f).into_py_any(py)?),
        MetaValue::I64(i) => Ok((*i).into_py_any(py)?),
        MetaValue::U32(u) => Ok((*u).into_py_any(py)?),
        MetaValue::Bool(b) => Ok((*b).into_py_any(py)?),
        MetaValue::Str(s) => Ok(s.clone().into_py_any(py)?),
        MetaValue::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(metavalue_to_py(py, item)?)?;
            }
            Ok(list.into_py_any(py)?)
        }
        MetaValue::Map(m) => {
            let dict = PyDict::new(py);
            for (k, v) in m {
                dict.set_item(k.as_str(), metavalue_to_py(py, v)?)?;
            }
            Ok(dict.into_py_any(py)?)
        }
    }
}

pub(crate) fn meta_to_py_dict(
    py: Python<'_>,
    meta: &Option<crate::trace_types::Meta>,
) -> PyResult<Py<PyAny>> {
    match meta {
        None => Ok(PyDict::new(py).into_py_any(py)?),
        Some(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k.as_str(), metavalue_to_py(py, v)?)?;
            }
            Ok(dict.into_py_any(py)?)
        }
    }
}

// ── KindPeek for partial msgpack decode ─────────────────────────────

#[derive(serde::Deserialize)]
struct KindPeek {
    kind: u8,
}

// ── ToolSnapshot ──────────────────────────────────────────────────

/// Snapshot of tool position and heading at a trace event.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    module = "raygeo.trace",
    name = "ToolSnapshot",
    skip_from_py_object
)]
pub(crate) struct PyToolSnapshot {
    #[pyo3(get)]
    pos_x: f64,
    #[pyo3(get)]
    pos_y: f64,
    #[pyo3(get)]
    pos_z: f64,
    #[pyo3(get)]
    heading: f64,
    #[pyo3(get)]
    prev_x: f64,
    #[pyo3(get)]
    prev_y: f64,
    #[pyo3(get)]
    prev_z: f64,
}

impl PyToolSnapshot {
    fn from_rust(t: &crate::trace_types::ToolSnapshot) -> Self {
        Self {
            pos_x: t.pos_x,
            pos_y: t.pos_y,
            pos_z: t.pos_z,
            heading: t.heading,
            prev_x: t.prev_x,
            prev_y: t.prev_y,
            prev_z: t.prev_z,
        }
    }
}

// ── ProgressSnapshot ──────────────────────────────────────────────

/// Snapshot of step progress during trace execution.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    module = "raygeo.trace",
    name = "ProgressSnapshot",
    skip_from_py_object
)]
pub(crate) struct PyProgressSnapshot {
    #[pyo3(get)]
    step_idx: u32,
    #[pyo3(get)]
    ops_len: u32,
    #[pyo3(get)]
    status: u8,
}

impl PyProgressSnapshot {
    fn from_rust(p: &crate::trace_types::ProgressSnapshot) -> Self {
        Self {
            step_idx: p.step_idx,
            ops_len: p.ops_len,
            status: p.status,
        }
    }
}

// ── Event ──────────────────────────────────────────────────────────

/// One trace event (init / move / resume / exit).
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.trace", name = "Event", skip_from_py_object)]
pub(crate) struct PyEvent {
    #[pyo3(get)]
    kind: String,
    #[pyo3(get)]
    seq: u32,
    #[pyo3(get)]
    span: u32,
    #[pyo3(get)]
    source: String,
    #[pyo3(get)]
    move_kind: Option<String>,
    #[pyo3(get)]
    tool: Option<Py<PyToolSnapshot>>,
    #[pyo3(get)]
    progress: Option<Py<PyProgressSnapshot>>,
    #[pyo3(get)]
    meta: Py<PyAny>,
}

// ── Span ──────────────────────────────────────────────────────────

/// One span record from a trace file.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.trace", name = "Span", skip_from_py_object)]
pub(crate) struct PySpan {
    #[pyo3(get)]
    id: u32,
    #[pyo3(get)]
    parent: u32,
    #[pyo3(get)]
    source: String,
    #[pyo3(get)]
    label: String,
    #[pyo3(get)]
    attrs: Py<PyAny>,
    children: Vec<Py<PySpan>>,
    events: Vec<Py<PyEvent>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySpan {
    #[getter]
    fn children(&self, py: Python<'_>) -> Vec<Py<PySpan>> {
        self.children.iter().map(|c| c.clone_ref(py)).collect()
    }

    #[getter]
    fn events(&self, py: Python<'_>) -> Vec<Py<PyEvent>> {
        self.events.iter().map(|e| e.clone_ref(py)).collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Span(id={}, parent={}, source={:?}, label={:?})",
            self.id, self.parent, self.source, self.label,
        )
    }
}

// ── TraceFile ─────────────────────────────────────────────────────

/// Binary trace file with span/event access.
///
/// Usage::
///
/// ```python
/// >>> from raygeo.trace import TraceFile
/// >>> t = TraceFile("path/to/trace.bin")
/// >>> t.ver
/// 3
/// >>> t.root
/// Span(id=1, parent=0, source='workplan', label='Workplan')
/// >>> len(t.events)
/// 42
/// ```
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.trace", name = "TraceFile")]
pub(crate) struct PyTraceFile {
    ver: u16,
    spans: Vec<Py<PySpan>>,
    events: Vec<Py<PyEvent>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTraceFile {
    #[new]
    fn new(path: &str, py: Python<'_>) -> PyResult<Self> {
        let data = TraceFileData::open(Path::new(path)).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("{e}"))
        })?;

        let mut span_starts: Vec<crate::trace_types::SpanRecord> = Vec::new();
        let mut trace_events: Vec<crate::trace_types::TraceEvent> = Vec::new();

        for blob in &data.records {
            let peek: KindPeek = rmp_serde::from_slice(blob).map_err(|e| {
                PyValueError::new_err(format!("msgpack decode failed: {e}"))
            })?;
            if peek.kind == 10 {
                let rec: crate::trace_types::SpanRecord =
                    rmp_serde::from_slice(blob).map_err(|e| {
                        PyValueError::new_err(format!(
                            "msgpack decode failed: {e}"
                        ))
                    })?;
                span_starts.push(rec);
            } else if peek.kind >= 12 && peek.kind <= 15 {
                let rec: crate::trace_types::TraceEvent =
                    rmp_serde::from_slice(blob).map_err(|e| {
                        PyValueError::new_err(format!(
                            "msgpack decode failed: {e}"
                        ))
                    })?;
                trace_events.push(rec);
            }
        }

        // Build PySpan objects (no children/events yet)
        let mut id_to_idx: HashMap<u32, usize> = HashMap::new();
        let mut spans: Vec<Py<PySpan>> = Vec::with_capacity(span_starts.len());
        for rec in &span_starts {
            let attrs = meta_to_py_dict(py, &rec.attrs)?;
            let span = PySpan {
                id: rec.id,
                parent: rec.parent,
                source: rec.source.clone(),
                label: rec.label.clone(),
                attrs,
                children: Vec::new(),
                events: Vec::new(),
            };
            let idx = spans.len();
            id_to_idx.insert(rec.id, idx);
            spans.push(Py::new(py, span)?);
        }

        // Build PyEvent objects and assign to spans
        let mut events: Vec<Py<PyEvent>> =
            Vec::with_capacity(trace_events.len());
        let mut event_assignments: Vec<(usize, Py<PyEvent>)> = Vec::new();

        for rec in &trace_events {
            let kind_str = event_kind_str(rec.kind).to_string();
            let move_kind_str =
                rec.move_kind.map(|k| move_kind_str(k).to_string());
            let tool = match &rec.tool {
                Some(t) => Some(Py::new(py, PyToolSnapshot::from_rust(t))?),
                None => None,
            };
            let progress = match &rec.progress {
                Some(p) => Some(Py::new(py, PyProgressSnapshot::from_rust(p))?),
                None => None,
            };
            let meta = meta_to_py_dict(py, &rec.meta)?;

            let evt = PyEvent {
                kind: kind_str,
                seq: rec.seq,
                span: rec.span,
                source: rec.source.clone(),
                move_kind: move_kind_str,
                tool,
                progress,
                meta,
            };
            let py_evt = Py::new(py, evt)?;

            if let Some(&span_idx) = id_to_idx.get(&rec.span) {
                event_assignments.push((span_idx, py_evt.clone_ref(py)));
            }
            events.push(py_evt);
        }

        for (span_idx, evt) in &event_assignments {
            spans[*span_idx]
                .borrow_mut(py)
                .events
                .push(evt.clone_ref(py));
        }

        // Build children
        let mut child_assignments: Vec<(usize, usize)> = Vec::new();
        for (i, span) in spans.iter().enumerate() {
            let parent_id = span.borrow(py).parent;
            if parent_id != 0 {
                if let Some(&parent_idx) = id_to_idx.get(&parent_id) {
                    child_assignments.push((parent_idx, i));
                }
            }
        }
        for (parent_idx, child_idx) in &child_assignments {
            let child = spans[*child_idx].clone_ref(py);
            spans[*parent_idx].borrow_mut(py).children.push(child);
        }

        Ok(PyTraceFile {
            ver: data.ver,
            spans,
            events,
        })
    }

    #[getter]
    fn ver(&self) -> u16 {
        self.ver
    }

    #[getter]
    fn spans(&self, py: Python<'_>) -> Vec<Py<PySpan>> {
        self.spans.iter().map(|s| s.clone_ref(py)).collect()
    }

    #[getter]
    fn events(&self, py: Python<'_>) -> Vec<Py<PyEvent>> {
        self.events.iter().map(|e| e.clone_ref(py)).collect()
    }

    /// The root span (first span with parent == 0), or None.
    #[getter]
    fn root(&self, py: Python<'_>) -> Option<Py<PySpan>> {
        for span in &self.spans {
            if span.borrow(py).parent == 0 {
                return Some(span.clone_ref(py));
            }
        }
        None
    }

    /// Distinct source strings across all spans and events.
    #[getter]
    fn sources(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let s = PySet::empty(py)?;
        for span in &self.spans {
            let b = span.borrow(py);
            s.add(b.source.clone())?;
        }
        for evt in &self.events {
            let b = evt.borrow(py);
            s.add(b.source.clone())?;
        }
        s.into_py_any(py)
    }

    /// Toolpath points from Move events.
    ///
    /// Returns a list of ``(x, y, move_kind_name)`` tuples.
    /// If *span* is given (an int span id or a Span object), restrict
    /// to events belonging to that span.
    #[pyo3(signature = (span = None))]
    fn toolpath(
        &self,
        span: Option<&Bound<'_, PyAny>>,
        py: Python<'_>,
    ) -> PyResult<Vec<(f64, f64, String)>> {
        let span_id: Option<u32> = match span {
            None => None,
            Some(obj) => {
                if let Ok(id) = obj.extract::<u32>() {
                    Some(id)
                } else if let Ok(id) =
                    obj.getattr("id").and_then(|a| a.extract::<u32>())
                {
                    Some(id)
                } else {
                    return Err(pyo3::exceptions::PyTypeError::new_err(
                        "span must be an int or Span object",
                    ));
                }
            }
        };

        let mut out = Vec::new();
        for evt in &self.events {
            let b = evt.borrow(py);
            if b.kind != "move" {
                continue;
            }
            if let Some(sid) = span_id {
                if b.span != sid {
                    continue;
                }
            }
            if let Some(ref tool) = b.tool {
                let tool = tool.borrow(py);
                let mk = b.move_kind.as_deref().unwrap_or("travel").to_string();
                out.push((tool.pos_x, tool.pos_y, mk));
            }
        }
        Ok(out)
    }

    fn __len__(&self) -> usize {
        self.events.len()
    }

    fn __getitem__(&self, idx: usize, py: Python<'_>) -> PyResult<Py<PyEvent>> {
        if idx >= self.events.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "event index out of range",
            ));
        }
        Ok(self.events[idx].clone_ref(py))
    }

    fn __repr__(&self) -> String {
        format!(
            "TraceFile(ver={}, spans={}, events={})",
            self.ver,
            self.spans.len(),
            self.events.len(),
        )
    }
}
