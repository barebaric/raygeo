pub(crate) mod collision;
pub(crate) mod genetic;
pub(crate) mod gravity;
pub(crate) mod ifp;
pub(crate) mod nfp;
pub(crate) mod placement;

use pyo3::prelude::*;

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let nest_mod = PyModule::new(py, "nest2d")?;
    nest_mod
        .setattr("__doc__", "2D Nesting algorithms (NFP, IFP, placement).")?;

    let nfp_mod = PyModule::new(py, "nfp")?;
    nfp_mod.setattr("__doc__", nfp::MODULE_DOC)?;
    nfp::register(&nfp_mod)?;

    let ifp_mod = PyModule::new(py, "ifp")?;
    ifp_mod.setattr("__doc__", ifp::MODULE_DOC)?;
    ifp::register(&ifp_mod)?;

    let placement_mod = PyModule::new(py, "placement")?;
    placement_mod.setattr("__doc__", placement::MODULE_DOC)?;
    placement::register(&placement_mod)?;

    let gravity_mod = PyModule::new(py, "gravity")?;
    gravity_mod.setattr("__doc__", gravity::MODULE_DOC)?;
    gravity::register(&gravity_mod)?;

    let genetic_mod = PyModule::new(py, "genetic")?;
    genetic_mod.setattr("__doc__", genetic::MODULE_DOC)?;
    genetic::register(&genetic_mod)?;

    let collision_mod = PyModule::new(py, "collision")?;
    collision_mod.setattr("__doc__", collision::MODULE_DOC)?;
    collision::register(&collision_mod)?;

    nest_mod.add_submodule(&nfp_mod)?;
    nest_mod.add_submodule(&ifp_mod)?;
    nest_mod.add_submodule(&placement_mod)?;
    nest_mod.add_submodule(&gravity_mod)?;
    nest_mod.add_submodule(&genetic_mod)?;
    nest_mod.add_submodule(&collision_mod)?;
    m.add_submodule(&nest_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.geo.algo.nest2d", &nest_mod)?;
    sys_modules.set_item("raygeo.geo.algo.nest2d.nfp", &nfp_mod)?;
    sys_modules.set_item("raygeo.geo.algo.nest2d.ifp", &ifp_mod)?;
    sys_modules.set_item("raygeo.geo.algo.nest2d.placement", &placement_mod)?;
    sys_modules.set_item("raygeo.geo.algo.nest2d.gravity", &gravity_mod)?;
    sys_modules.set_item("raygeo.geo.algo.nest2d.genetic", &genetic_mod)?;
    sys_modules.set_item("raygeo.geo.algo.nest2d.collision", &collision_mod)?;

    Ok(())
}
