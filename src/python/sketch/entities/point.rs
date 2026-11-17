//! Bindings for the sketch point.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::python::sketch::context::PointCtx;
use crate::sketch::constraint::PointContext;
use pyo3::types::PyTuple;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::types::EntityId;

/// A sketch point: coordinates and solver flags.
///
/// When obtained from a registry it is a live view: coordinate writes
/// are written back to the registry. It also behaves like an ``(x, y)``
/// pair for indexing and iteration.
#[gen_stub_pyclass]
#[pyclass(
    subclass,
    module = "raygeo.sketch.entities.point",
    name = "SketchPoint"
)]
pub struct PySketchPoint {
    pub inner: crate::sketch::entities::point::SketchPoint,
    /// Live back-reference to the owning registry, if any.
    pub registry: Option<Py<PyAny>>,
}

impl PySketchPoint {
    /// Builds a live point view bound to a registry.
    pub fn live(
        inner: crate::sketch::entities::point::SketchPoint,
        registry: &Bound<'_, PyAny>,
    ) -> Self {
        PySketchPoint {
            inner,
            registry: Some(registry.clone().unbind()),
        }
    }

    fn write_back(&self) {
        if let Some(registry) = &self.registry {
            Python::attach(|py| {
                let _ = registry.bind(py).call_method1(
                    "set_point_position",
                    (self.inner.id, self.inner.x, self.inner.y),
                );
            });
        }
    }

    /// Reads the current point state from the owning registry so
    /// views stay live across edits; falls back to the local snapshot
    /// when unattached.
    fn current(&self) -> crate::sketch::entities::point::SketchPoint {
        if let Some(registry) = &self.registry {
            return Python::attach(|py| {
                if let Ok(reg) =
                    registry.bind(py).extract::<PyRef<
                        '_,
                        crate::python::sketch::registry::PyEntityRegistry,
                    >>()
                {
                    if let Some(point) = reg.inner.get_point(self.inner.id) {
                        return point;
                    }
                }
                self.inner.clone()
            });
        }
        self.inner.clone()
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySketchPoint {
    /// Creates a point view with the given id and coordinates.
    #[new]
    #[pyo3(signature = (id, x, y, fixed = false))]
    fn new(id: u64, x: f64, y: f64, fixed: bool) -> Self {
        PySketchPoint {
            inner: crate::sketch::entities::point::SketchPoint {
                id,
                x,
                y,
                fixed,
                waypoint_type:
                    crate::sketch::entities::point::WaypointType::Sharp,
                constrained: false,
            },
            registry: None,
        }
    }

    /// Unique point id, stable across solves.
    #[getter]
    fn id(&self) -> EntityId {
        EntityId(self.inner.id)
    }

    #[setter]
    fn set_id(&mut self, value: EntityId) {
        self.inner.id = value.0;
    }

    /// Waypoint type string ("sharp"/"smooth"/"symmetric").
    #[getter]
    fn waypoint_type(&self) -> String {
        self.current().waypoint_type.as_str().to_string()
    }

    #[setter]
    fn set_waypoint_type(&mut self, value: String) {
        self.inner.waypoint_type =
            crate::sketch::entities::point::WaypointType::parse(&value)
                .unwrap_or(crate::sketch::entities::point::WaypointType::Sharp);
    }

    /// The point position as a ``(x, y)`` tuple.
    fn pos(&self) -> (f64, f64) {
        let point = self.current();
        (point.x, point.y)
    }

    /// Serializes to the reference point dictionary shape.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let point = self.current();
        let dict = PyDict::new(py);
        dict.set_item("id", point.id)?;
        dict.set_item("x", point.x)?;
        dict.set_item("y", point.y)?;
        dict.set_item("fixed", point.fixed)?;
        if point.waypoint_type
            != crate::sketch::entities::point::WaypointType::Sharp
        {
            dict.set_item(
                "waypoint_type",
                point.waypoint_type.as_str().to_string(),
            )?;
        }
        Ok(dict)
    }

    /// Builds a point from the reference point dictionary shape.
    #[staticmethod]
    fn from_dict(dict: &Bound<'_, PyDict>) -> PyResult<Self> {
        fn get<'a>(
            dict: &Bound<'a, PyDict>,
            key: &str,
        ) -> PyResult<Bound<'a, PyAny>> {
            let key = key.to_string();
            dict.get_item(&key)?
                .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(key))
        }
        let get = |key: &str| get(dict, key);
        let id: u64 = get("id")?.extract()?;
        let x: f64 = get("x")?.extract()?;
        let y: f64 = get("y")?.extract()?;
        let fixed: bool = dict
            .get_item("fixed")?
            .and_then(|v| v.extract().ok())
            .unwrap_or(false);
        let waypoint_type: String = dict
            .get_item("waypoint_type")?
            .and_then(|v| v.extract().ok())
            .unwrap_or_else(|| "sharp".to_string());
        Ok(PySketchPoint {
            inner: crate::sketch::entities::point::SketchPoint {
                id,
                x,
                y,
                fixed,
                waypoint_type: crate::sketch::entities::point::WaypointType::parse(
                    &waypoint_type,
                )
                .unwrap_or(crate::sketch::entities::point::WaypointType::Sharp),
                constrained: false,
            },
            registry: None,
        })
    }

    /// X coordinate.
    #[getter]
    fn x(&self) -> f64 {
        self.current().x
    }

    #[setter]
    fn set_x(&mut self, value: f64) {
        self.inner.x = value;
        self.write_back();
    }

    /// Y coordinate.
    #[getter]
    fn y(&self) -> f64 {
        self.current().y
    }

    #[setter]
    fn set_y(&mut self, value: f64) {
        self.inner.y = value;
        self.write_back();
    }

    /// Whether the point is pinned and never moves.
    #[getter]
    fn fixed(&self) -> bool {
        self.current().fixed
    }

    #[setter]
    fn set_fixed(&mut self, value: bool) {
        self.inner.fixed = value;
    }

    /// Whether the point is fully constrained, as computed by the
    /// solver.
    #[getter]
    fn constrained(&self) -> bool {
        self.current().constrained
    }

    #[setter]
    fn set_constrained(&mut self, value: bool) {
        self.inner.constrained = value;
        if let Some(registry) = &self.registry {
            Python::attach(|py| {
                let _ = registry.bind(py).call_method1(
                    "set_point_constrained",
                    (self.inner.id, value),
                );
            });
        }
    }

    /// Index 0 gives x, index 1 gives y (pair-like access).
    fn __getitem__(&self, index: i64) -> PyResult<f64> {
        match index {
            0 | -2 => Ok(self.current().x),
            1 | -1 => Ok(self.current().y),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "point index out of range",
            )),
        }
    }

    /// Writes a coordinate and persists it to the owning registry.
    fn __setitem__(&mut self, index: i64, value: f64) {
        match index {
            0 | -2 => self.inner.x = value,
            1 | -1 => self.inner.y = value,
            _ => {}
        }
        self.write_back();
    }

    /// The point is a pair of coordinates.
    fn __len__(&self) -> usize {
        2
    }

    /// Iterates ``(x, y)``.
    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let cur = self.current();
        let pair = PyTuple::new(py, [cur.x, cur.y])?;
        pair.call_method0("__iter__")
    }

    /// Compares equal to an ``(x, y)`` pair or another point with the
    /// same coordinates.
    fn __eq__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> Py<PyAny> {
        if let Ok(p) = other.extract::<PyRef<'_, PySketchPoint>>() {
            let eq = self.inner.x == p.inner.x && self.inner.y == p.inner.y;
            return pyo3::types::PyBool::new(py, eq)
                .to_owned()
                .into_any()
                .unbind();
        }
        if let Ok((x, y)) = other.extract::<(f64, f64)>() {
            let eq = self.inner.x == x && self.inner.y == y;
            return pyo3::types::PyBool::new(py, eq)
                .to_owned()
                .into_any()
                .unbind();
        }
        py.NotImplemented()
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.x.to_bits().hash(&mut hasher);
        self.inner.y.to_bits().hash(&mut hasher);
        hasher.finish()
    }

    /// Returns a readable representation.
    fn __repr__(&self) -> String {
        format!(
            "SketchPoint(id={}, x={}, y={}, fixed={})",
            self.inner.id, self.inner.x, self.inner.y, self.inner.fixed
        )
    }

    /// Whether the waypoint role is sharp.
    fn is_sharp(&self) -> bool {
        self.current().waypoint_type
            == crate::sketch::entities::point::WaypointType::Sharp
    }

    /// Whether the waypoint role is smooth.
    fn is_smooth(&self) -> bool {
        self.current().waypoint_type
            == crate::sketch::entities::point::WaypointType::Smooth
    }

    /// Whether the waypoint role is symmetric.
    fn is_symmetric(&self) -> bool {
        self.current().waypoint_type
            == crate::sketch::entities::point::WaypointType::Symmetric
    }

    /// Whether the point lies inside the given rect.
    fn is_in_rect(&self, rect: (f64, f64, f64, f64)) -> bool {
        let point = self.current();
        crate::geo::shape::line::is_point_inside_rect(
            crate::geo::types::Point::new(point.x, point.y),
            crate::geo::types::Rect {
                min: crate::geo::types::Point::new(rect.0, rect.1),
                max: crate::geo::types::Point::new(rect.2, rect.3),
            },
        )
    }

    /// The bezier entities connected to this point, as the
    /// registry's own Python objects.
    #[pyo3(signature = (registry, sketch = None))]
    fn get_connected_beziers<'py>(
        &self,
        py: Python<'py>,
        registry: PointCtx<'py>,
        sketch: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Vec<Bound<'py, PyAny>>> {
        let mut point_ids = vec![self.inner.id];
        if let Some(sketch) = sketch {
            if let Ok(ids) = sketch
                .call_method1("get_coincident_points", (self.inner.id,))
                .and_then(|v| v.extract::<Vec<u64>>())
            {
                point_ids.extend(ids);
            }
        }
        let mut connected = Vec::new();
        for eid in registry.entity_ids() {
            let Some(entity) = registry.get_entity(eid) else {
                continue;
            };
            if entity.type_name() != "bezier" {
                continue;
            }
            let ids = entity.get_point_ids();
            if ids.iter().any(|pid| point_ids.contains(pid)) {
                let raw = registry.0.call_method1("get_entity", (eid,))?;
                if raw.is_none() {
                    let object =
                        super::entity_to_py(py, entity.as_ref(), &registry.0)?;
                    connected.push(object);
                } else {
                    connected.push(raw);
                }
            }
        }
        Ok(connected)
    }

    /// The two bezier entities joined at this point, if any.
    #[pyo3(signature = (registry, sketch = None))]
    fn get_paired_beziers<'py>(
        &self,
        py: Python<'py>,
        registry: PointCtx<'py>,
        sketch: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>)> {
        let connected = self.get_connected_beziers(py, registry, sketch)?;
        let none = py.None().into_bound(py);
        match connected.len() {
            0 => Ok((none.clone(), none)),
            1 => Ok((connected[0].clone(), none)),
            _ => Ok((connected[0].clone(), connected[1].clone())),
        }
    }

}

/// Registers the point class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySketchPoint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities.point", m)?;
    Ok(())
}
