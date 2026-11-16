//! Bindings for the aspect-ratio constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::PyConstraintBase;

use crate::python::sketch::context::PointCtx;
use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::aspect_ratio::AspectRatioConstraint as CoreConstraint;

/// Enforces dist(p1, p2) == ratio * dist(p3, p4).
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    subclass,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.aspect_ratio",
    name = "AspectRatioConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyAspectRatioConstraint {
    /// First point of the reference segment.
    #[pyo3(get, set)]
    pub p1: u64,
    /// Second point of the reference segment.
    #[pyo3(get, set)]
    pub p2: u64,
    /// First point of the scaled segment.
    #[pyo3(get, set)]
    pub p3: u64,
    /// Second point of the scaled segment.
    #[pyo3(get, set)]
    pub p4: u64,
    /// Resolved ratio value.
    #[pyo3(get, set)]
    pub ratio: f64,
    /// Reference user-visibility flag.
    #[pyo3(get)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAspectRatioConstraint {
    /// Creates the aspect-ratio constraint from its resolved points
    /// and ratio.
    #[new]
    #[pyo3(signature = (p1, p2, p3, p4, ratio, user_visible = true))]
    fn new(
        p1: u64,
        p2: u64,
        p3: u64,
        p4: u64,
        ratio: f64,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase).add_subclass(
            PyAspectRatioConstraint {
                p1,
                p2,
                p3,
                p4,
                ratio,
                user_visible,
                status: 1,
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

    /// Sets the reference user-visibility flag.
    fn set_user_visible(&mut self, value: bool) {
        self.user_visible = value;
    }
}

impl PyAspectRatioConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            user_visible: self.user_visible,
            ..CoreConstraint::new(
                self.p1, self.p2, self.p3, self.p4, self.ratio,
            )
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAspectRatioConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.aspect_ratio", m)?;
    Ok(())
}
