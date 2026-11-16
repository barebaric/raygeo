//! Bindings for the equal-length constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use super::PyConstraintBase;

use crate::python::sketch::protocols::{Canvas, DrawContext, ScreenMap};
use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::equal_length::EqualLengthConstraint as CoreConstraint;

/// Enforces equal characteristic lengths, one residual per
/// `(base_a, base_b, other_a, other_b)` tuple.  The caller expands
/// the entities' characteristic-length pairs (including the
/// shorter-side fallback) into flat tuples.
#[gen_stub_pyclass]
#[pyclass(
    extends = PyConstraintBase,
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.equal_length",
    name = "EqualLengthConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyEqualLengthConstraint {
    /// The constrained entities.
    #[pyo3(get, set)]
    pub entity_ids: Vec<u64>,
    /// Reference user-visibility flag.
    #[pyo3(get)]
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEqualLengthConstraint {
    /// Creates the constraint for the given entities.
    #[new]
    #[pyo3(signature = (entity_ids, user_visible = true))]
    fn new(
        entity_ids: Vec<u64>,
        user_visible: bool,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyConstraintBase).add_subclass(
            PyEqualLengthConstraint {
                entity_ids,
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
        registry: &PyEntityRegistry,
        _ctx: Option<Py<PyAny>>,
    ) -> Vec<f64> {
        self.clone().into_constraint().error(&registry.inner)
    }

    /// Partial derivatives of the error by point id.
    fn gradient(
        &self,
        registry: &PyEntityRegistry,
    ) -> std::collections::HashMap<u64, Vec<(f64, f64)>> {
        super::constraint_gradient(&self.clone().into_constraint(), registry)
    }

    /// Point ids the constraint references.
    fn referenced_points(&self, registry: &PyEntityRegistry) -> Vec<u64> {
        self.clone()
            .into_constraint()
            .referenced_points(&registry.inner)
    }

    /// Alias for referenced_points (reference naming).
    fn get_referenced_point_ids(
        &self,
        registry: &PyEntityRegistry,
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

    /// Sets the reference user-visibility flag.
    fn set_user_visible(&mut self, value: bool) {
        self.user_visible = value;
    }

    /// Whether this constraint restricts the length of the given
    /// entity.
    fn targets_segment(
        &self,
        _p1: u64,
        _p2: u64,
        entity_id: Option<u64>,
    ) -> bool {
        entity_id.is_some_and(|eid| self.entity_ids.contains(&eid))
    }

    /// Whether this constraint explicitly defines the radius/length
    /// of the given entity.
    fn constrains_radius(
        &self,
        _registry: &PyEntityRegistry,
        entity_id: u64,
    ) -> bool {
        self.entity_ids.contains(&entity_id)
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

impl PyEqualLengthConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        CoreConstraint {
            entity_ids: self.entity_ids,
            user_visible: self.user_visible,
        }
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEqualLengthConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.equal_length", m)?;
    Ok(())
}
