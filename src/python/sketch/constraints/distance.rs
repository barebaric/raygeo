//! Bindings for the distance constraint.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyType};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::radius::{dict_expression, dict_type, split_value};
use super::{surface, PyConstraintBase};
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::sketch::constraint::{PointContext, SketchConstraint};
use crate::sketch::constraints::distance::DistanceConstraint as CoreConstraint;

/// Enforces a fixed distance between two points.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.distance",
    name = "DistanceConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyDistanceConstraint {
    /// First point.
    #[pyo3(get, set)]
    pub p1: i64,
    /// Second point.
    #[pyo3(get, set)]
    pub p2: i64,
    /// Resolved distance value.
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

#[gen_stub_pymethods]
#[pymethods]
impl PyDistanceConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p1, p2, value, expression = None,
                        user_visible = true))]
    fn new(
        p1: i64,
        p2: i64,
        value: &Bound<'_, PyAny>,
        expression: Option<String>,
        user_visible: bool,
    ) -> PyResult<PyClassInitializer<Self>> {
        let (value, expression) = split_value(value, expression)?;
        Ok(PyClassInitializer::from(PyConstraintBase).add_subclass(
            PyDistanceConstraint {
                p1,
                p2,
                value,
                user_visible,
                status: 1,
                expression,
            },
        ))
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

    /// Point ids the constraint references.
    fn referenced_points(&self, registry: PointCtx<'_>) -> Vec<u64> {
        self.clone().into_constraint().referenced_points(&registry)
    }

    /// Serializes to the reference constraint dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict_type(&dict, "DistanceConstraint")?;
        dict.set_item("p1", self.p1)?;
        dict.set_item("p2", self.p2)?;
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
        kwargs.set_item("p1", data.get_item("p1")?)?;
        kwargs.set_item("p2", data.get_item("p2")?)?;
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
        registry: DrawContext<'py>,
        to_screen: ScreenMap<'py>,
        is_selected: bool,
        is_hovered: bool,
        point_radius: f64,
    ) -> PyResult<()> {
        super::constraint_draw(
            &self.clone().into_constraint(),
            &ctx,
            &registry,
            &to_screen,
            is_selected,
            is_hovered,
            point_radius,
            self.status,
        )
    }

    /// Whether this constraint defines the length/distance of the
    /// segment (p1, p2).  The entity id is ignored: distance
    /// constraints track points only.
    #[pyo3(signature = (p1, p2, _entity_id = None))]
    fn targets_segment(
        &self,
        p1: i64,
        p2: i64,
        _entity_id: Option<i64>,
    ) -> bool {
        (p1 == self.p1 && p2 == self.p2) || (p1 == self.p2 && p2 == self.p1)
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
        "dist"
    }

    /// The human-readable type name (translated).
    #[staticmethod]
    fn get_type_name(py: Python<'_>) -> String {
        surface::gettext(py, "Distance")
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
        if point_ids.len() == 2 && entity_ids.is_empty() {
            return Ok(true);
        }
        if !point_ids.is_empty() {
            return Ok(false);
        }
        let Some(entities) = surface::resolve_entities(selection, sketch)?
        else {
            return Ok(false);
        };
        Ok(applies_check(&entities))
    }

    /// Whether the operand is a line.
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

    /// The subtitle describing the constrained points.
    fn get_subtitle(&self, py: Python<'_>, registry: PointCtx<'_>) -> String {
        let points = |id: i64| -> Option<(f64, f64)> {
            registry
                .get_point(id as u64)
                .map(|point| (point.x, point.y))
        };
        surface::subtitle_from_to(py, &points, self.p1, self.p2)
    }

    /// The user-facing hint for the constraint edit dialog.
    fn get_edit_subtitle(&self, py: Python<'_>) -> String {
        surface::gettext(py, "Enter length or expression.")
    }
}

/// The reference `applies_to_entities` body: exactly one operand,
/// and it must be a line.
fn applies_check(
    entities: &[Option<Box<dyn crate::sketch::entities::Entity>>],
) -> bool {
    entities.len() == 1
        && entities[0]
            .as_deref()
            .is_some_and(|entity| entity.type_name() == "line")
}

impl PyDistanceConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        let mut core =
            CoreConstraint::new(self.p1 as u64, self.p2 as u64, self.value);
        core.user_visible = self.user_visible;
        core.set_expression(self.expression);
        core
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDistanceConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.distance", m)?;
    Ok(())
}
