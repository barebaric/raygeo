//! Bindings for the drag constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::PyConstraintBase;

use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
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
        PyClassInitializer::from(PyConstraintBase).add_subclass(
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

    /// Sets the target coordinates and pull strength.
    fn set_target(&mut self, target_x: f64, target_y: f64, weight: f64) {
        self.target_x = target_x;
        self.target_y = target_y;
        self.weight = weight;
    }

    /// Returns the constraint error at the registry's point positions.
    #[pyo3(signature = (registry, _params = None))]
    fn error(
        &self,
        registry: PointCtx<'_>,
        _params: Option<Py<PyAny>>,
    ) -> Vec<f64> {
        self.clone().into_constraint().error(&registry)
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
