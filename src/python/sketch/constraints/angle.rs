//! Bindings for the angle constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::PyConstraintBase;

use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
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
    pub e1_id: u64,
    /// The other line entity.
    #[pyo3(get, set)]
    pub e2_id: u64,
    /// Far point of the anchor line.
    #[pyo3(get, set)]
    pub e1_far_idx: Option<u64>,
    /// Far point of the other line.
    #[pyo3(get, set)]
    pub e2_far_idx: Option<u64>,
    /// Angle value in degrees.
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
impl PyAngleConstraint {
    /// Creates the angle constraint between two lines.
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        e1_id,
        e2_id,
        value,
        user_visible = true,
        e1_far_idx = None,
        e2_far_idx = None,
    ))]
    fn new(
        e1_id: u64,
        e2_id: u64,
        value: f64,
        user_visible: bool,
        e1_far_idx: Option<u64>,
        e2_far_idx: Option<u64>,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase).add_subclass(
            PyAngleConstraint {
                e1_id,
                e2_id,
                e1_far_idx,
                e2_far_idx,
                value,
                user_visible,
                status: 1,
                expression: None,
            },
        )
    }

    /// Returns the constraint error at the registry's point
    /// positions.
    #[pyo3(signature = (registry, _params = None))]
    fn error(
        &self,
        registry: &PyEntityRegistry,
        _params: Option<Py<PyAny>>,
    ) -> Vec<f64> {
        self.clone().into_constraint().error(&registry.inner)
    }

    /// Partial derivatives of the error by point id.
    fn gradient(
        &self,
        registry: &PyEntityRegistry,
    ) -> std::collections::HashMap<u64, Vec<(f64, f64)>> {
        super::constraint_gradient(&self.clone().into_constraint(), registry)
    }

    /// Point ids the constraint references.
    fn referenced_points(&self, registry: &PyEntityRegistry) -> Vec<u64> {
        self.clone()
            .into_constraint()
            .referenced_points(&registry.inner)
    }

    /// Alias for referenced_points (reference naming).
    fn get_referenced_point_ids(
        &self,
        registry: &PyEntityRegistry,
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
        to_screen: Option<ScreenMap<'_>>,
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
    /// Mirrors the angle value (negates it).
    fn mirror(&mut self) {
        self.value = -self.value;
    }
}

impl PyAngleConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            e1_id: self.e1_id,
            e2_id: self.e2_id,
            e1_far_idx: self.e1_far_idx,
            e2_far_idx: self.e2_far_idx,
            value_deg: self.value,
            user_visible: self.user_visible,
            expression: self.expression,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAngleConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.angle", m)?;
    Ok(())
}
