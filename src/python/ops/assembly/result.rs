use std::path::PathBuf;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::ops::assembly::result::emit_trace_events;
use crate::python::ops::cut::search::PyToolPose;
use crate::python::ops::PyOps;
use crate::python::trace::{event_kind_str, meta_to_py_dict, move_kind_str};
use crate::trace::Tracer;
use crate::trace_types::{EventKind, ProgressSnapshot, ToolSnapshot};

pub(crate) fn register(assembly_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    assembly_mod.add_class::<PyAssemblyResult>()?;
    Ok(())
}

/// Universal return type for every assembly-level generator.
///
/// Returned by assemblers such as ``generate_helix``,
/// ``generate_toroidal_clear``, ``generate_slot``, and all other
/// assembly-level motion functions.  Contains the generated ``Ops``
/// sequence, the set of polygons that this operation clears, and the
/// tool pose at the start and end of the path.
#[gen_stub_pyclass(module = "raygeo.ops.assembly.result")]
#[pyclass(
    name = "AssemblyResult",
    skip_from_py_object,
    module = "raygeo.ops.assembly.result"
)]
#[derive(Clone, Debug)]
pub struct PyAssemblyResult {
    pub inner: crate::ops::assembly::result::AssemblyResult,
    #[pyo3(get)]
    pub ops: PyOps,
    #[pyo3(get)]
    pub cleared_polygons: Vec<Vec<(f64, f64)>>,
    #[pyo3(get)]
    pub start: PyToolPose,
    #[pyo3(get)]
    pub end: PyToolPose,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAssemblyResult {
    #[new]
    fn __new__() -> Self {
        let inner = crate::ops::assembly::result::AssemblyResult {
            ops: crate::ops::Ops::new(),
            cleared_polygons: vec![],
            start: crate::ops::cut::ToolPose {
                pos: crate::types::Point3D::ZERO,
                heading: 0.0,
            },
            end: crate::ops::cut::ToolPose {
                pos: crate::types::Point3D::ZERO,
                heading: 0.0,
            },
            trace: None,
        };
        PyAssemblyResult::from_inner(inner)
    }

    #[getter]
    fn trace(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        let trace = match &self.inner.trace {
            Some(t) => t,
            None => return Ok(None),
        };

        let trace_dict = PyDict::new(py);
        trace_dict.set_item("attrs", meta_to_py_dict(py, &trace.attrs)?)?;

        let events_list = PyList::empty(py);
        for ev in &trace.events {
            let d = PyDict::new(py);
            d.set_item("kind", event_kind_str(ev.kind as u8))?;

            if let Some(mk) = ev.move_kind {
                d.set_item("move_kind", move_kind_str(mk as u8))?;
            }

            if let Some(ref tool) = ev.tool {
                let td = PyDict::new(py);
                td.set_item("pos_x", tool.pos_x)?;
                td.set_item("pos_y", tool.pos_y)?;
                td.set_item("pos_z", tool.pos_z)?;
                td.set_item("heading", tool.heading)?;
                td.set_item("prev_x", tool.prev_x)?;
                td.set_item("prev_y", tool.prev_y)?;
                td.set_item("prev_z", tool.prev_z)?;
                d.set_item("tool", td)?;
            }

            if let Some(ref progress) = ev.progress {
                let pd = PyDict::new(py);
                pd.set_item("step_idx", progress.step_idx)?;
                pd.set_item("ops_len", progress.ops_len)?;
                pd.set_item("status", progress.status)?;
                d.set_item("progress", pd)?;
            }

            d.set_item("meta", meta_to_py_dict(py, &ev.meta)?)?;
            events_list.append(d)?;
        }
        trace_dict.set_item("events", events_list)?;

        Ok(Some(trace_dict.into()))
    }

    /// Write this result's AssemblyTrace bundle to a trace file.
    ///
    /// Emits a root "workplan" span with one child assembler span
    /// containing either the self-traced events or a minimal
    /// init/exit pair.
    #[pyo3(signature = (path, source, label))]
    fn write_trace(
        &self,
        path: &str,
        source: &str,
        label: &str,
    ) -> PyResult<()> {
        let mut tracer = Tracer::open(Some(PathBuf::from(path)));

        let root = tracer.enter(0, "workplan", "Standalone", None);
        let step_attrs =
            self.inner.trace.as_ref().and_then(|t| t.attrs.clone());
        let span = tracer.enter(root, source, label, step_attrs);

        match &self.inner.trace {
            Some(t) => {
                emit_trace_events(&mut tracer, span, source, &t.events);
            }
            None => {
                tracer.init(
                    span,
                    source,
                    ToolSnapshot {
                        pos_x: self.inner.start.pos.x,
                        pos_y: self.inner.start.pos.y,
                        pos_z: self.inner.start.pos.z,
                        heading: self.inner.start.heading,
                        prev_x: self.inner.start.pos.x,
                        prev_y: self.inner.start.pos.y,
                        prev_z: self.inner.start.pos.z,
                    },
                    ProgressSnapshot::default(),
                    None,
                );
                tracer.event(
                    span,
                    source,
                    EventKind::Exit,
                    Some(ToolSnapshot {
                        pos_x: self.inner.end.pos.x,
                        pos_y: self.inner.end.pos.y,
                        pos_z: self.inner.end.pos.z,
                        heading: self.inner.end.heading,
                        prev_x: self.inner.end.pos.x,
                        prev_y: self.inner.end.pos.y,
                        prev_z: self.inner.end.pos.z,
                    }),
                    None,
                );
            }
        }

        tracer.exit(span, source);
        tracer.exit(root, "workplan");
        tracer.finish();
        Ok(())
    }

    fn __repr__(&self) -> String {
        let n_ops = self.ops.inner.len();
        let n_polys = self.cleared_polygons.len();
        format!(
            "AssemblyResult(ops={n_ops} commands, cleared_polygons={n_polys}, \
             start=({sx:.3},{sy:.3},{sz:.3}), end=({ex:.3},{ey:.3},{ez:.3}))",
            sx = self.start.pos.0,
            sy = self.start.pos.1,
            sz = self.start.pos.2,
            ex = self.end.pos.0,
            ey = self.end.pos.1,
            ez = self.end.pos.2,
        )
    }
}

impl PyAssemblyResult {
    pub fn from_inner(
        inner: crate::ops::assembly::result::AssemblyResult,
    ) -> Self {
        let cleared_polys: Vec<Vec<(f64, f64)>> = inner
            .cleared_polygons
            .iter()
            .map(|poly| poly.iter().map(|p| (p.x, p.y)).collect())
            .collect();

        PyAssemblyResult {
            ops: PyOps {
                inner: inner.ops.clone(),
            },
            cleared_polygons: cleared_polys,
            start: PyToolPose {
                pos: (inner.start.pos.x, inner.start.pos.y, inner.start.pos.z),
                heading: inner.start.heading,
            },
            end: PyToolPose {
                pos: (inner.end.pos.x, inner.end.pos.y, inner.end.pos.z),
                heading: inner.end.heading,
            },
            inner,
        }
    }
}
