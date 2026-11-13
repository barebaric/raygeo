//! Bindings for the perpendicular constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::perpendicular::{
    PerpendicularConstraint as CoreConstraint, PerpendicularTarget,
};

/// The resolved geometry of the two perpendicular operands: two
/// lines, a line through a circle/arc center, or two circles meeting
/// at a right angle.
#[gen_stub_pyclass_complex_enum]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.perpendicular",
    name = "PerpendicularTarget"
)]
#[derive(Clone, PartialEq)]
pub enum PyPerpendicularTarget {
    /// Line-line: direction dot product.
    LineLine { a1: u64, a2: u64, b1: u64, b2: u64 },
    /// Line and circle/arc: the line passes through the center.
    LineCenter { l1: u64, l2: u64, center: u64 },
    /// Circle/arc pair: r1² + r2² == dist(c1, c2)².
    Circles {
        c1: u64,
        r1_point: u64,
        c2: u64,
        r2_point: u64,
    },
}

impl From<&PyPerpendicularTarget> for PerpendicularTarget {
    fn from(value: &PyPerpendicularTarget) -> Self {
        match value {
            PyPerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                PerpendicularTarget::LineLine {
                    a1: *a1,
                    a2: *a2,
                    b1: *b1,
                    b2: *b2,
                }
            }
            PyPerpendicularTarget::LineCenter { l1, l2, center } => {
                PerpendicularTarget::LineCenter {
                    l1: *l1,
                    l2: *l2,
                    center: *center,
                }
            }
            PyPerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => PerpendicularTarget::Circles {
                c1: *c1,
                r1_point: *r1_point,
                c2: *c2,
                r2_point: *r2_point,
            },
        }
    }
}

impl From<&PerpendicularTarget> for PyPerpendicularTarget {
    fn from(value: &PerpendicularTarget) -> Self {
        match value {
            PerpendicularTarget::LineLine { a1, a2, b1, b2 } => {
                PyPerpendicularTarget::LineLine {
                    a1: *a1,
                    a2: *a2,
                    b1: *b1,
                    b2: *b2,
                }
            }
            PerpendicularTarget::LineCenter { l1, l2, center } => {
                PyPerpendicularTarget::LineCenter {
                    l1: *l1,
                    l2: *l2,
                    center: *center,
                }
            }
            PerpendicularTarget::Circles {
                c1,
                r1_point,
                c2,
                r2_point,
            } => PyPerpendicularTarget::Circles {
                c1: *c1,
                r1_point: *r1_point,
                c2: *c2,
                r2_point: *r2_point,
            },
        }
    }
}

/// Enforces perpendicularity between two entities.
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.perpendicular",
    name = "PerpendicularConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyPerpendicularConstraint {
    /// Resolved operand geometry.
    #[pyo3(get)]
    pub target: PyPerpendicularTarget,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPerpendicularConstraint {
    /// Creates the constraint from resolved operands.
    #[new]
    #[pyo3(signature = (target, user_visible = true))]
    fn new(target: PyPerpendicularTarget, user_visible: bool) -> Self {
        PyPerpendicularConstraint {
            target,
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

impl PyPerpendicularConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            user_visible: self.user_visible,
            ..CoreConstraint::new(PerpendicularTarget::from(&self.target))
        }
    }

    pub(crate) fn from_core(core: CoreConstraint) -> Self {
        PyPerpendicularConstraint {
            target: PyPerpendicularTarget::from(&core.target),
            user_visible: core.user_visible,
            status: 0,
        }
    }
}

/// Registers the constraint classes with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPerpendicularTarget>()?;
    m.add_class::<PyPerpendicularConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.perpendicular", m)?;
    Ok(())
}
