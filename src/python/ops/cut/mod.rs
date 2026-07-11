pyo3_stub_gen::module_doc!("raygeo.ops.cut", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Cleared-area tracker for material removal.

Maintains a union of swept-disk polygons and provides a spatial-indexed
windowed query for efficient engagement computation.
";

use pyo3::prelude::*;

pub(crate) mod cleared_area;
pub(crate) mod crescent;
pub(crate) mod interp;
pub(crate) mod part;
pub(crate) mod search;
pub(crate) mod stepper;

pub fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let m = PyModule::new(py, "cut")?;
    m.setattr("__doc__", MODULE_DOC)?;

    cleared_area::register(&m)?;
    crescent::register(&m)?;
    interp::register(&m)?;
    part::register(&m)?;
    search::register(&m)?;
    stepper::register(&m)?;

    ops_mod.add_submodule(&m)?;
    Ok(())
}
