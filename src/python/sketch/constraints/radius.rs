//! Bindings for the radius constraint.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyString, PyType};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::{surface, PyConstraintBase};
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, ScreenMap};
use crate::sketch::constraint::{PointContext, SketchConstraint};
use crate::sketch::constraints::radius::RadiusConstraint as CoreConstraint;

/// Enforces the radius of a circle or arc.
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    extends = PyConstraintBase,
    subclass,
    module = "raygeo.sketch.constraints.radius",
    name = "RadiusConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyRadiusConstraint {
    /// The constrained entity (circle or arc).
    #[pyo3(get, set)]
    pub entity_id: i64,
    /// Target radius value.
    #[pyo3(get, set)]
    pub value: f64,
    /// Reference user-visibility flag.
    #[pyo3(get, set)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
    /// Stored expression string (evaluated by `update_from_context`).
    #[pyo3(get, set)]
    pub expression: Option<String>,
}

/// The reference constructor's `value: str | float` handling: an
/// explicit expression wins, a string value becomes the expression,
/// otherwise the value is the plain number.
pub(crate) fn split_value(
    value: &Bound<'_, PyAny>,
    expression: Option<String>,
) -> PyResult<(f64, Option<String>)> {
    if let Some(expression) = expression {
        let value: f64 = value.extract()?;
        return Ok((value, Some(expression)));
    }
    if let Ok(text) = value.cast::<PyString>() {
        return Ok((0.0, Some(text.to_str()?.to_string())));
    }
    let value: f64 = value.extract()?;
    Ok((value, None))
}

/// The `type` entry of the reference serialization dictionaries.
pub(crate) fn dict_type(
    dict: &Bound<'_, PyDict>,
    type_name: &str,
) -> PyResult<()> {
    dict.set_item("type", type_name)
}

/// Sets the `expression` entry only when one is stored, mirroring
/// the reference `to_dict`.
pub(crate) fn dict_expression(
    dict: &Bound<'_, PyDict>,
    expression: &Option<String>,
) -> PyResult<()> {
    if let Some(expression) = expression {
        dict.set_item("expression", expression)?;
    }
    Ok(())
}

#[gen_stub_pymethods]
#[pymethods]
impl PyRadiusConstraint {
    /// Creates the constraint for an entity.
    #[new]
    #[pyo3(signature = (entity_id, value, expression = None,
                        user_visible = true))]
    fn new(
        entity_id: i64,
        value: &Bound<'_, PyAny>,
        expression: Option<String>,
        user_visible: bool,
    ) -> PyResult<PyClassInitializer<Self>> {
        let (value, expression) = split_value(value, expression)?;
        Ok(
            PyClassInitializer::from(PyConstraintBase::default()).add_subclass(
                PyRadiusConstraint {
                    entity_id,
                    value,
                    user_visible,
                    status: 1,
                    expression,
                },
            ),
        )
    }

    /// Rewrites every id reference through *id_map*.
    fn remap_references(
        &mut self,
        id_map: std::collections::HashMap<i64, i64>,
    ) {
        super::remap_id(&mut self.entity_id, &id_map);
    }

    /// Returns the constraint error at the registry's point
    /// positions.
    #[pyo3(signature = (reg, params = None))]
    fn error(&self, reg: PointCtx<'_>, params: Option<Py<PyAny>>) -> f64 {
        let _ = params;
        self.clone()
            .into_constraint()
            .error(&reg)
            .first()
            .copied()
            .unwrap_or(0.0)
    }

    /// Partial derivatives of the error by point id.
    #[pyo3(signature = (reg, params = None))]
    fn gradient(
        &self,
        mut reg: PointCtx<'_>,
        params: Option<Py<PyAny>>,
    ) -> HashMap<u64, Vec<(f64, f64)>> {
        let _ = params;
        super::constraint_gradient(&self.clone().into_constraint(), &mut reg)
    }

    /// Point ids the constraint references, resolved through the
    /// registry.
    fn referenced_points(&self, registry: PointCtx<'_>) -> Vec<u64> {
        self.clone().into_constraint().referenced_points(&registry)
    }

    /// Serializes to the reference constraint dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict_type(&dict, "RadiusConstraint")?;
        dict.set_item("entity_id", self.entity_id)?;
        dict.set_item("value", self.value)?;
        dict.set_item("user_visible", self.user_visible)?;
        dict_expression(&dict, &self.expression)?;
        Ok(dict)
    }

    /// Rebuilds the constraint from its reference dictionary shape.
    #[classmethod]
    fn from_dict(
        cls: &Bound<'_, PyType>,
        py: Python<'_>,
        data: &Bound<'_, PyDict>,
    ) -> PyResult<Py<PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("entity_id", data.get_item("entity_id")?)?;
        kwargs.set_item("value", data.get_item("value")?)?;
        match data.get_item("expression")? {
            Some(value) => kwargs.set_item("expression", value)?,
            None => kwargs.set_item("expression", py.None())?,
        }
        kwargs.set_item(
            "user_visible",
            data.get_item("user_visible")?
                .and_then(|value| value.extract::<bool>().ok())
                .unwrap_or(true),
        )?;
        Ok(cls.as_any().call((), Some(&kwargs))?.unbind())
    }

    /// Draws the constraint's symbol onto the cairo context.
    #[pyo3(signature = (ctx, registry, to_screen, is_selected = false,
                        is_hovered = false, point_radius = 5.0))]
    #[allow(clippy::too_many_arguments)]
    fn draw<'py>(
        &self,
        ctx: Canvas<'py>,
        registry: &Bound<'py, PyAny>,
        to_screen: ScreenMap<'py>,
        is_selected: bool,
        is_hovered: bool,
        point_radius: f64,
    ) -> PyResult<()> {
        super::constraint_draw(
            &self.clone().into_constraint(),
            &ctx,
            registry,
            &to_screen,
            is_selected,
            is_hovered,
            point_radius,
            self.status,
        )
    }

    /// Whether this constraint restricts the given entity.
    fn targets_segment(
        &self,
        _p1: i64,
        _p2: i64,
        entity_id: Option<i64>,
    ) -> bool {
        entity_id.is_some_and(|eid| eid == self.entity_id)
    }

    /// Whether this constraint explicitly defines the radius of the
    /// given entity.
    fn constrains_radius(
        &self,
        _registry: PointCtx<'_>,
        entity_id: i64,
    ) -> bool {
        self.entity_id == entity_id
    }

    /// Whether the constraint's rendered symbol is hit at (sx, sy).
    #[pyo3(signature = (sx, sy, registry, to_screen, element,
                        threshold))]
    fn is_hit(
        &self,
        sx: f64,
        sy: f64,
        registry: PointCtx<'_>,
        to_screen: ScreenMap<'_>,
        element: Option<&Bound<'_, PyAny>>,
        threshold: f64,
    ) -> bool {
        let _ = element;
        let constraint = self.clone().into_constraint();
        super::constraint_is_hit(
            &constraint,
            sx,
            sy,
            threshold,
            &registry,
            Some(&to_screen),
        )
    }

    /// The string key identifying this constraint type.
    #[classmethod]
    fn get_type_key(_cls: &Bound<'_, PyType>) -> &'static str {
        "radius"
    }

    /// The human-readable type name (translated).
    #[staticmethod]
    fn get_type_name(py: Python<'_>) -> String {
        surface::gettext(py, "Radius")
    }

    /// Whether this constraint can be applied to the selection.
    #[classmethod]
    #[pyo3(signature = (selection, sketch = None))]
    fn can_apply_to(
        _cls: &Bound<'_, PyType>,
        selection: &Bound<'_, PyAny>,
        sketch: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let (point_ids, entity_ids) = surface::selection_ids(selection)?;
        if !point_ids.is_empty() || entity_ids.len() != 1 {
            return Ok(false);
        }
        let Some(entities) = surface::resolve_entities(selection, sketch)?
        else {
            return Ok(false);
        };
        Ok(applies_check(&entities))
    }

    /// Whether the operands are radius-bearing entities.
    #[classmethod]
    fn applies_to_entities(
        _cls: &Bound<'_, PyType>,
        entities: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let resolved = surface::entities_from_py(entities)?;
        Ok(applies_check(&resolved))
    }

    /// The human-readable title for this constraint.
    fn get_title(slf: &Bound<'_, Self>, _py: Python<'_>) -> PyResult<String> {
        surface::title_value(slf.as_any(), "")
    }

    /// The human-readable subtitle describing the constrained
    /// entity.
    fn get_subtitle(&self, py: Python<'_>, registry: PointCtx<'_>) -> String {
        let Some(entity) = registry.get_entity(self.entity_id as u64) else {
            return String::new();
        };
        let Some(center_id) = entity.center_point_id() else {
            return String::new();
        };
        let Some(center) = registry.get_point(center_id) else {
            return String::new();
        };
        let label = surface::entity_label(entity.as_ref());
        let coord = surface::format_coord(py, center.x, center.y);
        surface::gettext_format(py, "{} at {}", &[&label, &coord])
    }

    /// The user-facing hint for the constraint edit dialog.
    fn get_edit_subtitle(&self, py: Python<'_>) -> String {
        surface::gettext(py, "Enter radius or expression (e.g. 'width/2').")
    }
}

/// The reference `applies_to_entities` body: exactly one operand,
/// and it must be a radius-bearing entity.
fn applies_check(
    entities: &[Option<Box<dyn crate::sketch::entities::Entity>>],
) -> bool {
    entities.len() == 1
        && entities[0]
            .as_deref()
            .is_some_and(|entity| entity.is_radius_entity())
}

impl PyRadiusConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            entity_id: self.entity_id as u64,
            value: self.value,
            user_visible: self.user_visible,
            expression: self.expression,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRadiusConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.radius", m)?;
    Ok(())
}
