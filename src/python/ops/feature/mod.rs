pyo3_stub_gen::module_doc!(
    "raygeo.ops.feature",
    "{}",
    "Feature detection for machining analysis."
);

use pyo3::prelude::*;
pub(crate) mod narrow;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let feature_mod = PyModule::new(py, "feature")?;
    feature_mod
        .setattr("__doc__", "Feature detection for machining analysis.")?;

    narrow::register(&feature_mod)?;

    m.add_submodule(&feature_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.feature", &feature_mod)?;

    Ok(())
}
