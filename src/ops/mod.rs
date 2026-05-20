use pyo3::prelude::*;
mod axis;
mod container;
mod enums;
mod serialize;
mod state;

pub use container::{PyCommandInfo, PyOps, PyOpsSection, PyOpsSectionRange};

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let ops_mod = PyModule::new(py, "ops")?;

    ops_mod.setattr(
        "__doc__",
        "Command sequence (Ops) manipulation for laser cutter motion control.\n\
        \n\
        Provides Ops — a container of command primitives (move, line, arc, bezier,\n\
        state changes) with methods for transformation, clipping, linearization,\n\
        timing estimation, serialization, and more.\n\
        \n\
        Submodules:\n\
        - raygeo.ops.group — section/segmentation utilities\n\
        \n\
        Types:\n\
        - Ops — the main command sequence container\n\
        - CommandInfo — metadata about command positions in the sequence\n\
        - OpsSection, OpsSectionRange — sub-sequence views",
    )?;
    enums::register(&ops_mod)?;
    axis::register(&ops_mod)?;
    state::register(&ops_mod)?;
    ops_mod.add_class::<PyOps>()?;
    ops_mod.add_class::<PyCommandInfo>()?;
    ops_mod.add_class::<PyOpsSection>()?;
    ops_mod.add_class::<PyOpsSectionRange>()?;

    m.add_submodule(&ops_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops", &ops_mod)?;

    Ok(())
}
