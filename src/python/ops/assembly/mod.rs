pyo3_stub_gen::module_doc!("raygeo.ops.assembly", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Motion-path assembly: turning raw geometry primitives into Ops.

Functions in this module compose geo-layer primitives (polylines, arcs,
polygons) into complete motion sequences represented as Ops objects.
They decide traversal order, linking strategy, lead-in/out, overscan,
and tab insertion — concerns that belong to motion assembly rather than
pure geometry.
";

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::ops::assembly::ProgressEvent;

pub(crate) mod adaptive;
pub(crate) mod helix;
pub(crate) mod profile;
pub(crate) mod ramp;
pub(crate) mod result;
pub(crate) mod slot;
pub(crate) mod spiral;
pub(crate) mod toroid;
pub(crate) mod wavefront;

/// Convert a Rust ProgressEvent into a Python dict.
pub(crate) fn progress_event_to_py(
    py: Python<'_>,
    event: ProgressEvent,
) -> Py<PyAny> {
    match event {
        ProgressEvent::StepStart { step_index, label } => {
            let d = PyDict::new(py);
            d.set_item("kind", "step_start").unwrap();
            d.set_item("step_index", step_index).unwrap();
            d.set_item("label", label).unwrap();
            d.into_any().unbind()
        }
        ProgressEvent::Ops {
            commands,
            ops_total,
        } => {
            let d = PyDict::new(py);
            d.set_item("kind", "ops").unwrap();
            d.set_item("ops_count", commands.len()).unwrap();
            d.set_item("ops_total", ops_total).unwrap();
            d.into_any().unbind()
        }
        ProgressEvent::StepEnd { step_index } => {
            let d = PyDict::new(py);
            d.set_item("kind", "step_end").unwrap();
            d.set_item("step_index", step_index).unwrap();
            d.into_any().unbind()
        }
    }
}

pub(crate) fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let assembly_mod = PyModule::new(py, "assembly")?;
    assembly_mod.setattr("__doc__", MODULE_DOC)?;

    adaptive::register(&assembly_mod)?;
    helix::register(&assembly_mod)?;
    profile::register(&assembly_mod)?;
    ramp::register(&assembly_mod)?;
    result::register(&assembly_mod)?;
    slot::register(&assembly_mod)?;
    spiral::register(&assembly_mod)?;
    toroid::register(&assembly_mod)?;
    wavefront::register(&assembly_mod)?;

    ops_mod.add_submodule(&assembly_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.assembly", &assembly_mod)?;

    Ok(())
}
