//! Bindings for the diameter constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::diameter::DiameterConstraint as CoreConstraint;

/// Enforces the diameter of a circle.
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.diameter",
    name = "DiameterConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyDiameterConstraint {
    /// Center point of the circle.
    #[pyo3(get)]
    pub center: u64,
    /// Radius-bearing point of the circle.
    #[pyo3(get)]
    pub radius_point: u64,
    /// Resolved diameter value.
    #[pyo3(get)]
    pub value: f64,
    /// Reference user-visibility flag.
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
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (center, radius_point, value, user_visible = true))]
    fn new(
        center: u64,
        radius_point: u64,
        value: f64,
        user_visible: bool,
    ) -> Self {
        PyDiameterConstraint {
            center,
            radius_point,
            value,
            user_visible,
            status: 0,
            expression: None,
        }
    }

    /// Returns the constraint error at the registry's point
    /// positions.
    fn error(&self, registry: &PyEntityRegistry) -> Vec<f64> {
        self.clone().into_constraint().error(&registry.inner)
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
        ctx: &Bound<'py, PyAny>,
        registry: &PyEntityRegistry,
        to_screen: &Bound<'py, PyAny>,
        is_selected: bool,
        is_hovered: bool,
        point_radius: f64,
    ) -> PyResult<()> {
        super::constraint_draw(
            &self.clone().into_constraint(),
            ctx,
            registry,
            to_screen,
            is_selected,
            is_hovered,
            point_radius,
            self.status,
        )
    }

    /// Reference user-visibility flag.
    fn user_visible(&self) -> bool {
        self.user_visible
    }

    /// Sets the reference user-visibility flag.
    fn set_user_visible(&mut self, value: bool) {
        self.user_visible = value;
    }

    /// The stored expression string, if any (evaluated caller-side).
    fn expression(&self) -> Option<String> {
        self.expression.clone()
    }

    /// Sets the expression string (evaluated caller-side).
    fn set_expression(&mut self, expression: Option<String>) {
        self.expression = expression;
    }

    /// Whether this constraint restricts the given entity.  The
    /// entity is matched through the registry by its defining
    /// points, mirroring the reference's stored circle id.
    #[pyo3(signature = (p1, p2, entity_id, registry))]
    fn targets_segment(
        &self,
        p1: u64,
        p2: u64,
        entity_id: Option<u64>,
        registry: &PyEntityRegistry,
    ) -> bool {
        let _ = (p1, p2);
        match entity_id {
            Some(id) => super::entity_contains_points(
                registry,
                id,
                &[self.center, self.radius_point],
            ),
            None => false,
        }
    }

    /// Whether this constraint explicitly defines the radius of the
    /// given entity.  The entity is matched through the registry by
    /// its defining points, mirroring the reference's stored circle
    /// id.
    fn constrains_radius(
        &self,
        registry: &PyEntityRegistry,
        entity_id: u64,
    ) -> bool {
        super::entity_contains_points(
            registry,
            entity_id,
            &[self.center, self.radius_point],
        )
    }

    /// Whether the constraint's rendered symbol is hit at (mx, my).
    #[pyo3(signature = (mx, my, threshold, registry, to_screen = None))]
    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        registry: &PyEntityRegistry,
        to_screen: Option<Bound<'_, PyAny>>,
    ) -> bool {
        let constraint = self.clone().into_constraint();
        super::constraint_is_hit(
            &constraint,
            mx,
            my,
            threshold,
            registry,
            to_screen.as_ref(),
        )
    }
}

impl PyDiameterConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        let mut core =
            CoreConstraint::new(self.center, self.radius_point, self.value);
        core.user_visible = self.user_visible;
        core.set_expression(self.expression);
        core
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDiameterConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.diameter", m)?;
    Ok(())
}
