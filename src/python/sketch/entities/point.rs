//! Bindings for the sketch point.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::python::sketch::context::PointCtx;
use crate::python::sketch::registry::PyEntityRegistry;
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
    pub registry: Option<Py<PyEntityRegistry>>,
}

impl PySketchPoint {
    /// Builds a live point view bound to a registry.
    pub fn live(
        inner: crate::sketch::entities::point::SketchPoint,
        registry: &Bound<'_, PyEntityRegistry>,
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
                if let Ok(reg) = registry.bind(py).try_borrow() {
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
    fn new(id: i64, x: f64, y: f64, fixed: bool) -> Self {
        PySketchPoint {
            inner: crate::sketch::entities::point::SketchPoint {
                id: id as u64,
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
        let waypoint_type =
            crate::sketch::entities::point::WaypointType::parse(&value)
                .unwrap_or(crate::sketch::entities::point::WaypointType::Sharp);
        self.inner.waypoint_type = waypoint_type;
        if let Some(registry) = &self.registry {
            Python::attach(|py| {
                let _ = registry.bind(py).call_method1(
                    "set_point_waypoint_type",
                    (self.inner.id, value),
                );
            });
        }
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
                waypoint_type:
                    crate::sketch::entities::point::WaypointType::parse(
                        &waypoint_type,
                    )
                    .unwrap_or(
                        crate::sketch::entities::point::WaypointType::Sharp,
                    ),
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
        if let Some(registry) = &self.registry {
            Python::attach(|py| {
                let _ = registry
                    .bind(py)
                    .call_method1("set_point_fixed", (self.inner.id, value));
            });
        }
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
                    let object = super::entity_to_py(
                        py,
                        entity.as_ref(),
                        registry.0.cast::<PyEntityRegistry>().ok(),
                    )?;
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

    /// Applies the bezier-waypoint constraint after a control point
    /// edit on one of the point's connected beziers.
    #[pyo3(signature = (registry, bezier, cp_index, sketch = None))]
    fn apply_constraint(
        &self,
        py: Python<'_>,
        registry: PointCtx<'_>,
        bezier: &Bound<'_, PyAny>,
        cp_index: i32,
        sketch: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        if self.is_sharp() {
            return Ok(());
        }
        let (b1, b2) = self.get_paired_beziers(py, registry, sketch)?;
        if b1.is_none() || b2.is_none() {
            return Ok(());
        }
        let other = if bezier.eq(&b1)? { &b2 } else { &b1 };
        let mut point_ids = std::collections::HashSet::from([self.inner.id]);
        if let Some(sketch) = sketch {
            if let Ok(ids) = sketch
                .call_method1("get_coincident_points", (self.inner.id,))
                .and_then(|v| v.extract::<Vec<u64>>())
            {
                point_ids.extend(ids);
            }
        }
        let start: u64 = bezier.getattr("start_idx")?.extract()?;
        let end: u64 = bezier.getattr("end_idx")?.extract()?;
        if start == self.inner.id && cp_index == 1 {
            if let Some(cp) =
                bezier.getattr("cp1")?.extract::<Option<(f64, f64)>>()?
            {
                let other_end: u64 = other.getattr("end_idx")?.extract()?;
                let other_start: u64 = other.getattr("start_idx")?.extract()?;
                if point_ids.contains(&other_end) {
                    let old = other
                        .getattr("cp2")?
                        .extract::<Option<(f64, f64)>>()?;
                    other.setattr(
                        "cp2",
                        constrained_cp(cp, old, self.is_symmetric()),
                    )?;
                } else if point_ids.contains(&other_start) {
                    let old = other
                        .getattr("cp1")?
                        .extract::<Option<(f64, f64)>>()?;
                    other.setattr(
                        "cp1",
                        constrained_cp(cp, old, self.is_symmetric()),
                    )?;
                }
            }
        } else if end == self.inner.id && cp_index == 2 {
            if let Some(cp) =
                bezier.getattr("cp2")?.extract::<Option<(f64, f64)>>()?
            {
                let other_end: u64 = other.getattr("end_idx")?.extract()?;
                let other_start: u64 = other.getattr("start_idx")?.extract()?;
                if point_ids.contains(&other_start) {
                    let old = other
                        .getattr("cp1")?
                        .extract::<Option<(f64, f64)>>()?;
                    other.setattr(
                        "cp1",
                        constrained_cp(cp, old, self.is_symmetric()),
                    )?;
                } else if point_ids.contains(&other_end) {
                    let old = other
                        .getattr("cp2")?
                        .extract::<Option<(f64, f64)>>()?;
                    other.setattr(
                        "cp2",
                        constrained_cp(cp, old, self.is_symmetric()),
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Re-enforces the bezier-waypoint constraint across every
    /// connected bezier at this point.
    #[pyo3(signature = (registry, sketch = None))]
    fn enforce_constraint(
        &self,
        py: Python<'_>,
        registry: PointCtx<'_>,
        sketch: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        if self.is_sharp() {
            return Ok(());
        }
        let beziers = self.get_connected_beziers(py, registry, sketch)?;
        if beziers.len() < 2 {
            return Ok(());
        }
        let mut point_ids = std::collections::HashSet::from([self.inner.id]);
        if let Some(sketch) = sketch {
            if let Ok(ids) = sketch
                .call_method1("get_coincident_points", (self.inner.id,))
                .and_then(|v| v.extract::<Vec<u64>>())
            {
                point_ids.extend(ids);
            }
        }
        let mut cp_data: Vec<(Bound<'_, PyAny>, String, (f64, f64))> =
            Vec::new();
        for b in &beziers {
            let start: u64 = b.getattr("start_idx")?.extract()?;
            let end: u64 = b.getattr("end_idx")?.extract()?;
            if point_ids.contains(&start) {
                if let Some(cp) =
                    b.getattr("cp1")?.extract::<Option<(f64, f64)>>()?
                {
                    cp_data.push((b.clone(), "cp1".to_string(), cp));
                }
            } else if point_ids.contains(&end) {
                if let Some(cp) =
                    b.getattr("cp2")?.extract::<Option<(f64, f64)>>()?
                {
                    cp_data.push((b.clone(), "cp2".to_string(), cp));
                }
            }
        }
        if cp_data.len() < 2 {
            return Ok(());
        }
        if self.is_symmetric() {
            let avg_length = cp_data
                .iter()
                .map(|(_, _, cp)| cp.0.hypot(cp.1))
                .sum::<f64>()
                / cp_data.len() as f64;
            if avg_length < 1e-10 {
                return Ok(());
            }
            let mut sum_dx = 0.0;
            let mut sum_dy = 0.0;
            for (_, attr, cp) in &cp_data {
                let sign = if attr == "cp1" { 1.0 } else { -1.0 };
                sum_dx += sign * cp.0;
                sum_dy += sign * cp.1;
            }
            let length = sum_dx.hypot(sum_dy);
            if length < 1e-10 {
                return Ok(());
            }
            let direction = (sum_dx / length, sum_dy / length);
            for (b, attr, _) in &cp_data {
                let sign = if attr == "cp1" { 1.0 } else { -1.0 };
                b.setattr(
                    attr,
                    (
                        sign * direction.0 * avg_length,
                        sign * direction.1 * avg_length,
                    ),
                )?;
            }
        }
        Ok(())
    }
}

fn constrained_cp(
    modified: (f64, f64),
    other: Option<(f64, f64)>,
    symmetric: bool,
) -> (f64, f64) {
    if symmetric {
        return (-modified.0, -modified.1);
    }
    let Some(other) = other else {
        return (-modified.0, -modified.1);
    };
    let other_length = other.0.hypot(other.1);
    if other_length < 1e-10 {
        return (-modified.0, -modified.1);
    }
    let modified_length = modified.0.hypot(modified.1);
    if modified_length < 1e-10 {
        return (0.0, 0.0);
    }
    let direction =
        (-modified.0 / modified_length, -modified.1 / modified_length);
    (direction.0 * other_length, direction.1 * other_length)
}

/// Registers the point class with the given module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySketchPoint>()?;

    let sys_modules = m.py().import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.entities.point", m)?;
    Ok(())
}
