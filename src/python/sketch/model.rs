//! High-level sketch document: state, serialization, solving and
//! geometry emission.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyTuple};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::types::Point;
use crate::python::sketch::constraints::PyConstraintBase;
use crate::python::sketch::object_list::PyConstraintList;
use crate::python::sketch::registry::PyEntityRegistry;
use crate::python::sketch::solver::constraint_from_py;
use crate::sketch::constraint::SketchConstraint;
use crate::sketch::constraints::coincident::CoincidentConstraint;
use crate::sketch::document::Sketch as CoreSketch;
use crate::sketch::entities::arc::Arc;
use crate::sketch::object_list::{shared_list, ObjectList};

fn new_uid(py: Python<'_>) -> PyResult<String> {
    Ok(py.import("uuid")?.call_method0("uuid4")?.str()?.to_string())
}

fn document_class<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    py.import("raygeo.sketch.document")?.getattr("DocumentMeta")
}

fn params_class<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    py.import("raygeo.sketch.params")?
        .getattr("ParameterContext")
}

fn registry_class<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    py.import("raygeo.sketch.registry")?
        .getattr("EntityRegistry")
}

fn constraints_module<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(py.import("raygeo.sketch.constraints")?.into_any())
}

fn arrays_class<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    py.import("raygeo.sketch.arrays")?.getattr("Array")
}

fn fill_class<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    py.import("raygeo.sketch.document")?.getattr("Fill")
}

/// A complete host-agnostic sketch document.
#[gen_stub_pyclass]
#[pyclass(module = "raygeo.sketch", name = "Sketch")]
pub struct PySketch {
    meta: Py<PyAny>,
    params: Py<PyAny>,
    registry: Py<PyEntityRegistry>,
    model: CoreSketch<Py<PyConstraintBase>, Py<PyAny>>,
    fills: Py<PyList>,
    arrays: Py<PyList>,
    last_solve_values: Py<PyDict>,
    solved_ctx: Option<Py<PyDict>>,
    pending_constraints: Option<Vec<Py<PyAny>>>,
    pending_update_status: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySketch {
    #[new]
    #[pyo3(signature = (name = "New Sketch"))]
    fn new(py: Python<'_>, name: &str) -> PyResult<Self> {
        let meta = document_class(py)?.call1((new_uid(py)?, name))?.unbind();
        let params = params_class(py)?.call0()?.unbind();
        let registry: Py<PyEntityRegistry> =
            registry_class(py)?.call0()?.extract()?;
        let model = CoreSketch::<Py<PyConstraintBase>, Py<PyAny>>::new();
        let fills = PyList::empty(py).unbind();
        let arrays = PyList::empty(py).unbind();
        let origin = registry
            .bind(py)
            .call_method1("add_point", (0.0, 0.0, true))?;
        meta.bind(py).setattr("origin_id", origin)?;
        Ok(PySketch {
            meta,
            params,
            registry,
            model,
            fills,
            arrays,
            last_solve_values: PyDict::new(py).unbind(),
            solved_ctx: None,
            pending_constraints: None,
            pending_update_status: false,
        })
    }

    #[getter]
    fn meta(&self, py: Python<'_>) -> Py<PyAny> {
        self.meta.clone_ref(py)
    }

    #[setter]
    fn set_meta(&mut self, value: Py<PyAny>) {
        self.meta = value;
    }

    #[getter]
    fn params(&self, py: Python<'_>) -> Py<PyAny> {
        self.params.clone_ref(py)
    }

    #[setter]
    fn set_params(&mut self, value: Py<PyAny>) {
        self.params = value;
    }

    #[getter]
    fn registry(&self, py: Python<'_>) -> Py<PyEntityRegistry> {
        self.registry.clone_ref(py)
    }

    #[setter]
    fn set_registry(&mut self, value: Py<PyEntityRegistry>) {
        self.registry = value;
    }

    #[getter]
    fn constraints(&self, py: Python<'_>) -> PyResult<Py<PyConstraintList>> {
        Py::new(
            py,
            PyConstraintList::from_shared(self.model.constraints.clone()),
        )
    }

    #[setter]
    fn set_constraints(&mut self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.model.constraints =
            PyConstraintList::from_iterable(value)?.shared();
        Ok(())
    }

    #[getter]
    fn fills(&self, py: Python<'_>) -> Py<PyList> {
        self.fills.clone_ref(py)
    }

    #[setter]
    fn set_fills(&mut self, value: Py<PyList>) {
        self.fills = value;
    }

    #[getter]
    fn arrays(&self, py: Python<'_>) -> Py<PyList> {
        self.arrays.clone_ref(py)
    }

    #[setter]
    fn set_arrays(&mut self, value: Py<PyList>) {
        self.arrays = value;
    }

    #[getter]
    fn solved_ctx(&self, py: Python<'_>) -> Option<Py<PyDict>> {
        self.solved_ctx.as_ref().map(|ctx| ctx.clone_ref(py))
    }

    #[getter]
    fn last_solve_values(&self, py: Python<'_>) -> Py<PyDict> {
        self.last_solve_values.clone_ref(py)
    }

    #[getter]
    fn uid(&self, py: Python<'_>) -> PyResult<String> {
        self.meta.bind(py).getattr("uid")?.extract()
    }

    #[setter]
    fn set_uid(&self, value: &str) -> PyResult<()> {
        Python::attach(|py| self.meta.bind(py).setattr("uid", value))
    }

    #[getter]
    fn name(&self, py: Python<'_>) -> PyResult<String> {
        self.meta.bind(py).getattr("name")?.extract()
    }

    #[setter]
    fn set_name(&self, value: &str) -> PyResult<()> {
        Python::attach(|py| self.meta.bind(py).setattr("name", value))
    }

    #[getter]
    fn hidden(&self, py: Python<'_>) -> PyResult<bool> {
        self.meta.bind(py).getattr("hidden")?.extract()
    }

    #[setter]
    fn set_hidden(&self, value: bool) -> PyResult<()> {
        Python::attach(|py| self.meta.bind(py).setattr("hidden", value))
    }

    #[getter]
    fn origin_id(&self, py: Python<'_>) -> PyResult<u64> {
        self.meta.bind(py).getattr("origin_id")?.extract()
    }

    #[setter]
    fn set_origin_id(&self, value: u64) -> PyResult<()> {
        Python::attach(|py| self.meta.bind(py).setattr("origin_id", value))
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let data = PyDict::new(py);
        data.set_item(
            "version",
            self.meta.bind(py).getattr("format_version")?,
        )?;
        data.set_item("uid", self.uid(py)?)?;
        data.set_item("name", self.name(py)?)?;
        data.set_item("params", self.params.bind(py).call_method0("to_dict")?)?;
        data.set_item(
            "registry",
            self.registry.bind(py).call_method0("to_dict")?,
        )?;
        let constraints = PyList::empty(py);
        for constraint in self.constraint_items(py) {
            constraints.append(constraint.call_method0("to_dict")?)?;
        }
        data.set_item("constraints", constraints)?;
        let fills = PyList::empty(py);
        for fill in self.fills.bind(py).iter() {
            fills.append(fill.call_method0("to_dict")?)?;
        }
        data.set_item("fills", fills)?;
        let arrays = PyList::empty(py);
        for array in self.arrays.bind(py).iter() {
            arrays.append(array.call_method0("to_dict")?)?;
        }
        data.set_item("arrays", arrays)?;
        data.set_item("origin_id", self.origin_id(py)?)?;
        data.set_item("hidden", self.hidden(py)?)?;
        Ok(data)
    }

    #[staticmethod]
    fn from_dict(
        py: Python<'_>,
        data: &Bound<'_, PyDict>,
    ) -> PyResult<PySketch> {
        for key in ["params", "registry", "constraints", "origin_id"] {
            if !data.contains(key)? {
                return Err(PyKeyError::new_err(format!(
                    "Sketch data is missing required key: {key}"
                )));
            }
        }
        let meta = document_class(py)?
            .call_method1("from_dict", (data,))?
            .unbind();
        let params = params_class(py)?
            .call_method1("from_dict", (data.get_item("params")?.unwrap(),))?
            .unbind();
        let registry: Py<PyEntityRegistry> = registry_class(py)?
            .call_method1("from_dict", (data.get_item("registry")?.unwrap(),))?
            .extract()?;
        let mut constraints = Vec::new();
        if let Some(items) = data.get_item("constraints")? {
            for item in items.try_iter()? {
                let item = item?;
                let type_name: String = item.get_item("type")?.extract()?;
                let cls = constraints_module(py)?.getattr(&type_name)?;
                constraints.push(
                    cls.call_method1("from_dict", (&item,))?.extract::<Py<
                        crate::python::sketch::constraints::PyConstraintBase,
                    >>(
                    )?,
                );
            }
        }
        let fills = PyList::empty(py);
        if let Some(items) = data.get_item("fills")? {
            for item in items.try_iter()? {
                fills.append(
                    fill_class(py)?.call_method1("from_dict", (item?,))?,
                )?;
            }
        }
        let arrays = PyList::empty(py);
        if let Some(items) = data.get_item("arrays")? {
            for item in items.try_iter()? {
                arrays.append(
                    arrays_class(py)?.call_method1("from_dict", (item?,))?,
                )?;
            }
        }
        Ok(PySketch {
            meta,
            params,
            registry,
            model: CoreSketch {
                constraints: std::sync::Arc::new(std::sync::RwLock::new(
                    ObjectList::from_vec(constraints),
                )),
                arrays: shared_list(),
            },
            fills: fills.unbind(),
            arrays: arrays.unbind(),
            last_solve_values: PyDict::new(py).unbind(),
            solved_ctx: None,
            pending_constraints: None,
            pending_update_status: false,
        })
    }

    fn clone_for_geometry(&self, py: Python<'_>) -> PyResult<PySketch> {
        let meta = self.meta.bind(py).call_method0("copy")?.unbind();
        let registry: Py<PyEntityRegistry> = registry_class(py)?
            .call_method1(
                "from_dict",
                (self.registry.bind(py).call_method0("to_dict")?,),
            )?
            .extract()?;
        let params = params_class(py)?
            .call_method1(
                "from_dict",
                (self.params.bind(py).call_method0("to_dict")?,),
            )?
            .unbind();
        let constraints = self.constraint_handles(py);
        let arrays = PyList::empty(py);
        for array in self.arrays.bind(py).iter() {
            arrays.append(array.call_method0("copy")?)?;
        }
        Ok(PySketch {
            meta,
            params,
            registry,
            model: CoreSketch {
                constraints: std::sync::Arc::new(std::sync::RwLock::new(
                    ObjectList::from_vec(constraints),
                )),
                arrays: shared_list(),
            },
            fills: self.fills.clone_ref(py),
            arrays: arrays.unbind(),
            last_solve_values: PyDict::new(py).unbind(),
            solved_ctx: None,
            pending_constraints: None,
            pending_update_status: false,
        })
    }

    #[pyo3(signature = (x, y, fixed = false))]
    fn add_point(
        &self,
        py: Python<'_>,
        x: f64,
        y: f64,
        fixed: bool,
    ) -> PyResult<u64> {
        self.registry
            .bind(py)
            .call_method1("add_point", (x, y, fixed))?
            .extract()
    }

    #[pyo3(signature = (p1, p2, construction = false))]
    fn add_line(
        &self,
        py: Python<'_>,
        p1: u64,
        p2: u64,
        construction: bool,
    ) -> PyResult<u64> {
        self.registry
            .bind(py)
            .call_method1("add_line", (p1, p2, construction))?
            .extract()
    }

    #[pyo3(signature = (start, end, center, clockwise = false, construction = false))]
    fn add_arc(
        &self,
        py: Python<'_>,
        start: u64,
        end: u64,
        center: u64,
        clockwise: bool,
        construction: bool,
    ) -> PyResult<u64> {
        self.registry
            .bind(py)
            .call_method1(
                "add_arc",
                (start, end, center, clockwise, construction),
            )?
            .extract()
    }

    #[pyo3(signature = (start, end, construction = false, cp1 = None, cp2 = None))]
    fn add_bezier(
        &self,
        py: Python<'_>,
        start: u64,
        end: u64,
        construction: bool,
        cp1: Option<(f64, f64)>,
        cp2: Option<(f64, f64)>,
    ) -> PyResult<u64> {
        self.registry
            .bind(py)
            .call_method1("add_bezier", (start, end, construction, cp1, cp2))?
            .extract()
    }

    #[pyo3(signature = (center, radius, construction = false))]
    fn add_circle(
        &self,
        py: Python<'_>,
        center: u64,
        radius: u64,
        construction: bool,
    ) -> PyResult<u64> {
        self.registry
            .bind(py)
            .call_method1("add_circle", (center, radius, construction))?
            .extract()
    }

    fn remove_entities(
        &self,
        py: Python<'_>,
        entities: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let ids = PyList::empty(py);
        for entity in entities.try_iter()? {
            ids.append(entity?.getattr("id")?)?;
        }
        self.registry
            .bind(py)
            .call_method1("remove_entities", (ids,))?;
        self.validate_fills(py)
    }

    #[pyo3(signature = (pid = None))]
    fn remove_point_if_unused(
        &self,
        py: Python<'_>,
        pid: Option<u64>,
    ) -> PyResult<bool> {
        let Some(pid) = pid else {
            return Ok(false);
        };
        let registry = self.registry.bind(py);
        if registry
            .call_method1("is_point_used", (pid,))?
            .extract::<bool>()?
        {
            return Ok(false);
        }
        let ids = PyList::new(py, [pid])?;
        registry.call_method1("remove_points", (ids,))?;
        Ok(true)
    }

    fn capture_undo_state<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let state = PyDict::new(py);
        let points = PyDict::new(py);
        for point in
            self.registry.bind(py).call_method0("points")?.try_iter()?
        {
            let point = point?;
            let pid: u64 = point.getattr("id")?.extract()?;
            points.set_item(pid, (point.getattr("x")?, point.getattr("y")?))?;
        }
        state.set_item("points", points)?;
        let entities = PyDict::new(py);
        for entity in self
            .registry
            .bind(py)
            .call_method0("entities")?
            .try_iter()?
        {
            let entity = entity?;
            let entity_state = entity.call_method0("get_state")?;
            if !entity_state.is_none() {
                entities.set_item(entity.getattr("id")?, entity_state)?;
            }
        }
        state.set_item("entities", entities)?;
        let arrays = PyList::empty(py);
        for array in self.arrays.bind(py).iter() {
            arrays.append(PyTuple::new(
                py,
                [array.getattr("uid")?, array.call_method0("snapshot")?],
            )?)?;
        }
        state.set_item("arrays", arrays)?;
        Ok(state)
    }

    fn apply_undo_state(
        &self,
        py: Python<'_>,
        state: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        let registry = self.registry.bind(py);
        if let Some(points) = state.get_item("points")? {
            for item in points.cast::<PyDict>()?.items() {
                let (pid, coords): (u64, (f64, f64)) = item.extract()?;
                if let Ok(point) = registry.call_method1("get_point", (pid,)) {
                    point.setattr("x", coords.0)?;
                    point.setattr("y", coords.1)?;
                }
            }
        }
        if let Some(entities) = state.get_item("entities")? {
            for item in entities.cast::<PyDict>()?.items() {
                let (eid, estate): (u64, Py<PyAny>) = item.extract()?;
                if !registry.call_method1("get_entity", (eid,))?.is_none() {
                    registry
                        .call_method1("set_state", (eid, estate.bind(py)))?;
                }
            }
        }
        let by_uid: HashMap<String, Py<PyAny>> = self
            .arrays
            .bind(py)
            .iter()
            .map(|array| {
                Ok::<_, PyErr>((
                    array.getattr("uid")?.extract::<String>()?,
                    array.unbind(),
                ))
            })
            .collect::<PyResult<_>>()?;
        if let Some(arrays) = state.get_item("arrays")? {
            for item in arrays.try_iter()? {
                let (uid, astate): (String, Py<PyAny>) = item?.extract()?;
                if let Some(array) = by_uid.get(&uid) {
                    array
                        .bind(py)
                        .call_method1("restore", (astate.bind(py),))?;
                }
            }
        }
        Ok(())
    }

    fn set_param(
        &self,
        py: Python<'_>,
        name: &str,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.params.bind(py).call_method1("set", (name, value))?;
        Ok(())
    }

    fn constrain_distance(
        &self,
        py: Python<'_>,
        p1: u64,
        p2: u64,
        dist: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let constraint = constraints_module(py)?
            .getattr("DistanceConstraint")?
            .call1((p1, p2, dist))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(constraint.unbind())
    }

    fn constrain_equal_distance(
        &self,
        py: Python<'_>,
        p1: u64,
        p2: u64,
        p3: u64,
        p4: u64,
    ) -> PyResult<()> {
        let constraint = constraints_module(py)?
            .getattr("EqualDistanceConstraint")?
            .call1((p1, p2, p3, p4))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(())
    }

    fn constrain_horizontal(
        &self,
        py: Python<'_>,
        p1: u64,
        p2: u64,
    ) -> PyResult<()> {
        let constraint = constraints_module(py)?
            .getattr("HorizontalConstraint")?
            .call1((p1, p2))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(())
    }

    fn constrain_vertical(
        &self,
        py: Python<'_>,
        p1: u64,
        p2: u64,
    ) -> PyResult<()> {
        let constraint = constraints_module(py)?
            .getattr("VerticalConstraint")?
            .call1((p1, p2))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(())
    }

    fn constrain_coincident(
        &self,
        py: Python<'_>,
        p1: u64,
        p2: u64,
    ) -> PyResult<()> {
        let constraint = constraints_module(py)?
            .getattr("CoincidentConstraint")?
            .call1((p1, p2))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(())
    }

    fn constrain_point_on_line(
        &self,
        py: Python<'_>,
        point_id: u64,
        shape_id: u64,
    ) -> PyResult<()> {
        let constraint = constraints_module(py)?
            .getattr("PointOnLineConstraint")?
            .call1((point_id, shape_id))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(())
    }

    fn constrain_radius(
        &self,
        py: Python<'_>,
        entity_id: u64,
        radius: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let constraint = constraints_module(py)?
            .getattr("RadiusConstraint")?
            .call1((entity_id, radius))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(constraint.unbind())
    }

    fn constrain_diameter(
        &self,
        py: Python<'_>,
        circle_id: u64,
        diameter: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let constraint = constraints_module(py)?
            .getattr("DiameterConstraint")?
            .call1((circle_id, diameter))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(constraint.unbind())
    }

    fn constrain_perpendicular(
        &self,
        py: Python<'_>,
        l1: u64,
        l2: u64,
    ) -> PyResult<()> {
        let constraint = constraints_module(py)?
            .getattr("PerpendicularConstraint")?
            .call1((l1, l2))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(())
    }

    fn constrain_tangent(
        &self,
        py: Python<'_>,
        line: u64,
        shape: u64,
    ) -> PyResult<()> {
        let constraint = constraints_module(py)?
            .getattr("TangentConstraint")?
            .call1((line, shape))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(())
    }

    fn constrain_equal_length(
        &self,
        py: Python<'_>,
        entity_ids: Vec<u64>,
    ) -> PyResult<()> {
        if entity_ids.len() < 2 {
            return Ok(());
        }
        let constraint = constraints_module(py)?
            .getattr("EqualLengthConstraint")?
            .call1((entity_ids,))?;
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(())
    }

    fn constrain_symmetry(
        &self,
        py: Python<'_>,
        point_ids: Vec<u64>,
        entity_ids: Vec<u64>,
    ) -> PyResult<()> {
        let class = constraints_module(py)?.getattr("SymmetryConstraint")?;
        let constraint = if point_ids.len() == 3 && entity_ids.is_empty() {
            class.call1((point_ids[1], point_ids[2], point_ids[0]))?
        } else if point_ids.len() == 2 && entity_ids.len() == 1 {
            let kwargs = PyDict::new(py);
            kwargs.set_item("axis", entity_ids[0])?;
            class.call((point_ids[0], point_ids[1]), Some(&kwargs))?
        } else {
            return Ok(());
        };
        self.model
            .constraints
            .write()
            .unwrap()
            .push(constraint.extract()?);
        Ok(())
    }

    fn get_coincident_points(
        &self,
        py: Python<'_>,
        pid: u64,
    ) -> PyResult<HashSet<u64>> {
        let parsed = self.parsed_constraints(py)?;
        let mut adjacency: HashMap<u64, Vec<u64>> = HashMap::new();
        for constraint in &parsed {
            let Some(c) =
                constraint.as_any().downcast_ref::<CoincidentConstraint>()
            else {
                continue;
            };
            adjacency.entry(c.p1).or_default().push(c.p2);
            adjacency.entry(c.p2).or_default().push(c.p1);
        }
        let mut group = HashSet::new();
        let mut stack = vec![pid];
        while let Some(current) = stack.pop() {
            if !group.insert(current) {
                continue;
            }
            if let Some(neighbors) = adjacency.get(&current) {
                stack.extend(neighbors.iter().copied());
            }
        }
        if group.is_empty() {
            group.insert(pid);
        }
        Ok(group)
    }

    fn is_empty(&self, py: Python<'_>) -> PyResult<bool> {
        Ok(self
            .registry
            .bind(py)
            .call_method0("entities")?
            .try_iter()?
            .count()
            == 0)
    }

    fn is_fully_constrained(&self, py: Python<'_>) -> PyResult<bool> {
        let registry = self.registry.bind(py);
        let entities: Vec<Bound<'_, PyAny>> = registry
            .call_method0("entities")?
            .try_iter()?
            .collect::<PyResult<_>>()?;
        if entities.iter().any(|e| {
            e.getattr("constrained")
                .and_then(|v| v.extract::<bool>())
                .map(|v| !v)
                .unwrap_or(true)
        }) {
            return Ok(false);
        }
        let mut usage: HashMap<u64, usize> = HashMap::new();
        for entity in &entities {
            for pid in entity.call_method0("get_point_ids")?.try_iter()? {
                *usage.entry(pid?.extract()?).or_default() += 1;
            }
        }
        let mut allowed = HashSet::new();
        for entity in &entities {
            for pid in entity
                .call_method0("get_ignorable_unconstrained_points")?
                .try_iter()?
            {
                let pid: u64 = pid?.extract()?;
                if usage.get(&pid).copied().unwrap_or(0) == 1 {
                    allowed.insert(pid);
                }
            }
        }
        for point in registry.call_method0("points")?.try_iter()? {
            let point = point?;
            let pid: u64 = point.getattr("id")?.extract()?;
            let constrained: bool = point.getattr("constrained")?.extract()?;
            if !constrained && !allowed.contains(&pid) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn conflicting_constraints<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyList>> {
        let result = PyList::empty(py);
        for constraint in self.constraint_items(py) {
            if constraint.getattr("status")?.extract::<i32>()? == 4 {
                result.append(constraint)?;
            }
        }
        Ok(result)
    }

    fn has_conflicts(&self, py: Python<'_>) -> PyResult<bool> {
        for constraint in self.constraint_items(py) {
            if constraint.getattr("status")?.extract::<i32>()? == 4 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn is_array_guide_radius_point(
        &self,
        py: Python<'_>,
        pid: u64,
    ) -> PyResult<bool> {
        for array in self.arrays.bind(py).iter() {
            if array
                .call_method1(
                    "is_guide_radius_point",
                    (self.registry.clone_ref(py), pid),
                )?
                .extract::<bool>()?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn get_derived_point_ids(&self, py: Python<'_>) -> PyResult<HashSet<u64>> {
        let mut result = HashSet::new();
        let registry = self.registry.bind(py);
        for array in self.arrays.bind(py).iter() {
            for slot in array.getattr("members")?.try_iter()? {
                for eid in slot?.get_item(1)?.try_iter()? {
                    let eid: u64 = eid?.extract()?;
                    if let Ok(entity) =
                        registry.call_method1("get_entity", (eid,))
                    {
                        if !entity.is_none() {
                            for pid in entity
                                .call_method0("get_point_ids")?
                                .try_iter()?
                            {
                                result.insert(pid?.extract()?);
                            }
                        }
                    }
                }
            }
            for standalone in array
                .getattr("standalone_pids")?
                .call_method0("values")?
                .try_iter()?
            {
                for pid in standalone?.try_iter()? {
                    result.insert(pid?.extract()?);
                }
            }
        }
        Ok(result)
    }

    fn find_standalone_point_ids(
        &self,
        py: Python<'_>,
        entity_ids: Vec<u64>,
    ) -> PyResult<Vec<u64>> {
        let registry = self.registry.bind(py);
        let mut group: HashSet<u64> = entity_ids.into_iter().collect();
        let mut group_pids: HashSet<u64> = HashSet::new();
        for eid in &group {
            if let Ok(entity) = registry.call_method1("get_entity", (*eid,)) {
                if !entity.is_none() {
                    for pid in
                        entity.call_method0("get_point_ids")?.try_iter()?
                    {
                        group_pids.insert(pid?.extract()?);
                    }
                }
            }
        }
        let mut extra = Vec::new();
        let mut changed = true;
        while changed {
            changed = false;
            for constraint in self.constraint_items(py) {
                let eids: HashSet<u64> = constraint
                    .call_method0("get_referenced_entity_ids")?
                    .extract()?;
                if !eids.is_subset(&group) {
                    continue;
                }
                let pids: HashSet<u64> = constraint
                    .call_method0("get_referenced_point_ids")?
                    .extract()?;
                let missing: Vec<u64> =
                    pids.difference(&group_pids).copied().collect();
                if missing.len() == 1
                    && pids.intersection(&group_pids).count() > 1
                {
                    group_pids.extend(missing.iter().copied());
                    group.extend(missing.iter().copied());
                    extra.extend(missing);
                    changed = true;
                }
            }
        }
        Ok(extra)
    }

    fn get_internal_constraints<'py>(
        &self,
        py: Python<'py>,
        entity_ids: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyList>> {
        let entity_ids: HashSet<u64> = entity_ids
            .try_iter()?
            .map(|item| item?.extract())
            .collect::<PyResult<_>>()?;
        let registry = self.registry.bind(py);
        let mut group_pids = HashSet::new();
        for eid in &entity_ids {
            if let Ok(entity) = registry.call_method1("get_entity", (*eid,)) {
                if !entity.is_none() {
                    for pid in
                        entity.call_method0("get_point_ids")?.try_iter()?
                    {
                        group_pids.insert(pid?.extract()?);
                    }
                    continue;
                }
            }
            if registry.call_method1("get_point", (*eid,)).is_ok() {
                group_pids.insert(*eid);
            }
        }
        let paired = self.find_axis_coincidence_pairs(py)?;
        let result = PyList::empty(py);
        let origin = self.origin_id(py)?;
        for constraint in self.constraint_items(py) {
            let pids: HashSet<u64> = constraint
                .call_method0("get_referenced_point_ids")?
                .extract()?;
            if pids.contains(&origin) {
                continue;
            }
            if constraint
                .call_method0("is_world_anchored")?
                .extract::<bool>()?
                && !paired.contains(&ptr_key(&constraint))
            {
                continue;
            }
            let eids: HashSet<u64> = constraint
                .call_method0("get_referenced_entity_ids")?
                .extract()?;
            if pids.is_disjoint(&group_pids) && eids.is_disjoint(&entity_ids) {
                continue;
            }
            if pids.is_subset(&group_pids) && eids.is_subset(&entity_ids) {
                result.append(constraint)?;
            }
        }
        Ok(result)
    }

    fn find_axis_coincidence_pairs(
        &self,
        py: Python<'_>,
    ) -> PyResult<HashSet<usize>> {
        let mut verticals: HashMap<(u64, u64), usize> = HashMap::new();
        for constraint in self.constraint_items(py) {
            if constraint.get_type().name()? == "VerticalConstraint" {
                let p1: u64 = constraint.getattr("p1")?.extract()?;
                let p2: u64 = constraint.getattr("p2")?.extract()?;
                verticals
                    .insert((p1.min(p2), p1.max(p2)), ptr_key(&constraint));
            }
        }
        let mut paired = HashSet::new();
        for constraint in self.constraint_items(py) {
            if constraint.get_type().name()? == "HorizontalConstraint" {
                let p1: u64 = constraint.getattr("p1")?.extract()?;
                let p2: u64 = constraint.getattr("p2")?.extract()?;
                if let Some(partner) = verticals.get(&(p1.min(p2), p1.max(p2)))
                {
                    paired.insert(ptr_key(&constraint));
                    paired.insert(*partner);
                }
            }
        }
        Ok(paired)
    }

    fn prune_arrays(&self, py: Python<'_>) -> PyResult<()> {
        let arrays = self.arrays.bind(py);
        let mut index = 0;
        while index < arrays.len() {
            let keep: bool = arrays
                .get_item(index)?
                .call_method1("prune", (self.registry.clone_ref(py),))?
                .extract()?;
            if keep {
                index += 1;
            } else {
                arrays.del_item(index)?;
            }
        }
        Ok(())
    }

    fn move_point(
        &mut self,
        py: Python<'_>,
        pid: u64,
        x: f64,
        y: f64,
    ) -> PyResult<bool> {
        let registry = self.registry.bind(py);
        let point = match registry.call_method1("get_point", (pid,)) {
            Ok(point) => point,
            Err(_) => return Ok(false),
        };
        if point.getattr("fixed")?.extract::<bool>()?
            || point.getattr("constrained")?.extract::<bool>()?
        {
            return Ok(false);
        }
        point.setattr("x", x)?;
        point.setattr("y", y)?;
        self.solve(py, None, true, None, None, None, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        extra_constraints = None,
        update_constraint_status = true,
        variable_overrides = None,
        excluded_constraints = None,
        point_scope = None,
        initial_values = None,
        template_functions = None
    ))]
    fn solve(
        &mut self,
        py: Python<'_>,
        extra_constraints: Option<Vec<Py<PyAny>>>,
        update_constraint_status: bool,
        variable_overrides: Option<Py<PyAny>>,
        excluded_constraints: Option<HashSet<usize>>,
        point_scope: Option<HashSet<u64>>,
        initial_values: Option<Py<PyDict>>,
        template_functions: Option<Py<PyDict>>,
    ) -> PyResult<bool> {
        let started = Instant::now();
        let registry = self.registry.bind(py);
        let mut scope = point_scope;
        if let Some(scope_set) = &scope {
            let existing: HashSet<u64> = registry
                .call_method0("points")?
                .try_iter()?
                .map(|p| p?.getattr("id")?.extract())
                .collect::<PyResult<_>>()?;
            if !scope_set.is_subset(&existing) {
                scope = None;
            }
        }

        let solve_params = params_class(py)?.call_method1(
            "from_dict",
            (self.params.bind(py).call_method0("to_dict")?,),
        )?;
        let seed = PyDict::new(py);
        if let Some(values) = initial_values {
            seed.update(values.bind(py).as_mapping())?;
        }
        if let Some(overrides) = variable_overrides {
            seed.update(overrides.bind(py).cast::<PyDict>()?.as_mapping())?;
        }
        self.last_solve_values = seed.clone().unbind();
        solve_params.call_method1("evaluate_all", (seed.clone(),))?;
        let ctx: Py<PyDict> =
            solve_params.call_method0("get_all_values")?.extract()?;
        let solved_ctx = PyDict::new(py);
        solved_ctx.update(ctx.bind(py).as_mapping())?;
        if let Some(templates) = template_functions {
            solved_ctx.update(templates.bind(py).as_mapping())?;
        }
        self.solved_ctx = Some(solved_ctx.unbind());

        let excluded = excluded_constraints.unwrap_or_default();
        let mut active: Vec<Py<PyAny>> = Vec::new();
        for (index, constraint) in
            self.constraint_items(py).into_iter().enumerate()
        {
            if excluded.contains(&index) {
                continue;
            }
            if let Some(scope_ref) = &scope {
                let registry = self.registry.bind(py).borrow();
                let points =
                    crate::python::sketch::components::referenced_points_of(
                        &registry,
                        &constraint,
                    )?;
                if points.iter().all(|pid| !scope_ref.contains(pid)) {
                    continue;
                }
            }
            active.push(constraint.extract()?);
        }
        if let Some(extra) = extra_constraints {
            for constraint in extra {
                if let Some(scope_ref) = &scope {
                    let registry = self.registry.bind(py).borrow();
                    let points = crate::python::sketch::components::
                        referenced_points_of(&registry, constraint.bind(py))?;
                    if points.iter().all(|pid| !scope_ref.contains(pid)) {
                        continue;
                    }
                }
                active.push(constraint);
            }
        }
        for constraint in &active {
            let constraint = constraint.bind(py);
            if constraint.hasattr("update_from_context")? {
                constraint
                    .call_method1("update_from_context", (ctx.bind(py),))?;
            }
        }

        let mut touched = HashSet::new();
        {
            let reg_borrow = self.registry.bind(py).borrow();
            for constraint in &active {
                let points =
                    crate::python::sketch::components::referenced_points_of(
                        &reg_borrow,
                        constraint.bind(py),
                    )?;
                touched.extend(points);
            }
        }
        let stabilizers = PyList::empty(py);
        let drag_class = constraints_module(py)?.getattr("DragConstraint")?;
        for point in registry.call_method0("points")?.try_iter()? {
            let point = point?;
            let pid: u64 = point.getattr("id")?.extract()?;
            if point.getattr("fixed")?.extract::<bool>()?
                || !touched.contains(&pid)
                || scope.as_ref().is_some_and(|set| !set.contains(&pid))
            {
                continue;
            }
            stabilizers.append(drag_class.call1((
                pid,
                point.getattr("x")?,
                point.getattr("y")?,
                1e-4,
                false,
            ))?)?;
        }

        let update_status = update_constraint_status && scope.is_none();
        let result: bool = crate::python::sketch::solver::run_solve(
            self.registry.bind(py),
            active.iter().map(|c| c.bind(py).clone()).collect(),
            Some(stabilizers.iter().map(|c| c.into_any()).collect::<Vec<_>>()),
            scope
                .as_ref()
                .map(|set| set.iter().copied().collect::<Vec<_>>()),
            1e-5,
            update_status,
        )?;

        self.pending_constraints = if update_status {
            Some(active.iter().map(|c| c.clone_ref(py)).collect())
        } else {
            None
        };
        self.pending_update_status = update_status;
        log_solve(py, started, &active, &scope, result)?;
        Ok(result)
    }

    fn finish_solve(&mut self, py: Python<'_>) -> PyResult<()> {
        if !self.pending_update_status {
            self.pending_constraints = None;
            return Ok(());
        }
        let Some(active) = self.pending_constraints.take() else {
            return Ok(());
        };
        let conflicting: Vec<usize> =
            crate::python::sketch::solver::run_conflicting(
                self.registry.bind(py),
                active.iter().map(|c| c.bind(py).clone()).collect(),
                1e-3,
            )?;
        let index_of: HashMap<usize, usize> = self
            .constraint_items(py)
            .iter()
            .enumerate()
            .map(|(index, constraint)| (ptr_key(constraint), index))
            .collect();
        let mapped: HashSet<usize> = conflicting
            .into_iter()
            .filter_map(|i| active.get(i))
            .filter_map(|constraint| {
                index_of.get(&ptr_key(constraint.bind(py))).copied()
            })
            .collect();
        self.apply_conflict_status(py, &mapped)
    }

    #[pyo3(signature = (resolved_text = None))]
    fn to_geometry<'py>(
        &self,
        py: Python<'py>,
        resolved_text: Option<Py<PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let resolved = resolved_text
            .map(|dict| dict.bind(py).extract::<HashMap<u64, Option<String>>>())
            .transpose()?;
        crate::python::sketch::geometry::run_document_geometry(
            self.registry.bind(py),
            self.constraint_items(py),
            resolved,
        )
        .map(|geometry| {
            Py::new(py, geometry).map(|g| g.into_bound(py).into_any())
        })?
    }

    fn create_fill_geometry<'py>(
        &self,
        py: Python<'py>,
        fill: &Bound<'_, PyAny>,
        exclude_ids: HashSet<u64>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let boundary: Vec<(u64, bool)> = fill.getattr("boundary")?.extract()?;
        if boundary.is_empty() {
            return Ok(None);
        }
        let registry = self.registry.bind(py);
        if boundary.len() == 1 {
            let (eid, _) = boundary[0];
            if exclude_ids.contains(&eid) {
                return Ok(None);
            }
            let entity = registry.call_method1("get_entity", (eid,))?;
            if entity.is_none() {
                return Ok(None);
            }
            let geo = entity.call_method1(
                "create_fill_geometry",
                (self.registry.clone_ref(py),),
            )?;
            return Ok(if geo.is_none() { None } else { Some(geo) });
        }
        let (first_eid, first_fwd) = boundary[0];
        if exclude_ids.contains(&first_eid) {
            return Ok(None);
        }
        let first = registry.call_method1("get_entity", (first_eid,))?;
        if first.is_none() {
            return Ok(None);
        }
        let pids: Vec<u64> =
            first.call_method0("get_endpoint_ids")?.extract()?;
        let start_pid = if first_fwd { pids[0] } else { pids[1] };
        let start = registry.call_method1("get_point", (start_pid,))?;
        let geo = py.import("raygeo.geo")?.getattr("Geometry")?.call0()?;
        geo.call_method1(
            "move_to",
            (start.getattr("x")?, start.getattr("y")?),
        )?;
        for (eid, fwd) in boundary {
            if exclude_ids.contains(&eid) {
                return Ok(None);
            }
            let entity = registry.call_method1("get_entity", (eid,))?;
            if entity.is_none() {
                return Ok(None);
            }
            entity.call_method1(
                "append_to_geometry",
                (&geo, self.registry.clone_ref(py), fwd),
            )?;
        }
        Ok(Some(geo))
    }

    fn validate_fills(&self, py: Python<'_>) -> PyResult<()> {
        let loops = self.find_all_closed_loops(py)?;
        let loop_sets: HashSet<Vec<(u64, bool)>> = loops
            .into_iter()
            .map(|mut l| {
                l.sort();
                l
            })
            .collect();
        let fills = self.fills.bind(py);
        let mut index = 0;
        while index < fills.len() {
            let boundary: Vec<(u64, bool)> =
                fills.get_item(index)?.getattr("boundary")?.extract()?;
            let mut key = boundary;
            key.sort();
            if loop_sets.contains(&key) {
                index += 1;
            } else {
                fills.del_item(index)?;
            }
        }
        Ok(())
    }

    fn find_all_closed_loops(
        &self,
        py: Python<'_>,
    ) -> PyResult<Vec<Vec<(u64, bool)>>> {
        let registry = &self.registry.bind(py).borrow().inner;
        let parsed = self.parsed_constraints(py)?;
        let point_ids: Vec<u64> = registry.point_ids();
        let mut parent: HashMap<u64, u64> =
            point_ids.iter().map(|id| (*id, *id)).collect();
        for constraint in &parsed {
            if let Some(c) =
                constraint.as_any().downcast_ref::<CoincidentConstraint>()
            {
                union(&mut parent, c.p1, c.p2);
            }
        }
        let mut groups_map: HashMap<u64, Vec<u64>> = HashMap::new();
        for pid in &point_ids {
            let root = find(&mut parent, *pid);
            groups_map.entry(root).or_default().push(*pid);
        }
        let coincident_groups: Vec<Vec<u64>> =
            groups_map.into_values().collect();
        let mut edges = Vec::new();
        let mut arcs = Vec::new();
        let mut closed = Vec::new();
        for entity in registry.all_entities() {
            if entity.is_edge_entity() {
                let endpoints = entity.get_endpoint_ids();
                if endpoints.len() != 2 {
                    continue;
                }
                let p1 = endpoints[0];
                let p2 = endpoints[1];
                let t1 = entity.tangent_at(registry, p1).unwrap_or((1.0, 0.0));
                let t2 = entity.tangent_at(registry, p2).unwrap_or((1.0, 0.0));
                edges.push(crate::sketch::topology::SketchEdge {
                    entity_id: entity.base().id,
                    p1,
                    p2,
                    angle_at_p1: t1.1.atan2(t1.0),
                    angle_at_p2: t2.1.atan2(t2.0),
                });
                if let Some(arc) = entity.as_any().downcast_ref::<Arc>() {
                    arcs.push(crate::sketch::topology::SketchArc {
                        entity_id: arc.base.id,
                        start: arc.start,
                        end: arc.end,
                        center: arc.center,
                        clockwise: arc.clockwise,
                    });
                }
            }
            if entity.is_closed_loop() {
                let signed_area =
                    entity.enclosed_signed_area(registry).unwrap_or(0.0);
                closed.push(crate::sketch::topology::ClosedEntity {
                    entity_id: entity.base().id,
                    signed_area,
                });
            }
        }
        let points: Vec<(u64, f64, f64)> = registry
            .all_points()
            .iter()
            .map(|p| (p.id, p.x, p.y))
            .collect();
        Ok(crate::sketch::topology::find_closed_loops(
            &edges,
            &points,
            &coincident_groups,
            &arcs,
            &closed,
        ))
    }

    fn get_loop_at_point(
        &self,
        py: Python<'_>,
        mx: f64,
        my: f64,
    ) -> PyResult<Option<Vec<(u64, bool)>>> {
        let loops = self.find_all_closed_loops(py)?;
        let registry = &self.registry.bind(py).borrow().inner;
        let mut hits: Vec<(f64, Vec<(u64, bool)>)> = Vec::new();
        for loop_edges in loops {
            let hit = if loop_edges.len() == 1 {
                let entity = registry.get_entity(loop_edges[0].0);
                entity.is_some_and(|e| e.contains_point(registry, mx, my))
            } else {
                let polygon: Vec<Point> = loop_polygon(registry, &loop_edges)
                    .into_iter()
                    .map(|(x, y)| Point::new(x, y))
                    .collect();
                !polygon.is_empty()
                    && crate::geo::shape::polygon::is_point_inside_polygon(
                        Point::new(mx, my),
                        &polygon,
                    )
            };
            if hit {
                hits.push((
                    loop_signed_area(registry, &loop_edges).abs(),
                    loop_edges,
                ));
            }
        }
        hits.sort_by(|a, b| {
            a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(hits.into_iter().next().map(|(_, loop_edges)| loop_edges))
    }

    fn _build_adjacency_list<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let registry = self.registry.bind(py);
        let adjacency = PyDict::new(py);
        let mut point_to_group: HashMap<u64, HashSet<u64>> = HashMap::new();
        for point in registry.call_method0("points")?.try_iter()? {
            let pid: u64 = point?.getattr("id")?.extract()?;
            if point_to_group.contains_key(&pid) {
                continue;
            }
            let group = self.get_coincident_points(py, pid)?;
            for member in &group {
                point_to_group.insert(*member, group.clone());
            }
        }
        for entity in registry.call_method0("entities")?.try_iter()? {
            let entity = entity?;
            if !entity.call_method0("is_edge_entity")?.extract::<bool>()? {
                continue;
            }
            let pids: Vec<u64> =
                entity.call_method0("get_endpoint_ids")?.extract()?;
            let (p1, p2) = (pids[0], pids[1]);
            let group1 = point_to_group
                .get(&p1)
                .cloned()
                .unwrap_or_else(|| HashSet::from([p1]));
            let group2 = point_to_group
                .get(&p2)
                .cloned()
                .unwrap_or_else(|| HashSet::from([p2]));
            for (src_group, dst_group, forward) in
                [(&group1, &group2, true), (&group2, &group1, false)]
            {
                for src in src_group {
                    for dst in dst_group {
                        if src == dst {
                            continue;
                        }
                        let entry = match adjacency.get_item(src)? {
                            Some(entry) => entry,
                            None => {
                                let list = PyList::empty(py);
                                adjacency.set_item(src, &list)?;
                                list.into_any()
                            }
                        };
                        let edge = PyDict::new(py);
                        edge.set_item("to", dst)?;
                        edge.set_item("id", entity.getattr("id")?)?;
                        edge.set_item("fwd", forward)?;
                        entry.cast::<PyList>()?.append(edge)?;
                    }
                }
            }
        }
        Ok(adjacency)
    }

    fn _sort_edges_by_angle<'py>(
        &self,
        py: Python<'py>,
        adjacency: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let registry = self.registry.bind(py);
        let sorted = PyDict::new(py);
        for (pid, edges) in adjacency.iter() {
            let pid: u64 = pid.extract()?;
            let mut entries: Vec<(f64, Bound<'py, PyAny>)> = Vec::new();
            for edge in edges.try_iter()? {
                let edge = edge?;
                let eid: u64 = edge.get_item("id")?.extract()?;
                let entity = registry.call_method1("get_entity", (eid,))?;
                if entity.is_none() {
                    continue;
                }
                let tangent: (f64, f64) = entity
                    .call_method1(
                        "tangent_at",
                        (self.registry.clone_ref(py), pid),
                    )?
                    .extract()?;
                let angle = tangent.1.atan2(tangent.0);
                let with_angle = PyDict::new(py);
                with_angle.set_item("angle", angle)?;
                for key in ["to", "id", "fwd"] {
                    with_angle.set_item(key, edge.get_item(key)?)?;
                }
                entries.push((angle, with_angle.into_any()));
            }
            entries.sort_by(|a, b| {
                a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
            });
            let list = PyList::empty(py);
            for (_, entry) in entries {
                list.append(entry)?;
            }
            sorted.set_item(pid, list)?;
        }
        Ok(sorted)
    }

    fn _get_next_edge_ccw<'py>(
        &self,
        _py: Python<'py>,
        current_pid: u64,
        incoming_entity_id: u64,
        incoming_fwd: bool,
        sorted_adj: &Bound<'py, PyDict>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let Some(outgoing) = sorted_adj.get_item(current_pid)? else {
            return Ok(None);
        };
        let edges: Vec<Bound<'py, PyAny>> =
            outgoing.try_iter()?.collect::<PyResult<_>>()?;
        if edges.is_empty() {
            return Ok(None);
        }
        let rev_fwd = !incoming_fwd;
        let Some(index) = edges.iter().position(|edge| {
            let eid: u64 = edge
                .get_item("id")
                .ok()
                .and_then(|v| v.extract().ok())
                .unwrap_or(u64::MAX);
            let fwd: bool = edge
                .get_item("fwd")
                .ok()
                .and_then(|v| v.extract().ok())
                .unwrap_or(false);
            eid == incoming_entity_id && fwd == rev_fwd
        }) else {
            return Ok(None);
        };
        let next = (index + edges.len() - 1) % edges.len();
        Ok(Some(edges[next].clone()))
    }

    fn _calculate_loop_signed_area(
        &self,
        py: Python<'_>,
        loop_edges: Vec<(u64, bool)>,
    ) -> f64 {
        let registry = self.registry.bind(py);
        loop_signed_area(&registry.borrow().inner, &loop_edges)
    }

    fn _loop_to_polygon(
        &self,
        py: Python<'_>,
        loop_edges: Vec<(u64, bool)>,
    ) -> Vec<(f64, f64)> {
        let registry = self.registry.bind(py);
        loop_polygon(&registry.borrow().inner, &loop_edges)
    }
}

impl PySketch {
    /// Snapshot of the owned constraint handles.
    fn constraint_handles(&self, py: Python<'_>) -> Vec<Py<PyConstraintBase>> {
        self.model
            .constraints
            .read()
            .unwrap()
            .items()
            .iter()
            .map(|item| item.clone_ref(py))
            .collect()
    }

    /// Snapshot of the owned constraints, bound to the GIL.
    fn constraint_items<'py>(&self, py: Python<'py>) -> Vec<Bound<'py, PyAny>> {
        self.constraint_handles(py)
            .into_iter()
            .map(|item| item.into_bound(py).into_any())
            .collect()
    }

    fn apply_conflict_status(
        &self,
        py: Python<'_>,
        conflicting: &HashSet<usize>,
    ) -> PyResult<()> {
        for (index, constraint) in
            self.constraint_items(py).into_iter().enumerate()
        {
            let status: i32 = constraint.getattr("status")?.extract()?;
            if conflicting.contains(&index) {
                if status != 3 {
                    constraint.setattr("status", 4)?;
                }
            } else if status == 4 {
                constraint.setattr("status", 1)?;
            }
        }
        Ok(())
    }

    fn parsed_constraints(
        &self,
        py: Python<'_>,
    ) -> PyResult<Vec<Box<dyn SketchConstraint>>> {
        self.constraint_items(py)
            .iter()
            .map(|c| constraint_from_py(c))
            .collect()
    }
}

fn log_solve(
    py: Python<'_>,
    started: Instant,
    active: &[Py<PyAny>],
    scope: &Option<HashSet<u64>>,
    result: bool,
) -> PyResult<()> {
    let logger = py
        .import("logging")?
        .call_method1("getLogger", ("sketcher",))?;
    logger.call_method1(
        "debug",
        (
            "Sketch.solve: constraints=%d scope=%s total=%.1fms ok=%s",
            active.len(),
            scope.as_ref().map(|s| s.len()),
            started.elapsed().as_secs_f64() * 1000.0,
            result,
        ),
    )?;
    Ok(())
}

fn ptr_key(obj: &Bound<'_, PyAny>) -> usize {
    obj.as_ptr() as *mut std::ffi::c_void as usize
}

fn find(parent: &mut HashMap<u64, u64>, id: u64) -> u64 {
    let mut root = id;
    while parent.get(&root).copied().unwrap_or(root) != root {
        root = parent.get(&root).copied().unwrap_or(root);
    }
    let mut current = id;
    while current != root {
        let next = parent.get(&current).copied().unwrap_or(root);
        parent.insert(current, root);
        current = next;
    }
    root
}

fn union(parent: &mut HashMap<u64, u64>, a: u64, b: u64) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent.insert(ra, rb);
    }
}

fn loop_polygon(
    registry: &crate::sketch::registry::EntityRegistry,
    loop_edges: &[(u64, bool)],
) -> Vec<(f64, f64)> {
    let mut polygon = Vec::new();
    for (eid, fwd) in loop_edges {
        let Some(entity) = registry.get_entity(*eid) else {
            return Vec::new();
        };
        let vertices = entity.to_polygon_vertices(registry, *fwd);
        if vertices.is_empty() {
            return Vec::new();
        }
        polygon.extend(vertices);
    }
    polygon
}

fn loop_signed_area(
    registry: &crate::sketch::registry::EntityRegistry,
    loop_edges: &[(u64, bool)],
) -> f64 {
    if loop_edges.is_empty() {
        return 0.0;
    }
    if loop_edges.len() == 1 {
        if let Some(entity) = registry.get_entity(loop_edges[0].0) {
            if entity.is_closed_loop() {
                return entity.enclosed_signed_area(registry).unwrap_or(0.0);
            }
        }
    }
    let polygon = loop_polygon(registry, loop_edges);
    if polygon.is_empty() {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..polygon.len() {
        let (x1, y1) = polygon[i];
        let (x2, y2) = polygon[(i + 1) % polygon.len()];
        area += x1 * y2 - x2 * y1;
    }
    area * 0.5
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySketch>()?;
    Ok(())
}
