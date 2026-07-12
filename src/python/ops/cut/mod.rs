pyo3_stub_gen::module_doc!("raygeo.ops.cut", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Adaptive stepping solver and resume-point search for material removal.

Provides the forward-stepping engagement solver, angle interpolation,
and frontier-walk search used by the adaptive clearing assembler.
";

use pyo3::prelude::*;

pub(crate) mod interp;
pub(crate) mod search;
pub(crate) mod stepper;

pub fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let m = PyModule::new(py, "cut")?;
    m.setattr("__doc__", MODULE_DOC)?;

    interp::register(&m)?;
    search::register(&m)?;
    stepper::register(&m)?;

    ops_mod.add_submodule(&m)?;
    Ok(())
}
