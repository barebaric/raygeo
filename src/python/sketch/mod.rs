//! Python bindings for the sketch module.

pub(crate) mod solver;

use pyo3::prelude::*;

pyo3_stub_gen::module_doc!("raygeo.sketch", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Sketch constraint solving.

Solves 2D geometric constraint systems for parametric sketching: a
set of points is moved so that geometric relationships (horizontal,
distance, tangency, ...) hold, using the trust-region-reflective
least-squares algorithm. The caller describes the problem as a flat
list of PointDef entries and typed Constraint variants and receives
solved positions, per-point constrained (fully-determined) flags,
and per-constraint conflict indices.
";

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let sketch_mod = PyModule::new(py, "sketch")?;

    sketch_mod.setattr("__doc__", MODULE_DOC)?;

    sketch_mod.add_class::<solver::PyPointDef>()?;
    sketch_mod.add_class::<solver::PyConstraint>()?;
    sketch_mod.add_class::<solver::PySketchSolver>()?;

    m.add_submodule(&sketch_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch", &sketch_mod)?;

    Ok(())
}
