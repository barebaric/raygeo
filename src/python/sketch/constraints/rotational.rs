//! Bindings for the rotational constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::rotational::RotationalConstraint as CoreConstraint;

/// Enforces p2 is p1 rotated by angle_rad about center.
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.rotational",
    name = "RotationalConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyRotationalConstraint {
    /// Rotation center.
    #[pyo3(get)]
    pub center: u64,
    /// Source point.
    #[pyo3(get)]
    pub p1: u64,
    /// Rotated point.
    #[pyo3(get)]
    pub p2: u64,
    /// Resolved rotation angle in radians.
    #[pyo3(get)]
    pub angle_rad: f64,
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
impl PyRotationalConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (center, p1, p2, angle_rad, user_visible = true))]
    fn new(
        center: u64,
        p1: u64,
        p2: u64,
        angle_rad: f64,
        user_visible: bool,
    ) -> Self {
        PyRotationalConstraint {
            center,
            p1,
            p2,
            angle_rad,
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

    /// Point ids the constraint references.
    fn referenced_points(&self) -> Vec<u64> {
        self.clone().into_constraint().referenced_points()
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
}

impl PyRotationalConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        let mut core =
            CoreConstraint::new(self.center, self.p1, self.p2, self.angle_rad);
        core.user_visible = self.user_visible;
        core.set_expression(self.expression);
        core
    }

    pub(crate) fn from_core(core: CoreConstraint) -> Self {
        PyRotationalConstraint {
            center: core.center,
            p1: core.p1,
            p2: core.p2,
            angle_rad: core.angle_rad,
            user_visible: core.user_visible,
            status: 0,
            expression: core.expression,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRotationalConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.rotational", m)?;
    Ok(())
}
