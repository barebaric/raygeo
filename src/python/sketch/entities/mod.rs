//! Python bindings for the sketch entities, mirroring the
//! `sketch::entities` module tree: one leaf module per entity kind.

pub(crate) mod line;
pub(crate) mod point;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3_stub_gen::{ImportKind, ImportRef, ModuleRef, PyStubType, TypeInfo};
use std::collections::HashMap;

use crate::sketch::entities::Entity;

/// Builds the Python object for an entity from its concrete type.
pub(crate) fn entity_to_py<'py>(
    py: Python<'py>,
    entity: &dyn Entity,
) -> PyResult<Bound<'py, PyAny>> {
    use crate::python::sketch::entities::line::PyLine;
    use crate::sketch::entities::line::Line;

    if let Some(line) = entity.as_any().downcast_ref::<Line>() {
        let object = Py::new(
            py,
            PyLine {
                inner: line.clone(),
            },
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    Err(PyTypeError::new_err(format!(
        "unsupported entity kind: {}",
        entity.type_name()
    )))
}

/// Extracts an owned entity from a Python entity object.
pub(crate) fn extract_entity(
    obj: &Bound<'_, PyAny>,
) -> PyResult<Box<dyn Entity>> {
    use crate::python::sketch::entities::line::PyLine;
    use crate::sketch::entities::line::Line;

    if let Ok(line) = obj.extract::<PyRef<'_, PyLine>>() {
        return Ok(Box::new(line.inner.clone()));
    }
    Err(PyTypeError::new_err("expected a sketch entity object"))
}

/// The union of all entity classes, for typed registry returns.
pub struct AnyEntity<'py>(pub Bound<'py, PyAny>);

impl<'a, 'py> FromPyObject<'a, 'py> for AnyEntity<'py> {
    type Error = PyErr;

    fn extract(ob: pyo3::Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        Ok(AnyEntity(ob.to_owned()))
    }
}

impl PyStubType for AnyEntity<'_> {
    fn type_output() -> TypeInfo {
        const KINDS: [(&str, &str); 1] = [("line", "Line")];

        let mut import = std::collections::HashSet::new();
        let mut type_refs = HashMap::new();
        let qualified: Vec<String> = KINDS
            .iter()
            .map(|(module_name, class_name)| {
                let module = ModuleRef::Named(format!(
                    "raygeo.sketch.entities.{module_name}"
                ));
                import.insert(ImportRef::Module(module.clone()));
                type_refs.insert(
                    class_name.to_string(),
                    pyo3_stub_gen::TypeIdentifierRef {
                        module: module.clone(),
                        import_kind: ImportKind::Module,
                    },
                );
                format!("{module_name}.{class_name}")
            })
            .collect();
        TypeInfo {
            name: format!("typing.Union[{}]", qualified.join(", ")),
            source_module: None,
            import,
            type_refs,
        }
    }
}

/// A sketch point view: coordinates and solver flags.
pub struct AnyPoint<'py>(pub Bound<'py, PyAny>);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let entities_mod = PyModule::new(py, "entities")?;

    point::register(&entities_mod)?;
    line::register(&entities_mod)?;

    m.add_submodule(&entities_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities", &entities_mod)?;
    Ok(())
}
