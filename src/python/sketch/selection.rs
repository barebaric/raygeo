//! Python bindings for the sketch selection state.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::entities::{entity_to_py, AnyEntity};
use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::types::EntityId;
use crate::sketch::selection::Selection;

/// The current editor selection: point ids, entity ids, a constraint
/// index and an optional junction point.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.selection",
    name = "Selection",
    skip_from_py_object
)]
#[derive(Clone, Default)]
pub struct PySelection {
    inner: Selection,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySelection {
    #[new]
    fn new() -> Self {
        PySelection {
            inner: Selection::default(),
        }
    }

    /// Selected point ids.
    #[getter]
    fn point_ids(&self) -> Vec<EntityId> {
        self.inner.point_ids.iter().copied().map(EntityId).collect()
    }

    #[setter]
    fn set_point_ids(&mut self, value: Vec<EntityId>) {
        self.inner.point_ids = value.into_iter().map(|id| id.0).collect();
    }

    /// Selected entity ids.
    #[getter]
    fn entity_ids(&self) -> Vec<EntityId> {
        self.inner
            .entity_ids
            .iter()
            .copied()
            .map(EntityId)
            .collect()
    }

    #[setter]
    fn set_entity_ids(&mut self, value: Vec<EntityId>) {
        self.inner.entity_ids = value.into_iter().map(|id| id.0).collect();
    }

    /// Selected constraint index.
    #[getter]
    fn constraint_idx(&self) -> Option<usize> {
        self.inner.constraint_idx
    }

    #[setter]
    fn set_constraint_idx(&mut self, value: Option<usize>) {
        self.inner.constraint_idx = value;
    }

    /// Selected implicit junction point.
    #[getter]
    fn junction_pid(&self) -> Option<EntityId> {
        self.inner.junction_pid.map(EntityId)
    }

    #[setter]
    fn set_junction_pid(&mut self, value: Option<EntityId>) {
        self.inner.junction_pid = value.map(|id| id.0);
    }

    /// Clears every selected item.
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Whether nothing is selected.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Creates a shallow copy of the selection.
    fn copy(&self) -> PySelection {
        PySelection {
            inner: self.inner.clone(),
        }
    }

    fn __copy__(&self) -> PySelection {
        self.copy()
    }

    /// Selects a constraint by index.
    fn select_constraint(&mut self, idx: usize, is_multi: bool) {
        self.inner.select_constraint(idx, is_multi);
    }

    /// Selects an implicit junction point.
    fn select_junction(&mut self, pid: EntityId, is_multi: bool) {
        self.inner.select_junction(pid.0, is_multi);
    }

    /// Selects a point by id.
    fn select_point(&mut self, pid: EntityId, is_multi: bool) {
        self.inner.select_point(pid.0, is_multi);
    }

    /// Selects an entity by id.
    fn select_entity(&mut self, entity_id: EntityId, is_multi: bool) {
        self.inner.select_entity(entity_id.0, is_multi);
    }

    /// Resolves selected entity ids to entity objects.
    fn resolve_entities<'py>(
        &self,
        py: Python<'py>,
        registry: &Bound<'_, PyEntityRegistry>,
    ) -> PyResult<Option<Vec<AnyEntity<'py>>>> {
        let mut entities = Vec::with_capacity(self.inner.entity_ids.len());
        let reg = registry.borrow();
        for &eid in &self.inner.entity_ids {
            let Some(entity) = reg.inner.get_entity(eid) else {
                return Ok(None);
            };
            entities.push(AnyEntity(entity_to_py(py, entity, Some(registry))?));
        }
        Ok(Some(entities))
    }

    /// Adds all entities connected through shared points.
    fn select_connected_entities(
        &mut self,
        entity_id: EntityId,
        registry: &PyEntityRegistry,
    ) {
        self.inner
            .select_connected_entities(entity_id.0, &registry.inner);
    }

    /// Adds one point without clearing other selection kinds.
    fn add_point(&mut self, pid: EntityId) {
        self.inner.add_point(pid.0);
    }

    /// Adds one entity without clearing other selection kinds.
    fn add_entity(&mut self, entity_id: EntityId) {
        self.inner.add_entity(entity_id.0);
    }

    /// Clears only the selected points.
    fn clear_points(&mut self) {
        self.inner.clear_points();
    }

    /// Clears only the selected entities.
    fn clear_entities(&mut self) {
        self.inner.clear_entities();
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let selection_mod = PyModule::new(py, "selection")?;

    selection_mod
        .setattr("__doc__", "Selection state of the sketch editor.")?;
    selection_mod.add_class::<PySelection>()?;

    m.add_submodule(&selection_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.selection", &selection_mod)?;

    Ok(())
}
