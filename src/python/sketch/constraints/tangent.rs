//! Bindings for the tangent constraint.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyType};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::radius::dict_type;
use super::{surface, PyConstraintBase};
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, ScreenMap};
use crate::sketch::constraint::{PointContext, SketchConstraint};
use crate::sketch::constraints::tangent::TangentConstraint as CoreConstraint;

/// Enforces a line is tangent to a circle/arc.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.tangent",
    name = "TangentConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyTangentConstraint {
    /// The tangent line.
    #[pyo3(get, set)]
    pub line_id: i64,
    /// The tangent circle or arc.
    #[pyo3(get, set)]
    pub shape_id: i64,
    /// Reference user-visibility flag.
    #[pyo3(get, set)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTangentConstraint {
    /// Creates the constraint between a line and a circle/arc.
    #[new]
    #[pyo3(signature = (line_id, shape_id, user_visible = true))]
    fn new(
        line_id: i64,
        shape_id: i64,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase::default()).add_subclass(
            PyTangentConstraint {
                line_id,
                shape_id,
                user_visible,
                status: 1,
            },
        )
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
        dict_type(&dict, "TangentConstraint")?;
        dict.set_item("line_id", self.line_id)?;
        dict.set_item("shape_id", self.shape_id)?;
        dict.set_item("user_visible", self.user_visible)?;
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
        kwargs.set_item("line_id", data.get_item("line_id")?)?;
        kwargs.set_item("shape_id", data.get_item("shape_id")?)?;
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

    /// The string key identifying this constraint type.
    #[classmethod]
    fn get_type_key(_cls: &Bound<'_, PyType>) -> &'static str {
        "tangent"
    }

    /// The human-readable type name (translated).
    #[staticmethod]
    fn get_type_name(py: Python<'_>) -> String {
        surface::gettext(py, "Tangent")
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
        if !point_ids.is_empty() || entity_ids.len() != 2 {
            return Ok(false);
        }
        let Some(entities) = surface::resolve_entities(selection, sketch)?
        else {
            return Ok(false);
        };
        Ok(applies_check(&entities))
    }

    /// Whether one operand is a line and the other radius-bearing.
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
        surface::title_plain(slf.as_any())
    }

    /// The subtitle describing the constrained entities.
    fn get_subtitle(&self, py: Python<'_>, registry: PointCtx<'_>) -> String {
        let line = registry.get_entity(self.line_id as u64);
        let shape = registry.get_entity(self.shape_id as u64);
        let (Some(line), Some(shape)) = (line, shape) else {
            return String::new();
        };
        if line.type_name() != "line" || !shape.is_radius_entity() {
            return String::new();
        }
        let Some(center_id) = shape.center_point_id() else {
            return String::new();
        };
        let Some(center) = registry.get_point(center_id) else {
            return String::new();
        };
        let label = surface::entity_label(shape.as_ref());
        let coord = surface::format_coord(py, center.x, center.y);
        surface::gettext_format(py, "Line to {} at {}", &[&label, &coord])
    }

    /// Whether the constraint's rendered symbol is hit at (mx, my).
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
}

/// The reference `applies_to_entities` body: exactly two operands;
/// one must be a line, the other radius-bearing.
fn applies_check(
    entities: &[Option<Box<dyn crate::sketch::entities::Entity>>],
) -> bool {
    if entities.len() != 2 {
        return false;
    }
    let (Some(first), Some(second)) =
        (entities[0].as_deref(), entities[1].as_deref())
    else {
        return false;
    };
    let is_line = |entity: &dyn crate::sketch::entities::Entity| {
        entity.type_name() == "line"
    };
    if is_line(first) {
        return second.is_radius_entity();
    }
    is_line(second) && first.is_radius_entity()
}

impl PyTangentConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            line_id: self.line_id as u64,
            shape_id: self.shape_id as u64,
            user_visible: self.user_visible,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTangentConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.tangent", m)?;
    Ok(())
}
