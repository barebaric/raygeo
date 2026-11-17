//! Bindings for the rotational constraint.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::PyConstraintBase;
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::rotational::RotationalConstraint as CoreConstraint;

/// Enforces p2 is p1 rotated by angle_rad about center.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.rotational",
    name = "RotationalConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyRotationalConstraint {
    /// Rotation center.
    #[pyo3(get, set)]
    pub center: i64,
    /// Source point.
    #[pyo3(get, set)]
    pub p1: i64,
    /// Rotated point.
    #[pyo3(get, set)]
    pub p2: i64,
    /// Rotation angle in radians (the reference ``value`` attribute).
    #[pyo3(get, set)]
    pub angle_rad: f64,
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
impl PyRotationalConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (center, p1, p2, angle_rad, user_visible = true))]
    fn new(
        center: i64,
        p1: i64,
        p2: i64,
        angle_rad: f64,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase).add_subclass(
            PyRotationalConstraint {
                center,
                p1,
                p2,
                angle_rad,
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
    #[pyo3(signature = (registry, _params = None))]
    fn gradient(
        &self,
        mut registry: PointCtx<'_>,
        _params: Option<Py<PyAny>>,
    ) -> HashMap<u64, Vec<(f64, f64)>> {
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

    /// Sets the reference user-visibility flag.
    fn set_user_visible(&mut self, value: bool) {
        self.user_visible = value;
    }

    /// The rotation angle in radians (reference ``value`` attribute).
    #[getter]
    fn value(&self) -> f64 {
        self.angle_rad
    }

    #[setter]
    fn set_value(&mut self, value: f64) {
        self.angle_rad = value;
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
}

impl PyRotationalConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        let mut core = CoreConstraint::new(
            self.center as u64,
            self.p1 as u64,
            self.p2 as u64,
            self.angle_rad,
        );
        core.user_visible = self.user_visible;
        core.set_expression(self.expression);
        core
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRotationalConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.rotational", m)?;
    Ok(())
}
