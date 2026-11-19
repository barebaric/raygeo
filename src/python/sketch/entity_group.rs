//! Python bindings for ephemeral entity groups.

use std::cell::RefCell;
use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen::derive::{
    gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods,
};

use crate::python::sketch::entities::entity_to_py;
use crate::python::sketch::entities::point::PySketchPoint;
use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::types::EntityId;
use crate::sketch::entity_group::{
    points_bbox_center as core_points_bbox_center, EntityGroup,
};
use crate::sketch::transform::PlacementTransform;

/// Adapts a Python placement object to the core transform trait.
struct PyPlacementTransform<'py> {
    obj: Bound<'py, PyAny>,
    error: RefCell<Option<PyErr>>,
}

impl<'py> PyPlacementTransform<'py> {
    fn new(obj: Bound<'py, PyAny>) -> Self {
        PyPlacementTransform {
            obj,
            error: RefCell::new(None),
        }
    }

    fn call(&self, method: &str, x: f64, y: f64) -> Option<(f64, f64)> {
        match self.obj.call_method1(method, (x, y)) {
            Ok(result) => match result.extract::<(f64, f64)>() {
                Ok(pair) => Some(pair),
                Err(error) => {
                    self.store_error(error);
                    None
                }
            },
            Err(error) => {
                self.store_error(error);
                None
            }
        }
    }

    fn store_error(&self, error: PyErr) {
        let mut slot = self.error.borrow_mut();
        if slot.is_none() {
            *slot = Some(error);
        }
    }

    fn into_result(self) -> PyResult<()> {
        match self.error.into_inner() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl PlacementTransform for PyPlacementTransform<'_> {
    fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        self.call("transform_point", x, y).unwrap_or((x, y))
    }

    fn transform_offset(&self, dx: f64, dy: f64) -> (f64, f64) {
        self.call("transform_offset", dx, dy).unwrap_or((dx, dy))
    }
}

/// An ephemeral view over ``(registry, entity_ids)``: the surviving
/// entities plus their unique defining points, manipulable as one
/// piece of geometry.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.entity_group",
    name = "EntityGroup",
    skip_from_py_object
)]
pub struct PyEntityGroup {
    registry: Py<PyEntityRegistry>,
    group: EntityGroup,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEntityGroup {
    #[new]
    fn new(
        registry: &Bound<'_, PyEntityRegistry>,
        entity_ids: Vec<EntityId>,
    ) -> Self {
        PyEntityGroup {
            registry: registry.clone().unbind(),
            group: EntityGroup::new(
                entity_ids.into_iter().map(|id| id.0).collect(),
            ),
        }
    }

    /// The group's entity ids, in group order.
    #[getter]
    fn entity_ids(&self) -> Vec<EntityId> {
        self.group
            .entity_ids
            .iter()
            .map(|&id| EntityId(id))
            .collect()
    }

    /// Returns the group's surviving entities, in group order.
    fn entities<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let registry = self.registry.bind(py);
        let inner = registry.borrow();
        let entities = PyList::empty(py);
        for entity in self.group.entities(&inner.inner) {
            entities.append(entity_to_py(py, entity, Some(registry))?)?;
        }
        Ok(entities)
    }

    /// Returns the group's unique point ids in stable order.
    fn point_ids(&self, py: Python<'_>) -> Vec<EntityId> {
        let registry = self.registry.bind(py);
        let inner = registry.borrow();
        self.group
            .point_ids(&inner.inner)
            .into_iter()
            .map(EntityId)
            .collect()
    }

    /// Returns the group's unique defining points.
    fn points<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let registry = self.registry.bind(py);
        let inner = registry.borrow();
        let points = PyList::empty(py);
        for point in self.group.points(&inner.inner) {
            let live = PySketchPoint::live(point, registry);
            points.append(Py::new(py, live)?)?;
        }
        Ok(points)
    }

    /// Returns the logical center of the group's entities.
    fn center(&self, py: Python<'_>) -> (f64, f64) {
        let registry = self.registry.bind(py);
        let inner = registry.borrow();
        self.group.center(&inner.inner)
    }

    /// Rigidly moves the group's defining points and internal
    /// offsets by the placement transform.
    fn apply_placement(
        &self,
        py: Python<'_>,
        placement: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let transform = PyPlacementTransform::new(placement.clone());
        let registry = self.registry.bind(py);
        let mut inner = registry.borrow_mut();
        self.group.apply_placement(&mut inner.inner, &transform);
        drop(inner);
        transform.into_result()
    }

    /// Applies a shape-preserving placement transform.
    fn apply_rigid_motion(
        &self,
        py: Python<'_>,
        motion: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.apply_placement(py, motion)
    }

    /// Moves every point of the group by the given delta.
    fn translate(&self, py: Python<'_>, dx: f64, dy: f64) {
        let registry = self.registry.bind(py);
        let mut inner = registry.borrow_mut();
        self.group.translate(&mut inner.inner, dx, dy);
    }

    /// Translates the group radially so its center sits on a circle.
    #[pyo3(signature = (center, radius, pin = None))]
    fn radial_project(
        &self,
        py: Python<'_>,
        center: (f64, f64),
        radius: f64,
        pin: Option<(f64, f64)>,
    ) {
        let registry = self.registry.bind(py);
        let mut inner = registry.borrow_mut();
        self.group
            .radial_project(&mut inner.inner, center, radius, pin);
    }

    /// Rewrites an existing copy group from this template group.
    #[pyo3(signature = (copy, placement, extra_point_pairs = None))]
    fn rewrite_copy_from<'py>(
        &self,
        py: Python<'py>,
        copy: &PyEntityGroup,
        placement: &Bound<'_, PyAny>,
        extra_point_pairs: Option<Vec<(EntityId, EntityId)>>,
    ) -> PyResult<Bound<'py, PyList>> {
        let transform = PyPlacementTransform::new(placement.clone());
        let pairs: Vec<(u64, u64)> = extra_point_pairs
            .unwrap_or_default()
            .into_iter()
            .map(|(template, copy)| (template.0, copy.0))
            .collect();
        let registry = self.registry.bind(py);
        let mut inner = registry.borrow_mut();
        let applied = self.group.rewrite_copy_from(
            &mut inner.inner,
            &copy.group,
            &transform,
            &pairs,
        );
        drop(inner);
        transform.into_result()?;

        let applied_list = PyList::empty(py);
        for (point_id, x, y) in applied {
            let point = inner_point(py, &self.registry, point_id)?;
            applied_list.append((point, x, y).into_pyobject(py)?)?;
        }
        Ok(applied_list)
    }

    /// Captures the current position of every point of the group.
    fn snapshot_positions<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyList>> {
        let registry = self.registry.bind(py);
        let inner = registry.borrow();
        let snapshot = PyList::empty(py);
        for point in self.group.points(&inner.inner) {
            let x = point.x;
            let y = point.y;
            let live = PySketchPoint::live(point, registry);
            snapshot.append((Py::new(py, live)?, x, y).into_pyobject(py)?)?;
        }
        Ok(snapshot)
    }

    /// Restores point positions captured by ``snapshot_positions``.
    #[staticmethod]
    fn restore_positions(snapshot: &Bound<'_, PyAny>) -> PyResult<()> {
        for item in snapshot.try_iter()? {
            let item = item?;
            let point = item.get_item(0)?;
            let x: f64 = item.get_item(1)?.extract()?;
            let y: f64 = item.get_item(2)?.extract()?;
            point.setattr("x", x)?;
            point.setattr("y", y)?;
        }
        Ok(())
    }

    /// Samples every entity of the group into polylines.
    fn polylines(&self, py: Python<'_>) -> Vec<Vec<(f64, f64)>> {
        let registry = self.registry.bind(py);
        let inner = registry.borrow();
        self.group.polylines(&inner.inner)
    }

    /// Returns helper geometry belonging to the group's entities.
    fn helper_ids(&self, py: Python<'_>) -> Vec<EntityId> {
        let registry = self.registry.bind(py);
        let inner = registry.borrow();
        self.group
            .helper_ids(&inner.inner)
            .into_iter()
            .map(EntityId)
            .collect()
    }

    /// Returns a view extended with this group's helper entities.
    fn with_helpers(&self, py: Python<'_>) -> PyEntityGroup {
        let registry = self.registry.bind(py);
        let inner = registry.borrow();
        PyEntityGroup {
            registry: self.registry.clone_ref(py),
            group: self.group.with_helpers(&inner.inner),
        }
    }

    /// Rewrites the group's entities' point ID references.
    fn remap_point_refs(&self, py: Python<'_>, pid_map: HashMap<i64, i64>) {
        let map: HashMap<u64, u64> = pid_map
            .into_iter()
            .map(|(old, new)| (old as u64, new as u64))
            .collect();
        let registry = self.registry.bind(py);
        let mut inner = registry.borrow_mut();
        self.group.remap_point_refs(&mut inner.inner, &map);
    }
}

fn inner_point<'py>(
    py: Python<'py>,
    registry: &Py<PyEntityRegistry>,
    point_id: u64,
) -> PyResult<Bound<'py, PyAny>> {
    let registry = registry.bind(py);
    let inner = registry.borrow();
    let point = inner.inner.get_point(point_id).ok_or_else(|| {
        pyo3::exceptions::PyIndexError::new_err("unknown point id")
    })?;
    Ok(Py::new(py, PySketchPoint::live(point, registry))?
        .into_bound(py)
        .into_any())
}

/// Rewrites point ID references on an entity or constraint.
#[gen_stub_pyfunction(module = "raygeo.sketch.entity_group")]
#[pyfunction]
fn remap_point_refs(
    obj: &Bound<'_, PyAny>,
    pid_map: HashMap<i64, i64>,
) -> PyResult<()> {
    obj.call_method1("remap_references", (pid_map,))?;
    Ok(())
}

/// Returns the bbox center of a list of points.
#[gen_stub_pyfunction(module = "raygeo.sketch.entity_group")]
#[pyfunction]
fn points_bbox_center(points: &Bound<'_, PyAny>) -> PyResult<(f64, f64)> {
    let mut collected = Vec::new();
    for point in points.try_iter()? {
        let point = point?;
        let x: f64 = point.getattr("x")?.extract()?;
        let y: f64 = point.getattr("y")?.extract()?;
        collected.push(crate::sketch::entities::point::SketchPoint {
            id: 0,
            x,
            y,
            fixed: false,
            waypoint_type: crate::sketch::entities::point::WaypointType::Sharp,
            constrained: false,
        });
    }
    Ok(core_points_bbox_center(&collected))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let entity_group_mod = PyModule::new(py, "entity_group")?;

    entity_group_mod.setattr(
        "__doc__",
        "Ephemeral views over groups of sketch entities.",
    )?;

    entity_group_mod.add_class::<PyEntityGroup>()?;
    entity_group_mod
        .add_function(wrap_pyfunction!(remap_point_refs, &entity_group_mod)?)?;
    entity_group_mod.add_function(wrap_pyfunction!(
        points_bbox_center,
        &entity_group_mod
    )?)?;

    m.add_submodule(&entity_group_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entity_group", &entity_group_mod)?;

    Ok(())
}
