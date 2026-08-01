pyo3_stub_gen::module_doc!("raygeo.pipeline", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Runtime intent-tree pipeline.

Executes trees of NodeRequests on a rayon thread pool. Each node
carries an opaque stage that the CNC layer interprets.

Submodules:

- **request** — ``NodeRequest``: one node of the intent tree.
- **completed** — ``CompletedNode``: completion record.
- **execute** — ``execute_stages``: the rayon-scoped runner.
";

pub(crate) mod cache;
pub(crate) mod callbacks;
pub(crate) mod completed;
pub(crate) mod execute;
pub(crate) mod request;
pub(crate) mod stage;

use pyo3::prelude::*;

pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = parent.py();
    let pipeline_mod = PyModule::new(py, "pipeline")?;
    pipeline_mod.setattr("__doc__", MODULE_DOC)?;

    // Child submodule: raygeo.pipeline.cache
    cache::register(&pipeline_mod)?;

    // Child submodule: raygeo.pipeline.stage
    stage::register(&pipeline_mod)?;

    // Child submodule: raygeo.pipeline.request
    request::register(&pipeline_mod)?;

    // Child submodule: raygeo.pipeline.completed
    completed::register(&pipeline_mod)?;

    // Child submodule: raygeo.pipeline.execute
    execute::register(&pipeline_mod)?;

    parent.add_submodule(&pipeline_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.pipeline", &pipeline_mod)?;

    Ok(())
}
