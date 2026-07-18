//! Python bindings for the transformer batch dispatch.
//!
//! This module owns only the cross-cutting pieces that do not belong to
//! a single transformer:
//!
//! - [`PyExecutionPhase`], the Python-visible phase enum.
//! - [`extract_transformer`], which converts an arbitrary Python spec
//!   object into a `Box<dyn Transformer>` for the core dispatch.
//! - [`PyProgress`], the adapter from a Python progress callback to the
//!   core [`Progress`](crate::ops::transform::apply::Progress) trait.
//!
//! The individual spec pyclasses live in sibling modules that mirror the
//! Rust hierarchy (`smooth.rs`, `overscan.rs`, ...); this module does
//! not re-export them.
//!
//! Layering: this module depends downward on the `ops` layer and does
//! not contain any transform algorithm logic of its own.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyclass_enum;

use crate::ops::transform::apply as core_apply;
use crate::python::ops::transform::bidir_scan_offset::BidirScanOffsetSpec;
use crate::python::ops::transform::clip::CropSpec;
use crate::python::ops::transform::lead_in_out::LeadInOutSpec;
use crate::python::ops::transform::merge_lines::MergeLinesSpec;
use crate::python::ops::transform::multipass::MultiPassSpec;
use crate::python::ops::transform::optimize::OptimizeSpec;
use crate::python::ops::transform::overscan::OverscanSpec;
use crate::python::ops::transform::smooth::SmoothSpec;
use crate::python::ops::transform::tabs::TabsSpec;

/// Register the `ExecutionPhase` class on the transform submodule.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyExecutionPhase>()?;
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
/// [`core_apply::apply_transformers`].
pub fn extract_transformer(
    ob: &Bound<'_, PyAny>,
) -> PyResult<Box<dyn core_apply::Transformer>> {
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

/// Adapter implementing [`core_apply::Progress`] by delegating to a
/// single Python object.
///
/// The Python object is expected to be callable as
/// `cb(progress, message)` and to expose an `is_cancelled()` method.
/// Either may be absent on the object; the adapter degrades gracefully
/// (no reports / no cancellation) in that case.
pub struct PyProgress<'py> {
    pub cb: Option<&'py Bound<'py, PyAny>>,
}

impl<'py> core_apply::Progress for PyProgress<'py> {
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
