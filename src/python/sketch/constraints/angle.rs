//! Bindings for the angle constraint.

use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyType};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::radius::{dict_expression, dict_type, split_value};
use super::{surface, PyConstraintBase};
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, ScreenMap};
use crate::sketch::constraint::{PointContext, SketchConstraint};
use crate::sketch::constraints::angle::AngleConstraint as CoreConstraint;

/// Enforces a clockwise angle in degrees between two lines.  The
/// angle is measured at the lines' intersection between the rays
/// selected by `e1_far_idx`/`e2_far_idx` (endpoint ids; `None` picks
/// the endpoint farther from the intersection).
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.angle",
    name = "AngleConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyAngleConstraint {
    /// The anchor line entity.
    #[pyo3(get, set)]
    pub e1_id: i64,
    /// The other line entity.
    #[pyo3(get, set)]
    pub e2_id: i64,
    /// Far point of the anchor line.
    #[pyo3(get, set)]
    pub e1_far_idx: Option<i64>,
    /// Far point of the other line.
    #[pyo3(get, set)]
    pub e2_far_idx: Option<i64>,
    /// Angle value in degrees.
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
impl PyAngleConstraint {
    /// Creates the angle constraint between two lines.
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        e1_id,
        e2_id,
        value,
        expression = None,
        user_visible = true,
        e1_far_idx = None,
        e2_far_idx = None,
    ))]
    fn new(
        e1_id: i64,
        e2_id: i64,
        value: &Bound<'_, PyAny>,
        expression: Option<String>,
        user_visible: bool,
        e1_far_idx: Option<i64>,
        e2_far_idx: Option<i64>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let (value, expression) = split_value(value, expression)?;
        Ok(
            PyClassInitializer::from(PyConstraintBase::default()).add_subclass(
                PyAngleConstraint {
                    e1_id,
                    e2_id,
                    e1_far_idx,
                    e2_far_idx,
                    value,
                    user_visible,
                    status: 1,
                    expression,
                },
            ),
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

    /// The set of point ids this constraint references: the far
    /// points selected on the two lines.
    fn get_referenced_point_ids(&self) -> HashSet<i64> {
        let mut ids = HashSet::new();
        if let Some(id) = self.e1_far_idx {
            ids.insert(id);
        }
        if let Some(id) = self.e2_far_idx {
            ids.insert(id);
        }
        ids
    }

    /// Serializes to the reference constraint dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict_type(&dict, "AngleConstraint")?;
        dict.set_item("e1_id", self.e1_id)?;
        dict.set_item("e2_id", self.e2_id)?;
        dict.set_item("value", self.value)?;
        dict.set_item("user_visible", self.user_visible)?;
        match self.e1_far_idx {
            Some(id) => dict.set_item("e1_far_idx", id)?,
            None => dict.set_item("e1_far_idx", py.None())?,
        }
        match self.e2_far_idx {
            Some(id) => dict.set_item("e2_far_idx", id)?,
            None => dict.set_item("e2_far_idx", py.None())?,
        }
        dict_expression(&dict, &self.expression)?;
        Ok(dict)
    }

    /// Rebuilds the constraint from its reference dictionary shape.
    #[classmethod]
    #[allow(clippy::too_many_arguments)]
    fn from_dict(
        cls: &Bound<'_, PyType>,
        py: Python<'_>,
        data: &Bound<'_, PyDict>,
    ) -> PyResult<Py<PyAny>> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("e1_id", data.get_item("e1_id")?)?;
        kwargs.set_item("e2_id", data.get_item("e2_id")?)?;
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
        kwargs.set_item(
            "e1_far_idx",
            match data.get_item("e1_far_idx")? {
                Some(value) => value,
                None => py.None().into_bound(py),
            },
        )?;
        kwargs.set_item(
            "e2_far_idx",
            match data.get_item("e2_far_idx")? {
                Some(value) => value,
                None => py.None().into_bound(py),
            },
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

    /// Whether the constraint survives mirroring of all referenced
    /// geometry.  An expression-based angle encodes user intent that
    /// does not auto-flip under reflection.
    fn is_mirror_compatible(&self) -> bool {
        self.expression.is_none()
    }

    /// Mirrors the angle value: reflection reverses the signed
    /// (clockwise) angle, so the numeric value is negated.
    fn mirror(&mut self, _axis: &Bound<'_, PyAny>) {
        if self.expression.is_some() {
            return;
        }
        self.value = -self.value;
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
        "angle"
    }

    /// The human-readable type name (translated).
    #[staticmethod]
    fn get_type_name(py: Python<'_>) -> String {
        surface::gettext(py, "Angle")
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

    /// Whether both operands are lines.
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
        surface::title_value(slf.as_any(), "\u{00b0}")
    }

    /// The subtitle describing the constrained entities.
    fn get_subtitle(&self, py: Python<'_>, registry: PointCtx<'_>) -> String {
        let first = registry.get_entity(self.e1_id as u64);
        let second = registry.get_entity(self.e2_id as u64);
        let both_lines = [first, second].into_iter().all(|entity| {
            entity.is_some_and(|entity| entity.type_name() == "line")
        });
        if both_lines {
            return surface::gettext(py, "Between two lines");
        }
        String::new()
    }

    /// The user-facing hint for the constraint edit dialog.
    fn get_edit_subtitle(&self, py: Python<'_>) -> String {
        surface::gettext(py, "Enter angle in degrees or expression.")
    }
}

/// The reference `applies_to_entities` body: exactly two operands,
/// both lines.
fn applies_check(
    entities: &[Option<Box<dyn crate::sketch::entities::Entity>>],
) -> bool {
    entities.len() == 2
        && entities.iter().all(|entity| {
            entity.as_deref().is_some_and(|e| e.type_name() == "line")
        })
}

impl PyAngleConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            e1_id: self.e1_id as u64,
            e2_id: self.e2_id as u64,
            e1_far_idx: self.e1_far_idx.map(|id| id as u64),
            e2_far_idx: self.e2_far_idx.map(|id| id as u64),
            value_deg: self.value,
            user_visible: self.user_visible,
            expression: self.expression,
        }
    }
}

/// Draw layout constants, mirroring the reference `angle.py`.
pub(crate) const ARC_RADIUS: f64 = 35.0;
pub(crate) const LABEL_RADIUS: f64 = ARC_RADIUS - 10.0;

pyo3_stub_gen::module_variable!(
    "raygeo.sketch.constraints.angle",
    "ANGLE_WEIGHT",
    f64,
    10.0
);
pyo3_stub_gen::module_variable!(
    "raygeo.sketch.constraints.angle",
    "ARC_RADIUS",
    f64,
    35.0
);
pyo3_stub_gen::module_variable!(
    "raygeo.sketch.constraints.angle",
    "LABEL_RADIUS",
    f64,
    25.0
);

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAngleConstraint>()?;
    m.add("ANGLE_WEIGHT", super::ANGLE_WEIGHT)?;
    m.add("ARC_RADIUS", ARC_RADIUS)?;
    m.add("LABEL_RADIUS", LABEL_RADIUS)?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.angle", m)?;
    Ok(())
}
