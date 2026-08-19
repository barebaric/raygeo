//! Python bindings for `raygeo.ops.material.fold`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::ops::material::fold;

use super::spec::PyMaterialFoldSpec;
use super::state::PyMaterialState;

pyo3_stub_gen::module_doc!("raygeo.ops.material.fold", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
The fold kernel: aggregate material effects against one stock.
";

/// Fold the spec's entries against the stock into a snapshot.
///
/// Runs the prismatic fold only: through-cut classification, void
/// union clipped to the stock, the burn surface map, provenance, and
/// escalation signals. The GIL is released while folding.
#[gen_stub_pyfunction(module = "raygeo.ops.material.fold")]
#[pyfunction(name = "fold_effects")]
fn fold_effects_py(
    py: Python<'_>,
    spec: &PyMaterialFoldSpec,
) -> PyResult<PyMaterialState> {
    let core_spec = spec.to_core(py)?;
    let state = py
        .detach(|| fold::fold_effects(&core_spec))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    Ok(PyMaterialState { inner: state })
}

pub(crate) fn register(mat_mod: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = mat_mod.py();
    let m = PyModule::new(py, "fold")?;
    m.setattr("__doc__", MODULE_DOC)?;
    register_functions!(m, fold_effects_py,);

    mat_mod.add_submodule(&m)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.ops.material.fold", &m)?;

    Ok(())
}
