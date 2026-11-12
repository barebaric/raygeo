//! Bindings for the distance constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::distance::DistanceConstraint as CoreConstraint;

/// Enforces a fixed distance between two points.
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.distance",
    name = "DistanceConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyDistanceConstraint {
    /// First point.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point.
    #[pyo3(get)]
    pub p2: u64,
    /// Resolved distance value.
    #[pyo3(get)]
    pub value: f64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Stored expression string (evaluated caller-side).
    pub expression: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDistanceConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p1, p2, value, user_visible = true))]
    fn new(p1: u64, p2: u64, value: f64, user_visible: bool) -> Self {
        PyDistanceConstraint {
            p1,
            p2,
            value,
            user_visible,
            expression: None,
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

    /// The stored expression string, if any (evaluated caller-side).
    fn expression(&self) -> Option<String> {
        self.expression.clone()
    }

    /// Sets the expression string (evaluated caller-side).
    fn set_expression(&mut self, expression: Option<String>) {
        self.expression = expression;
    }

    /// Whether this constraint defines the length/distance of the
    /// segment (p1, p2).  The entity id is ignored: distance
    /// constraints track points only.
    #[pyo3(signature = (p1, p2, entity_id, registry))]
    fn targets_segment(
        &self,
        p1: u64,
        p2: u64,
        entity_id: Option<u64>,
        registry: &PyEntityRegistry,
    ) -> bool {
        let _ = (entity_id, registry);
        (p1 == self.p1 && p2 == self.p2) || (p1 == self.p2 && p2 == self.p1)
    }

    /// Whether this constraint explicitly defines the radius/length
    /// of the given entity.
    fn constrains_radius(
        &self,
        registry: &PyEntityRegistry,
        entity_id: u64,
    ) -> bool {
        let _ = (registry, entity_id);
        false
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

impl PyDistanceConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        let mut core = CoreConstraint::new(self.p1, self.p2, self.value);
        core.user_visible = self.user_visible;
        core.set_expression(self.expression);
        core
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDistanceConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.distance", m)?;
    Ok(())
}
