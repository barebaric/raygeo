pyo3_stub_gen::module_doc!("raygeo.ops", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Command sequence (Ops) manipulation for laser cutter motion control.

Ops is a container of ordered commands (move, line, arc, bezier, state
changes like power/speed) that defines a complete laser engraving or
cutting job. It supports building sequences programmatically (move_to,
line_to, arc_to, etc.), transforming them (translate, rotate, scale,
transform with 4x4 matrices), clipping to rectangles or regions,
linearizing curves, estimating processing time, and serializing to
dict or numpy arrays for persistence.

The module also provides command-type enumerations (CommandType,
CommandCategory, SectionType), machine State tracking (power, speed,
air assist, frequency), and an Axis bitflag for multi-axis machines.
";

use pyo3::prelude::*;
pub(crate) mod axis;
mod container;
pub(crate) mod optimize;
mod serialize;
pub(crate) mod state;
pub(crate) mod types;

pub use container::{PyCommandInfo, PyOps, PyOpsSection, PyOpsSectionRange};

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let ops_mod = PyModule::new(py, "ops")?;

    ops_mod.setattr("__doc__", MODULE_DOC)?;

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

    // Child submodule: raygeo.ops.algo.optimize
    optimize::register(&ops_mod)?;

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
