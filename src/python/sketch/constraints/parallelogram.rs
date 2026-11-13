//! Bindings for the parallelogram constraint.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::sketch::constraints::parallelogram::ParallelogramConstraint as CoreConstraint;

/// Enforces (p_width - p_origin) == (p4 - p_height).
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.parallelogram",
    name = "ParallelogramConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyParallelogramConstraint {
    /// Origin point.
    #[pyo3(get)]
    pub p_origin: u64,
    /// Width point.
    #[pyo3(get)]
    pub p_width: u64,
    /// Height point.
    #[pyo3(get)]
    pub p_height: u64,
    /// Fourth corner.
    #[pyo3(get)]
    pub p4: u64,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyParallelogramConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p_origin, p_width, p_height, p4))]
    fn new(p_origin: u64, p_width: u64, p_height: u64, p4: u64) -> Self {
        PyParallelogramConstraint {
            p_origin,
            p_width,
            p_height,
            p4,
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

impl PyParallelogramConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint::new(self.p_origin, self.p_width, self.p_height, self.p4)
    }

    pub(crate) fn from_core(core: CoreConstraint) -> Self {
        PyParallelogramConstraint {
            p_origin: core.p_origin,
            p_width: core.p_width,
            p_height: core.p_height,
            p4: core.p4,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyParallelogramConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.parallelogram", m)?;
    Ok(())
}
