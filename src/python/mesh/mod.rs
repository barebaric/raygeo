use pyo3::prelude::*;

pub(crate) mod build;
pub(crate) mod gradient;
pub(crate) mod laplace;
pub(crate) mod pde;
pub(crate) mod remesh;
pub(crate) mod types;

pub fn register(py_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = py_mod.py();
    let m = PyModule::new(py, "mesh")?;
    m.setattr(
        "__doc__",
        "Mesh construction, PDE solving, and spiral tracing.",
    )?;

    build::register(&m)?;
    gradient::register(&m)?;
    laplace::register(&m)?;
    pde::register(&m)?;
    remesh::register(&m)?;
    types::register(&m)?;

    py_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.mesh", &m)?;
    sys_modules.set_item("raygeo.mesh.build", &m.getattr("build")?)?;
    sys_modules.set_item("raygeo.mesh.gradient", &m.getattr("gradient")?)?;
    sys_modules.set_item("raygeo.mesh.laplace", &m.getattr("laplace")?)?;
    sys_modules.set_item("raygeo.mesh.pde", &m.getattr("pde")?)?;
    sys_modules.set_item("raygeo.mesh.remesh", &m.getattr("remesh")?)?;
    sys_modules.set_item("raygeo.mesh.types", &m.getattr("types")?)?;

    Ok(())
}
