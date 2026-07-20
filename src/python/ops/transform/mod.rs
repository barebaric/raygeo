//! Python bindings for the transform layer.
//!
//! This module owns the cross-cutting pieces that do not belong to a
//! single transformer:
//!
//! - [`PyExecutionPhase`], the Python-visible phase enum.
//! - [`extract_transformer`], which converts an arbitrary Python spec
//!   object into a `Box<dyn Transformer>` for the core dispatch.
//! - [`PyCallableCallbacks`], the adapter from a Python progress
//!   callable to the core [`TaskCallbacks`] trait.
//!
//! The individual spec pyclasses live in sibling modules that mirror
//! the Rust hierarchy (`smooth.rs`, `overscan.rs`, ...); this module
//! does not re-export them.
//!
//! Layering: this module depends downward on the `ops` layer and does
//! not contain any transform algorithm logic of its own.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyclass_enum;

use crate::ops::callbacks::Callbacks;
use crate::ops::transform as core_transform;
use crate::python::ops::transform::bidir_scan_offset::BidirScanOffsetSpec;
use crate::python::ops::transform::clip::CropSpec;
use crate::python::ops::transform::lead_in_out::LeadInOutSpec;
use crate::python::ops::transform::merge_lines::MergeLinesSpec;
use crate::python::ops::transform::multipass::MultiPassSpec;
use crate::python::ops::transform::optimize::OptimizeSpec;
use crate::python::ops::transform::overscan::OverscanSpec;
use crate::python::ops::transform::smooth::SmoothSpec;
use crate::python::ops::transform::tabs::TabsSpec;

pub(crate) mod bidir_scan_offset;
pub(crate) mod clip;
pub(crate) mod lead_in_out;
pub(crate) mod link;
pub(crate) mod merge_lines;
pub(crate) mod multipass;
pub(crate) mod optimize;
pub(crate) mod overscan;
pub(crate) mod smooth;
pub(crate) mod tabs;

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let transform_mod = PyModule::new(ops_mod.py(), "transform")?;

    transform_mod.add_class::<PyExecutionPhase>()?;
    bidir_scan_offset::register(&transform_mod)?;
    clip::register(&transform_mod)?;
    lead_in_out::register(&transform_mod)?;
    link::register(&transform_mod)?;
    merge_lines::register(&transform_mod)?;
    multipass::register(&transform_mod)?;
    optimize::register(&transform_mod)?;
    overscan::register(&transform_mod)?;
    smooth::register(&transform_mod)?;
    tabs::register(&transform_mod)?;

    ops_mod.add_submodule(&transform_mod)?;

    let sys_modules = ops_mod.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.transform", &transform_mod)?;

    Ok(())
}

/// Execution phase of a transformer.
///
/// Phases are applied in this order: ``GEOMETRY_REFINEMENT`` first,
/// then ``PATH_INTERRUPTION``, then ``POST_PROCESSING``.
#[gen_stub_pyclass_enum]
#[pyclass(
    module = "raygeo.ops.transform",
    name = "ExecutionPhase",
    frozen,
    eq,
    hash,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyExecutionPhase {
    #[pyo3(name = "GEOMETRY_REFINEMENT")]
    GeometryRefinement,
    #[pyo3(name = "PATH_INTERRUPTION")]
    PathInterruption,
    #[pyo3(name = "POST_PROCESSING")]
    PostProcessing,
}

/// Try to extract a transformer spec from a Python object.
///
/// Returns `PyTypeError` if the object is not one of the known spec
/// pyclasses. The returned `Box<dyn Transformer>` is fed directly to
/// [`core_transform::apply_transformers`].
pub fn extract_transformer(
    ob: &Bound<'_, PyAny>,
) -> PyResult<Box<dyn core_transform::Transformer>> {
    if let Ok(s) = ob.extract::<SmoothSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<OptimizeSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<MergeLinesSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<OverscanSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<LeadInOutSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<MultiPassSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<CropSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<TabsSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    if let Ok(s) = ob.extract::<BidirScanOffsetSpec>() {
        return Ok(Box::new(s.into_core()));
    }
    let type_name = ob
        .get_type()
        .qualname()
        .map(|s| s.to_string())
        .unwrap_or_default();
    Err(PyErr::new::<PyTypeError, _>(format!(
        "Unknown transformer spec type: {type_name}"
    )))
}

/// Adapter implementing [`Callbacks`] by delegating to a single
/// Python callable.
///
/// The Python object is expected to be callable as
/// `cb(progress, message)` and to expose an `is_cancelled()` method.
/// Either may be absent on the object; the adapter degrades
/// gracefully (no reports / no cancellation) in that case.
pub struct PyCallableCallbacks {
    cb: Option<Py<PyAny>>,
}

impl PyCallableCallbacks {
    /// Build a callback adapter from an optional Python callable.
    ///
    /// `cb` is converted to an owned `Py<PyAny>` so the resulting
    /// adapter is `Send + Sync` (the GIL is reacquired inside each
    /// method via `Python::attach`).
    pub fn new(cb: Option<Py<PyAny>>) -> Self {
        PyCallableCallbacks { cb }
    }
}

impl Callbacks for PyCallableCallbacks {
    fn report_progress(&self, frac: f64, msg: &str) {
        if let Some(ref cb) = self.cb {
            Python::attach(|py| {
                let _ = cb.call1(py, (frac, msg));
            });
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cb.as_ref().is_some_and(|cb| {
            Python::attach(|py| {
                cb.call_method0(py, "is_cancelled")
                    .and_then(|v| v.extract::<bool>(py))
                    .unwrap_or(false)
            })
        })
    }
}
