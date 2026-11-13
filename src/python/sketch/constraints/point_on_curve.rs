//! Bindings for the point-on-curve constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::point_on_curve::PointOnCurveConstraint as CoreConstraint;

/// Enforces a point lies on a cubic Bezier curve.  The control
/// points are stored as offsets from their owning endpoints; curves
/// without control points use the straight-segment residual
/// (`is_line`).
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.point_on_curve",
    name = "PointOnCurveConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyPointOnCurveConstraint {
    /// The constrained point.
    #[pyo3(get)]
    pub point: u64,
    /// Curve start point.
    #[pyo3(get)]
    pub start: u64,
    /// Curve end point.
    #[pyo3(get)]
    pub end: u64,
    /// First control point as offset from the start point.
    #[pyo3(get)]
    pub cp1_offset: (f64, f64),
    /// Second control point as offset from the end point.
    #[pyo3(get)]
    pub cp2_offset: (f64, f64),
    /// Whether the curve degenerates to a straight segment.
    #[pyo3(get)]
    pub is_line: bool,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPointOnCurveConstraint {
    /// Creates the point-on-curve constraint.
    #[new]
    #[pyo3(signature = (
        point,
        start,
        end,
        cp1_offset,
        cp2_offset,
        is_line,
        user_visible = true,
    ))]
    fn new(
        point: u64,
        start: u64,
        end: u64,
        cp1_offset: (f64, f64),
        cp2_offset: (f64, f64),
        is_line: bool,
        user_visible: bool,
    ) -> Self {
        PyPointOnCurveConstraint {
            point,
            start,
            end,
            cp1_offset,
            cp2_offset,
            is_line,
            user_visible,
            status: 0,
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
}

impl PyPointOnCurveConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        let mut core = CoreConstraint::new(
            self.point,
            self.start,
            self.end,
            [self.cp1_offset.0, self.cp1_offset.1],
            [self.cp2_offset.0, self.cp2_offset.1],
            self.is_line,
        );
        core.user_visible = self.user_visible;
        core
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPointOnCurveConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.point_on_curve", m)?;
    Ok(())
}
