//! Bindings for the angle constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::angle::AngleConstraint as CoreConstraint;

/// Enforces a clockwise angle in degrees between two lines.  The
/// angle is measured at the lines' intersection between the rays
/// selected by `e1_far`/`e2_far` (endpoint ids; `None` picks the
/// endpoint farther from the intersection).
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.angle",
    name = "AngleConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyAngleConstraint {
    /// First point of the anchor line.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point of the anchor line.
    #[pyo3(get)]
    pub p2: u64,
    /// First point of the other line.
    #[pyo3(get)]
    pub p3: u64,
    /// Second point of the other line.
    #[pyo3(get)]
    pub p4: u64,
    /// Far point of the anchor line.
    #[pyo3(get)]
    pub e1_far: Option<u64>,
    /// Far point of the other line.
    #[pyo3(get)]
    pub e2_far: Option<u64>,
    /// Resolved angle value in degrees.
    #[pyo3(get)]
    pub value_deg: f64,
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
impl PyAngleConstraint {
    /// Creates the angle constraint from its resolved points and
    /// value.
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        p1,
        p2,
        p3,
        p4,
        value_deg,
        e1_far = None,
        e2_far = None,
        user_visible = true,
    ))]
    fn new(
        p1: u64,
        p2: u64,
        p3: u64,
        p4: u64,
        value_deg: f64,
        e1_far: Option<u64>,
        e2_far: Option<u64>,
        user_visible: bool,
    ) -> Self {
        PyAngleConstraint {
            p1,
            p2,
            p3,
            p4,
            e1_far,
            e2_far,
            value_deg,
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
        registry: &Bound<'py, PyAny>,
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

    /// Whether the constraint survives mirroring of all referenced
    /// geometry.  An expression-based angle encodes user intent that
    /// does not auto-flip under reflection.
    fn is_mirror_compatible(&self) -> bool {
        self.expression.is_none()
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

impl PyAngleConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        let mut core = CoreConstraint::new(
            self.p1,
            self.p2,
            self.p3,
            self.p4,
            self.e1_far,
            self.e2_far,
            self.value_deg,
        );
        core.user_visible = self.user_visible;
        core.set_expression(self.expression);
        core
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAngleConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.angle", m)?;
    Ok(())
}
