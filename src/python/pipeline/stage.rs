pyo3_stub_gen::module_doc!("raygeo.pipeline.stage", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "Stage specification types.";

use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3_stub_gen::derive::gen_stub_pyclass_complex_enum;

/// Stage specification for one node of the intent tree.
///
/// ``Compute`` — a leaf node that produces Ops from geometry via an
/// assembler. ``Aggregate`` — an interior node that concatenates and
/// transforms Ops from one or more upstream nodes.
#[gen_stub_pyclass_complex_enum]
#[pyclass(
    name = "StageSpec",
    module = "raygeo.pipeline.stage",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub enum PyStageSpec {
    /// A compute leaf node.
    #[pyo3(constructor = (part, params, face_id="".to_string()))]
    Compute {
        /// The part to process.
        part: Py<PyAny>,
        /// Compute parameters (assembler, etc.).
        params: Py<PyAny>,
        /// Face identifier for multi-face parts.
        face_id: String,
    },
    /// An aggregate interior node.
    Aggregate {
        /// Aggregate specification (groups, markers, placement).
        spec: Py<PyAny>,
    },
}

pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let stage_mod = PyModule::new(py, "stage")?;
    stage_mod.setattr("__doc__", "Stage specification types.")?;

    stage_mod.add_class::<PyStageSpec>()?;

    m.add_submodule(&stage_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.pipeline.stage", &stage_mod)?;

    Ok(())
}
