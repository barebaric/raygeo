//! Python bindings for the sketch module.

pub(crate) mod constraints;
pub(crate) mod point;
pub(crate) mod protocols;
pub(crate) mod registry;
pub(crate) mod solver;
pub(crate) mod topology;

use pyo3::prelude::*;

use point::PyPointDef;
use solver::PySketchSolver;

pyo3_stub_gen::module_doc!("raygeo.sketch", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Sketch constraint solving.

Solves 2D geometric constraint systems for parametric sketching: a
set of points is moved so that geometric relationships (horizontal,
distance, tangency, ...) hold, using the trust-region-reflective
least-squares algorithm. The caller describes the problem as a flat
list of PointDef entries and constraint objects (see the
raygeo.sketch.constraints submodules) and receives solved positions,
per-point constrained (fully-determined) flags, and per-constraint
conflict indices.
";

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let sketch_mod = PyModule::new(py, "sketch")?;

    sketch_mod.setattr("__doc__", MODULE_DOC)?;

    sketch_mod.add_class::<PyPointDef>()?;
    sketch_mod.add_class::<PySketchSolver>()?;
    constraints::register(&sketch_mod)?;
    topology::register(&sketch_mod)?;
    registry::register(&sketch_mod)?;

    m.add_submodule(&sketch_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch", &sketch_mod)?;

    Ok(())
}
