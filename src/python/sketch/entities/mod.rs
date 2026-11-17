//! Python bindings for the sketch entities, mirroring the
//! `sketch::entities` module tree: one leaf module per entity kind.

pub(crate) mod arc;
pub(crate) mod bezier;
pub(crate) mod circle;
pub(crate) mod ellipse;
pub(crate) mod line;
pub(crate) mod point;
pub(crate) mod polygon;
pub(crate) mod text_box;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pyclass;
use pyo3_stub_gen::{ImportKind, ImportRef, ModuleRef, PyStubType, TypeInfo};
use std::collections::{HashMap, HashSet};

use crate::sketch::entities::Entity;

/// Builds the Python object for an entity from its concrete type.
pub(crate) fn entity_to_py<'py>(
    py: Python<'py>,
    entity: &dyn Entity,
    registry: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    use crate::python::sketch::entities::arc::PyArc;
    use crate::python::sketch::entities::bezier::PyBezier;
    use crate::python::sketch::entities::circle::PyCircle;
    use crate::python::sketch::entities::ellipse::PyEllipse;
    use crate::python::sketch::entities::line::PyLine;
    use crate::python::sketch::entities::polygon::PyPolygon;
    use crate::python::sketch::entities::text_box::PyTextBox;
    use crate::sketch::entities::arc::Arc;
    use crate::sketch::entities::bezier::Bezier;
    use crate::sketch::entities::circle::Circle;
    use crate::sketch::entities::ellipse::Ellipse;
    use crate::sketch::entities::line::Line;
    use crate::sketch::entities::polygon::PolygonEntity;
    use crate::sketch::entities::text_box::TextBoxEntity;

    let any = entity.as_any();
    if let Some(line) = any.downcast_ref::<Line>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyLine {
                inner: line.clone(),
                registry: Some(registry.clone().unbind()),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(arc) = any.downcast_ref::<Arc>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyArc {
                inner: arc.clone(),
                registry: Some(registry.clone().unbind()),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(circle) = any.downcast_ref::<Circle>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyCircle {
                inner: circle.clone(),
                registry: Some(registry.clone().unbind()),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(ellipse) = any.downcast_ref::<Ellipse>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyEllipse {
                inner: ellipse.clone(),
                helper_line_ids: ellipse
                    .helper_line_ids
                    .iter()
                    .map(|&id| id as i64)
                    .collect(),
                registry: Some(registry.clone().unbind()),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(bezier) = any.downcast_ref::<Bezier>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyBezier {
                inner: bezier.clone(),
                registry: Some(registry.clone().unbind()),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(polygon) = any.downcast_ref::<PolygonEntity>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyPolygon {
                inner: polygon.clone(),
                registry: Some(registry.clone().unbind()),
            }),
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(text_box) = any.downcast_ref::<TextBoxEntity>() {
        let object = Py::new(
            py,
            PyClassInitializer::from(PyEntityBase).add_subclass(PyTextBox {
                inner: text_box.clone(),
                registry: Some(registry.clone().unbind()),
            }),
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
    use crate::python::sketch::entities::arc::PyArc;
    use crate::python::sketch::entities::bezier::PyBezier;
    use crate::python::sketch::entities::circle::PyCircle;
    use crate::python::sketch::entities::ellipse::PyEllipse;
    use crate::python::sketch::entities::line::PyLine;
    use crate::python::sketch::entities::polygon::PyPolygon;
    use crate::python::sketch::entities::text_box::PyTextBox;

    if let Ok(line) = obj.extract::<PyRef<'_, PyLine>>() {
        return Ok(Box::new(line.inner.clone()));
    }
    if let Ok(arc) = obj.extract::<PyRef<'_, PyArc>>() {
        return Ok(Box::new(arc.inner.clone()));
    }
    if let Ok(circle) = obj.extract::<PyRef<'_, PyCircle>>() {
        return Ok(Box::new(circle.inner.clone()));
    }
    if let Ok(ellipse) = obj.extract::<PyRef<'_, PyEllipse>>() {
        return Ok(Box::new(ellipse.inner.clone()));
    }
    if let Ok(bezier) = obj.extract::<PyRef<'_, PyBezier>>() {
        return Ok(Box::new(bezier.inner.clone()));
    }
    if let Ok(polygon) = obj.extract::<PyRef<'_, PyPolygon>>() {
        return Ok(Box::new(polygon.inner.clone()));
    }
    if let Ok(text_box) = obj.extract::<PyRef<'_, PyTextBox>>() {
        return Ok(Box::new(text_box.inner.clone()));
    }
    Err(PyTypeError::new_err("expected a sketch entity object"))
}

/// The common base class of every entity binding: a nominal `Entity`
/// type for `isinstance` checks and annotations.  The per-class
/// members live on the subclasses; polymorphism is carried by the
/// Rust-side `Entity` trait.
#[gen_stub_pyclass]
#[pyclass(subclass, module = "raygeo.sketch.entities", name = "Entity")]
#[derive(Default)]
pub struct PyEntityBase;

/// The base of all entity classes, for typed registry returns.
pub struct AnyEntity<'py>(pub Bound<'py, PyAny>);

impl<'a, 'py> FromPyObject<'a, 'py> for AnyEntity<'py> {
    type Error = PyErr;

    fn extract(ob: pyo3::Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        Ok(AnyEntity(ob.to_owned()))
    }
}

impl<'py> IntoPyObject<'py> for AnyEntity<'py> {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(
        self,
        _py: Python<'py>,
    ) -> Result<Self::Output, Self::Error> {
        Ok(self.0)
    }
}

impl PyStubType for AnyEntity<'_> {
    fn type_output() -> TypeInfo {
        let module = ModuleRef::Named("raygeo.sketch.entities".to_string());
        TypeInfo {
            name: "Entity".to_string(),
            source_module: None,
            import: HashSet::from([ImportRef::Module(module.clone())]),
            type_refs: HashMap::from([(
                "Entity".to_string(),
                pyo3_stub_gen::TypeIdentifierRef {
                    module,
                    import_kind: ImportKind::Module,
                },
            )]),
        }
    }
}

/// A sketch point view: coordinates and solver flags.
#[allow(dead_code)]
pub struct AnyPoint<'py>(pub Bound<'py, PyAny>);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let entities_mod = PyModule::new(py, "entities")?;

    entities_mod.add_class::<PyEntityBase>()?;

    point::register(&entities_mod)?;
    line::register(&entities_mod)?;
    arc::register(&entities_mod)?;
    circle::register(&entities_mod)?;
    ellipse::register(&entities_mod)?;
    bezier::register(&entities_mod)?;
    polygon::register(&entities_mod)?;
    text_box::register(&entities_mod)?;

    m.add_submodule(&entities_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities", &entities_mod)?;
    Ok(())
}
