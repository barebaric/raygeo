//! Bindings for the point-on-line constraint.

use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::PyConstraintBase;
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::sketch::constraint::PointContext;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::point_on_line::PointOnLineConstraint as CoreConstraint;

/// Enforces a point lies on a line or circle/arc circumference.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.point_on_line",
    name = "PointOnLineConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyPointOnLineConstraint {
    /// The constrained point.
    #[pyo3(get, set)]
    pub point_id: i64,
    /// The target shape (line or radius-bearing entity).
    #[pyo3(get, set)]
    pub shape_id: i64,
    /// Reference user-visibility flag.
    #[pyo3(get)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPointOnLineConstraint {
    /// Creates the point-on-line/curve constraint.
    #[new]
    #[pyo3(signature = (point_id, shape_id, user_visible = true))]
    fn new(
        point_id: i64,
        shape_id: i64,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase).add_subclass(
            PyPointOnLineConstraint {
                point_id,
                shape_id,
                user_visible,
                status: 1,
            },
        )
    }

    /// Returns the constraint error at the registry's point
    /// positions.
    #[pyo3(signature = (registry, _params = None))]
    fn error(
        &self,
        registry: PointCtx<'_>,
        _params: Option<Py<PyAny>>,
    ) -> Vec<f64> {
        self.clone().into_constraint().error(&registry)
    }

    /// Partial derivatives of the error by point id.
    fn gradient(
        &self,
        mut registry: PointCtx<'_>,
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
    fn get_referenced_point_ids(&self, registry: PointCtx<'_>) -> HashSet<u64> {
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

    /// Whether this constraint determines the target's radius: true
    /// when the target is the given entity and the constrained point
    /// is itself solver-constrained.
    fn constrains_radius(
        &self,
        registry: PointCtx<'_>,
        entity_id: i64,
    ) -> bool {
        if self.shape_id != entity_id as i64 {
            return false;
        }
        registry
            .get_point(self.point_id as u64)
            .is_some_and(|point| point.constrained)
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

impl PyPointOnLineConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            point_id: self.point_id as u64,
            shape_id: self.shape_id as u64,
            user_visible: self.user_visible,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPointOnLineConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.point_on_line", m)?;
    Ok(())
}
