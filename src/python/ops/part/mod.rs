pyo3_stub_gen::module_doc!("raygeo.ops.part", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Workpiece state: cleared-area tracker, stock region, and part descriptor.

Maintains a union of swept-disk polygons and provides a spatial-indexed
windowed query for efficient engagement computation.
";

use pyo3::prelude::*;

pub(crate) mod cleared_area;
pub(crate) mod crescent;
pub(crate) mod face_state;
pub(crate) mod image_source;
#[allow(clippy::module_inception)]
pub(crate) mod part;
pub(crate) mod stock_region;

pub fn register(ops_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = ops_mod.py();
    let m = PyModule::new(py, "part")?;
    m.setattr("__doc__", MODULE_DOC)?;

    cleared_area::register(&m)?;
    crescent::register(&m)?;
    face_state::register(&m)?;
    image_source::register(&m)?;
    part::register(&m)?;
    stock_region::register(&m)?;

    ops_mod.add_submodule(&m)?;
    Ok(())
}
