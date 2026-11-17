//! Rust helpers for the Python type vocabulary in
//! `raygeo.sketch.types`.

use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;
use pyo3::types::PyAny;
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
