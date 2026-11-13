//! Bindings for the drag constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::sketch::constraints::drag::DragConstraint as CoreConstraint;

/// Softly pulls a point towards a target coordinate.
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.drag",
    name = "DragConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyDragConstraint {
    /// The dragged point.
    #[pyo3(get)]
    pub point: u64,
    /// Target X coordinate.
    #[pyo3(get)]
    pub target_x: f64,
    /// Target Y coordinate.
    #[pyo3(get)]
    pub target_y: f64,
    /// Pull strength.
    #[pyo3(get)]
    pub weight: f64,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDragConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (point, target_x, target_y, weight))]
    fn new(point: u64, target_x: f64, target_y: f64, weight: f64) -> Self {
        PyDragConstraint {
            point,
            target_x,
            target_y,
            weight,
            status: 0,
        }
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
}

impl PyDragConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(
            self.point,
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
