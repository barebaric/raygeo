//! Bindings for the drag constraint.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::PyConstraintBase;
use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, ScreenMap};
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::drag::DragConstraint as CoreConstraint;

/// Softly pulls a point towards a target coordinate.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.drag",
    name = "DragConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyDragConstraint {
    /// The dragged point.
    #[pyo3(get, set)]
    pub point_id: i64,
    /// Target X coordinate.
    #[pyo3(get, set)]
    pub target_x: f64,
    /// Target Y coordinate.
    #[pyo3(get, set)]
    pub target_y: f64,
    /// Pull strength.
    #[pyo3(get, set)]
    pub weight: f64,
    /// Reference user-visibility flag.
    #[pyo3(get, set)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDragConstraint {
    /// Creates the drag constraint for a point and target position.
    #[new]
    #[pyo3(signature = (point_id, target_x, target_y, weight = 0.1,
                        user_visible = true))]
    fn new(
        point_id: i64,
        target_x: f64,
        target_y: f64,
        weight: f64,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase::default()).add_subclass(
            PyDragConstraint {
                point_id,
                target_x,
                target_y,
                weight,
                user_visible,
                status: 1,
            },
        )
    }

    /// Rewrites every id reference through *id_map*.
    fn remap_references(
        &mut self,
        id_map: std::collections::HashMap<i64, i64>,
    ) {
        super::remap_id(&mut self.point_id, &id_map);
    }

    /// Returns the constraint error at the registry's point positions.
    #[pyo3(signature = (reg, params = None))]
    fn error(
        &self,
        reg: PointCtx<'_>,
        params: Option<Py<PyAny>>,
    ) -> (f64, f64) {
        let _ = params;
        let errors = self.clone().into_constraint().error(&reg);
        (
            errors.first().copied().unwrap_or(0.0),
            errors.get(1).copied().unwrap_or(0.0),
        )
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

    /// Sets the target coordinates and pull strength.
    fn set_target(&mut self, target_x: f64, target_y: f64, weight: f64) {
        self.target_x = target_x;
        self.target_y = target_y;
        self.weight = weight;
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

    /// Deep-copies the transient constraint structurally: the
    /// reference drag constraint has no serialization shape.
    fn __deepcopy__<'py>(
        &self,
        py: Python<'py>,
        memo: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let _ = memo;
        let clone = Py::new(
            py,
            PyClassInitializer::from(PyConstraintBase::default())
                .add_subclass(self.clone()),
        )?;
        Ok(clone.into_bound(py).into_any())
    }

    fn __copy__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Self::__deepcopy__(self, py, &PyDict::new(py))
    }
}

impl PyDragConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(
            self.point_id as u64,
            self.target_x,
            self.target_y,
            self.weight,
        )
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDragConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.drag", m)?;
    Ok(())
}
