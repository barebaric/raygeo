use pyo3::prelude::*;
mod axis;
mod container;
mod enums;
mod flip;
mod group;
mod serialize;
mod state;

pub use container::{PyCommandInfo, PyOps};

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let ops_mod = PyModule::new(py, "ops")?;

    enums::register(&ops_mod)?;
    axis::register(&ops_mod)?;
    state::register(&ops_mod)?;
    group::register(&ops_mod)?;
    flip::register(&ops_mod)?;
    ops_mod.add_class::<PyOps>()?;
    ops_mod.add_class::<PyCommandInfo>()?;

    m.add_submodule(&ops_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops", &ops_mod)?;

    Ok(())
}
