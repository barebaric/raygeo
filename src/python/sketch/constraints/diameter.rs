//! Bindings for the diameter constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::PyConstraintBase;

use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::diameter::DiameterConstraint as CoreConstraint;

/// Enforces the diameter of a circle.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.diameter",
    name = "DiameterConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyDiameterConstraint {
    /// The constrained circle.
    #[pyo3(get, set)]
    pub circle_id: i64,
    /// Resolved diameter value.
    #[pyo3(get, set)]
    pub value: f64,
    /// Reference user-visibility flag.
    #[pyo3(get)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
    /// Stored expression string (evaluated caller-side).
    pub expression: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDiameterConstraint {
    /// Creates the constraint for a circle.
    #[new]
    #[pyo3(signature = (circle_id, value, user_visible = true))]
    fn new(
        circle_id: i64,
        value: f64,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase).add_subclass(
            PyDiameterConstraint {
                circle_id,
                value,
                user_visible,
                status: 1,
                expression: None,
            },
        )
    }

    /// Returns the constraint error at the registry's point
    /// positions.
    #[pyo3(signature = (registry, _ctx = None))]
    fn error(
        &self,
        registry: PointCtx<'_>,
        _ctx: Option<Py<PyAny>>,
    ) -> Vec<f64> {
        self.clone().into_constraint().error(&registry)
    }

    /// Partial derivatives of the error by point id.
    fn gradient(
        &self,
        mut registry: PointCtx<'_>,
    ) -> std::collections::HashMap<u64, Vec<(f64, f64)>> {
        super::constraint_gradient(
            &self.clone().into_constraint(),
            &mut registry,
        )
    }

    /// Point ids the constraint references.
    fn referenced_points(&self, registry: PointCtx<'_>) -> Vec<u64> {
        self.clone().into_constraint().referenced_points(&registry)
    }

    /// Alias for referenced_points (reference naming).
    fn get_referenced_point_ids(
        &self,
        registry: PointCtx<'_>,
    ) -> std::collections::HashSet<u64> {
        self.referenced_points(registry).into_iter().collect()
    }

    /// Serializes to the reference constraint dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        super::dict_from_value(py, &self.clone().into_constraint().to_json())
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

    /// Sets the reference user-visibility flag.
    fn set_user_visible(&mut self, value: bool) {
        self.user_visible = value;
    }

    /// The stored expression string, if any (evaluated caller-side).
    #[getter]
    fn expression(&self) -> Option<String> {
        self.expression.clone()
    }

    /// Sets the expression string (evaluated caller-side).
    #[setter]
    fn set_expression(&mut self, expression: Option<String>) {
        self.expression = expression;
    }

    /// Whether this constraint restricts the given entity.
    fn targets_segment(
        &self,
        _p1: i64,
        _p2: i64,
        entity_id: Option<i64>,
    ) -> bool {
        entity_id.is_some_and(|eid| eid == self.circle_id)
    }

    /// Whether this constraint explicitly defines the radius of the
    /// given entity.
    fn constrains_radius(
        &self,
        _registry: PointCtx<'_>,
        entity_id: i64,
    ) -> bool {
        self.circle_id == entity_id
    }

    /// Whether the constraint's rendered symbol is hit at (mx, my).
    #[pyo3(signature = (mx, my, threshold, registry, to_screen = None))]
    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        registry: PointCtx<'_>,
        to_screen: Option<ScreenMap<'_>>,
    ) -> bool {
        let constraint = self.clone().into_constraint();
        super::constraint_is_hit(
            &constraint,
            mx,
            my,
            threshold,
            &registry,
            to_screen.as_ref(),
        )
    }
}

impl PyDiameterConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            circle_id: self.circle_id as u64,
            value: self.value,
            user_visible: self.user_visible,
            expression: self.expression,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDiameterConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.diameter", m)?;
    Ok(())
}
