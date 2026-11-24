//! Rust helpers for the Python type vocabulary in
//! `raygeo.sketch.types`.

use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3_stub_gen::derive::gen_type_alias_from_python;
use pyo3_stub_gen::{ImportKind, ImportRef, ModuleRef, PyStubType, TypeInfo};

/// An entity or point id from the registry's id space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

impl<'a, 'py> FromPyObject<'a, 'py> for EntityId {
    type Error = PyErr;

    fn extract(
        ob: pyo3::Borrowed<'a, 'py, PyAny>,
    ) -> Result<Self, Self::Error> {
        Ok(EntityId(ob.extract::<i64>()? as u64))
    }
}

impl<'py> IntoPyObject<'py> for EntityId {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(
        self,
        py: Python<'py>,
    ) -> Result<Self::Output, Self::Error> {
        Ok((self.0 as i64).into_pyobject(py)?.into_any())
    }
}

impl PyStubType for EntityId {
    fn type_output() -> TypeInfo {
        let module = ModuleRef::Named("raygeo.sketch.types".to_string());
        TypeInfo {
            name: "EntityID".to_string(),
            source_module: None,
            import: HashSet::from([ImportRef::Module(module.clone())]),
            type_refs: HashMap::from([(
                "EntityID".to_string(),
                pyo3_stub_gen::TypeIdentifierRef {
                    module,
                    import_kind: ImportKind::Module,
                },
            )]),
        }
    }
}

/// The status of a sketch constraint.
///
/// The runtime object is an `IntEnum` built in [`register_types`]; this
/// declaration exists so the stub generator emits its members.
#[pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch.types", name = "ConstraintStatus")]
pub struct PyConstraintStatus;

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyConstraintStatus {
    #[classattr]
    pub const VALID: i64 = 1;
    #[classattr]
    pub const EXPRESSION_BASED: i64 = 2;
    #[classattr]
    pub const ERROR: i64 = 3;
    #[classattr]
    pub const CONFLICTING: i64 = 4;
}

gen_type_alias_from_python!(
    "raygeo.sketch.types",
    r#"
    type EntityID = int
    "#
);

#[pyo3_stub_gen::derive::gen_stub_pyclass]
#[pyclass(frozen, module = "raygeo.sketch.types", name = "WaypointType")]
pub struct PyWaypointType;

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyWaypointType {
    #[classattr]
    pub const SHARP: &'static str = "sharp";
    #[classattr]
    pub const SMOOTH: &'static str = "smooth";
    #[classattr]
    pub const SYMMETRIC: &'static str = "symmetric";
}

pub(crate) fn register_types(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = module.py();
    let members = pyo3::types::PyDict::new(py);
    members.set_item("VALID", 1)?;
    members.set_item("EXPRESSION_BASED", 2)?;
    members.set_item("ERROR", 3)?;
    members.set_item("CONFLICTING", 4)?;
    let status = py
        .import("enum")?
        .getattr("IntEnum")?
        .call1(("ConstraintStatus", members))?;
    module.setattr("ConstraintStatus", status)?;
    module.add_class::<PyWaypointType>()?;
    Ok(())
}
