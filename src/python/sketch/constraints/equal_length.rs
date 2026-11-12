//! Bindings for the equal-length constraint.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::equal_length::EqualLengthConstraint as CoreConstraint;

/// Enforces equal characteristic lengths, one residual per
/// `(base_a, base_b, other_a, other_b)` tuple.  The caller expands
/// the entities' characteristic-length pairs (including the
/// shorter-side fallback) into flat tuples.
#[gen_stub_pyclass]
#[pyclass(
    eq,
    from_py_object,
    module = "raygeo.sketch.constraints.equal_length",
    name = "EqualLengthConstraint"
)]
#[derive(Clone, PartialEq)]
pub struct PyEqualLengthConstraint {
    /// Resolved characteristic-length pairs.
    #[pyo3(get)]
    pub pairs: Vec<[u64; 4]>,
    /// Reference user-visibility flag.
    pub user_visible: bool,
    /// Reference validation status ordinal.
    #[pyo3(get, set)]
    pub status: u8,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEqualLengthConstraint {
    /// Creates the constraint from resolved pairs.
    #[new]
    #[pyo3(signature = (pairs, user_visible = true))]
    fn new(pairs: Vec<[u64; 4]>, user_visible: bool) -> Self {
        PyEqualLengthConstraint {
            pairs,
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
        registry: &PyEntityRegistry,
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

    /// Whether this constraint restricts the length of the given
    /// entity, matched through the registry by its defining points
    /// (the point ids are ignored, mirroring the reference).
    #[pyo3(signature = (p1, p2, entity_id, registry))]
    fn targets_segment(
        &self,
        p1: u64,
        p2: u64,
        entity_id: Option<u64>,
        registry: &PyEntityRegistry,
    ) -> bool {
        let _ = (p1, p2);
        match entity_id {
            Some(id) => self.covers_entity(registry, id),
            None => false,
        }
    }

    /// Whether this constraint explicitly defines the radius/length
    /// of the given entity, matched through the registry by its
    /// defining points.
    fn constrains_radius(
        &self,
        registry: &PyEntityRegistry,
        entity_id: u64,
    ) -> bool {
        self.covers_entity(registry, entity_id)
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

impl PyEqualLengthConstraint {
    /// Whether any resolved pair belongs to the entity, matched
    /// through its defining points (either the base or the other
    /// segment of a pair).
    fn covers_entity(
        &self,
        registry: &PyEntityRegistry,
        entity_id: u64,
    ) -> bool {
        let Some(ids) = registry.inner.entity_point_ids(entity_id) else {
            return false;
        };
        self.pairs.iter().any(|pair| {
            (ids.contains(&pair[0]) && ids.contains(&pair[1]))
                || (ids.contains(&pair[2]) && ids.contains(&pair[3]))
        })
    }
}

impl PyEqualLengthConstraint {
    pub(crate) fn into_constraint(self) -> CoreConstraint {
        let mut core = CoreConstraint::new(self.pairs);
        core.user_visible = self.user_visible;
        core
    }
}

/// Registers the constraint class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEqualLengthConstraint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.constraints.equal_length", m)?;
    Ok(())
}
