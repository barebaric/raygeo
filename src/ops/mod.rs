use pyo3::prelude::*;
mod axis;
mod container;
mod serialize;
mod state;
mod types;

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
        Types:\n\
        - Ops — the main command sequence container\n\
        - CommandInfo — metadata about command positions in the sequence\n\
        - OpsSection, OpsSectionRange — sub-sequence views",
    )?;

    // Child submodule: raygeo.ops.types
    let types_mod = PyModule::new(py, "types")?;
    types::register(&types_mod)?;
    ops_mod.add_submodule(&types_mod)?;

    // Child submodule: raygeo.ops.axis
    let axis_mod = PyModule::new(py, "axis")?;
    axis::register(&axis_mod)?;
    ops_mod.add_submodule(&axis_mod)?;

    // Child submodule: raygeo.ops.state
    let state_mod = PyModule::new(py, "state")?;
    state::register(&state_mod)?;
    ops_mod.add_submodule(&state_mod)?;

    // Root-level classes
    ops_mod.add_class::<PyOps>()?;
    ops_mod.add_class::<PyCommandInfo>()?;
    ops_mod.add_class::<PyOpsSection>()?;
    ops_mod.add_class::<PyOpsSectionRange>()?;

    m.add_submodule(&ops_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops", &ops_mod)?;
    sys_modules.set_item("raygeo.ops.types", &types_mod)?;
    sys_modules.set_item("raygeo.ops.axis", &axis_mod)?;
    sys_modules.set_item("raygeo.ops.state", &state_mod)?;

    Ok(())
}
