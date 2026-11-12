//! Bindings for the equal-distance constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::equal_distance::EqualDistanceConstraint as CoreConstraint;

/// Enforces dist(p1, p2) == dist(p3, p4).
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.equal_distance",
    name = "EqualDistanceConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyEqualDistanceConstraint {
    /// First point of the first pair.
    #[pyo3(get)]
    pub p1: u64,
    /// Second point of the first pair.
    #[pyo3(get)]
    pub p2: u64,
    /// First point of the second pair.
    #[pyo3(get)]
    pub p3: u64,
    /// Second point of the second pair.
    #[pyo3(get)]
    pub p4: u64,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEqualDistanceConstraint {
    /// Creates the constraint from its resolved points and values.
    #[new]
    #[pyo3(signature = (p1, p2, p3, p4, user_visible = true))]
    fn new(p1: u64, p2: u64, p3: u64, p4: u64, user_visible: bool) -> Self {
        PyEqualDistanceConstraint {
            p1,
            p2,
            p3,
            p4,
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
        ctx: &Bound<'py, PyAny>,
        registry: &Bound<'py, PyAny>,
        to_screen: &Bound<'py, PyAny>,
        is_selected: bool,
        is_hovered: bool,
        point_radius: f64,
    ) -> PyResult<()> {
        super::constraint_draw(
            &self.clone().into_constraint(),
            ctx,
            registry,
            to_screen,
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

    /// Whether this constraint defines the length/distance of the
    /// segment (p1, p2), for either pair (order independent).
    #[pyo3(signature = (p1, p2, entity_id, registry))]
    fn targets_segment(
        &self,
        p1: u64,
        p2: u64,
        entity_id: Option<u64>,
        registry: &PyEntityRegistry,
    ) -> bool {
        let _ = (entity_id, registry);
        let matches =
            |a: u64, b: u64| (p1 == a && p2 == b) || (p1 == b && p2 == a);
        matches(self.p1, self.p2) || matches(self.p3, self.p4)
    }
}

impl PyEqualDistanceConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            user_visible: self.user_visible,
            ..CoreConstraint::new(self.p1, self.p2, self.p3, self.p4)
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEqualDistanceConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.equal_distance", m)?;
    Ok(())
}
