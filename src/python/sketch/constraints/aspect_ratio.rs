//! Bindings for the aspect-ratio constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::aspect_ratio::AspectRatioConstraint as CoreConstraint;

/// Enforces dist(p1, p2) == ratio * dist(p3, p4).
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.aspect_ratio",
    name = "AspectRatioConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyAspectRatioConstraint {
    /// First point of the reference segment.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point of the reference segment.
    #[pyo3(get)]
    pub p2: u64,
    /// First point of the scaled segment.
    #[pyo3(get)]
    pub p3: u64,
    /// Second point of the scaled segment.
    #[pyo3(get)]
    pub p4: u64,
    /// Resolved ratio value.
    #[pyo3(get)]
    pub ratio: f64,
    /// Reference user-visibility flag.
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
    ) -> Self {
        PyAspectRatioConstraint {
            p1,
            p2,
            p3,
            p4,
            ratio,
            user_visible,
            status: 0,
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

    /// Sets the resolved ratio value.
    fn set_ratio(&mut self, ratio: f64) {
        self.ratio = ratio;
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

    pub(crate) fn from_core(core: CoreConstraint) -> Self {
        PyAspectRatioConstraint {
            p1: core.p1,
            p2: core.p2,
            p3: core.p3,
            p4: core.p4,
            ratio: core.ratio,
            user_visible: core.user_visible,
            status: 0,
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
