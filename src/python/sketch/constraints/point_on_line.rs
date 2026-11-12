//! Bindings for the point-on-line constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyclass_complex_enum, gen_stub_pymethods,
};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::point_on_line::{
    PointOnLineConstraint as CoreConstraint, PointOnTarget,
};

/// The resolved target geometry: an infinite line, or a circle/arc
/// circumference.
#[gen_stub_pyclass_complex_enum]
#[pyclass(
    frozen,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.point_on_line",
    name = "PointOnTarget"
)]
#[derive(Clone, PartialEq)]
pub enum PyPointOnTarget {
    /// The infinite line through two points.
    Line { l1: u64, l2: u64 },
    /// The circle/arc through center and radius-bearing point.
    RadiusCurve { center: u64, radius_point: u64 },
}

impl From<&PyPointOnTarget> for PointOnTarget {
    fn from(value: &PyPointOnTarget) -> Self {
        match value {
            PyPointOnTarget::Line { l1, l2 } => {
                PointOnTarget::Line { l1: *l1, l2: *l2 }
            }
            PyPointOnTarget::RadiusCurve {
                center,
                radius_point,
            } => PointOnTarget::RadiusCurve {
                center: *center,
                radius_point: *radius_point,
            },
        }
    }
}

/// Enforces a point lies on a line or circle/arc circumference.
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.point_on_line",
    name = "PointOnLineConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyPointOnLineConstraint {
    /// The constrained point.
    #[pyo3(get)]
    pub point: u64,
    /// Resolved target geometry.
    #[pyo3(get)]
    pub target: PyPointOnTarget,
    /// Reference user-visibility flag.
    pub user_visible: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPointOnLineConstraint {
    /// Creates the point-on-line/curve constraint.
    #[new]
    #[pyo3(signature = (point, target, user_visible = true))]
    fn new(point: u64, target: PyPointOnTarget, user_visible: bool) -> Self {
        PyPointOnLineConstraint {
            point,
            target,
            user_visible,
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

    /// Reference user-visibility flag.
    fn user_visible(&self) -> bool {
        self.user_visible
    }

    /// Sets the reference user-visibility flag.
    fn set_user_visible(&mut self, value: bool) {
        self.user_visible = value;
    }

    /// Whether this constraint determines the target's radius: true
    /// when the target is a circle/arc circumference and the
    /// constrained point is itself solver-constrained.
    fn constrains_radius(
        &self,
        registry: &PyEntityRegistry,
        entity_id: u64,
    ) -> bool {
        let PyPointOnTarget::RadiusCurve {
            center,
            radius_point,
        } = &self.target
        else {
            return false;
        };
        if !super::entity_contains_points(
            registry,
            entity_id,
            &[*center, *radius_point],
        ) {
            return false;
        }
        registry
            .inner
            .get_point(self.point)
            .is_some_and(|point| point.constrained)
    }

    /// Whether the constraint's rendered symbol is hit at (mx, my).
    #[pyo3(signature = (mx, my, threshold, registry, to_screen = None))]
    fn is_hit(
        &self,
        mx: f64,
        my: f64,
        threshold: f64,
        registry: &PyEntityRegistry,
        to_screen: Option<Bound<'_, PyAny>>,
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

impl PyPointOnLineConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            user_visible: self.user_visible,
            ..CoreConstraint::new(self.point, PointOnTarget::from(&self.target))
        }
    }
}

/// Registers the constraint classes with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPointOnTarget>()?;
    m.add_class::<PyPointOnLineConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.point_on_line", m)?;
    Ok(())
}
