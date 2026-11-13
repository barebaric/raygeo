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
use pyo3_stub_gen::{ImportKind, ImportRef, ModuleRef, PyStubType, TypeInfo};
use std::collections::HashMap;

use crate::sketch::entities::Entity;

/// Builds the Python object for an entity from its concrete type.
pub(crate) fn entity_to_py<'py>(
    py: Python<'py>,
    entity: &dyn Entity,
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
            PyLine {
                inner: line.clone(),
            },
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(arc) = any.downcast_ref::<Arc>() {
        let object = Py::new(py, PyArc { inner: arc.clone() })?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(circle) = any.downcast_ref::<Circle>() {
        let object = Py::new(
            py,
            PyCircle {
                inner: circle.clone(),
            },
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(ellipse) = any.downcast_ref::<Ellipse>() {
        let object = Py::new(
            py,
            PyEllipse {
                inner: ellipse.clone(),
            },
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(bezier) = any.downcast_ref::<Bezier>() {
        let object = Py::new(
            py,
            PyBezier {
                inner: bezier.clone(),
            },
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(polygon) = any.downcast_ref::<PolygonEntity>() {
        let object = Py::new(
            py,
            PyPolygon {
                inner: polygon.clone(),
            },
        )?;
        return Ok(object.bind(py).clone().into_any());
    }
    if let Some(text_box) = any.downcast_ref::<TextBoxEntity>() {
        let object = Py::new(
            py,
            PyTextBox {
                inner: text_box.clone(),
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

/// The union of all entity classes, for typed registry returns.
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
        const KINDS: [(&str, &str); 7] = [
            ("line", "Line"),
            ("arc", "Arc"),
            ("circle", "Circle"),
            ("ellipse", "Ellipse"),
            ("bezier", "Bezier"),
            ("polygon", "Polygon"),
            ("text_box", "TextBox"),
        ];

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
#[allow(dead_code)]
pub struct AnyPoint<'py>(pub Bound<'py, PyAny>);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let entities_mod = PyModule::new(py, "entities")?;

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
