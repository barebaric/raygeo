//! Snap engine and producer base class.

use std::collections::HashSet;

use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyTuple};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;

use super::spatial::PySnapLineIndex;
use super::types::{
    PyDragContext, PySnapLine, PySnapLineType, PySnapPoint, PySnapResult,
};

#[gen_stub_pyclass]
#[pyclass(subclass, skip_from_py_object, module = "raygeo.sketch.snap.engine", name = "SnapLineProducer")]
pub struct PySnapLineProducer;

#[gen_stub_pymethods]
#[pymethods]
impl PySnapLineProducer {
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(
        _args: &Bound<'_, pyo3::types::PyTuple>,
        _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>,
    ) -> Self {
        Self
    }

    fn produce(
        &self,
        _registry: &Bound<'_, PyAny>,
        _drag_position: (f64, f64),
        _drag_context: &Bound<'_, PyAny>,
        _threshold: f64,
    ) -> PyResult<()> {
        Err(PyNotImplementedError::new_err("produce"))
    }

    fn produce_points(
        &self,
        _registry: &Bound<'_, PyAny>,
        _drag_position: (f64, f64),
        _drag_context: &Bound<'_, PyAny>,
        _threshold: f64,
    ) -> PyResult<Vec<PySnapPoint>> {
        Ok(Vec::new())
    }
}

fn source_key(py: Python<'_>, source: Option<&Py<PyAny>>) -> (u8, usize) {
    match source {
        Some(source) => {
            if let Ok(id) = source.bind(py).getattr("id") {
                if let Ok(value) = id.extract::<usize>() {
                    return (0, value);
                }
            }
            (1, source.as_ptr() as usize)
        }
        None => (2, 0),
    }
}

fn line_type_key(kind: PySnapLineType) -> u8 {
    kind.0
}

/// Stable identity of the geometry a snap index depends on.
#[pyfunction]
#[pyo3(signature = (registry, drag_context=None, include_construction=true, include_points=true))]
pub(crate) fn geometry_key(
    py: Python<'_>,
    registry: &Bound<'_, PyAny>,
    drag_context: Option<&Bound<'_, PyAny>>,
    include_construction: bool,
    include_points: bool,
) -> PyResult<Py<PyAny>> {
    let registry = registry.extract::<PyRef<'_, PyEntityRegistry>>()?;
    let ctx = match drag_context {
        Some(value) if !value.is_none() => Some(value.extract::<PyRef<'_, PyDragContext>>()?),
        _ => None,
    };
    let empty = PyDragContext::default();
    let ctx = ctx.as_deref().unwrap_or(&empty);

    let mut excluded_entities: HashSet<u64> = ctx.entity_ids();
    excluded_entities.extend(ctx.preview_ids());
    let mut excluded_points = ctx.point_ids();
    for entity_id in &excluded_entities {
        if let Some(ids) = registry.inner.entity_point_ids(*entity_id) {
            excluded_points.extend(ids);
        }
    }

    let mut entities = Vec::new();
    for entity_id in registry.inner.entity_ids() {
        if excluded_entities.contains(&entity_id) {
            continue;
        }
        let Some(entity) = registry.inner.get_entity(entity_id) else {
            continue;
        };
        if entity.get_point_ids().iter().any(|p| excluded_points.contains(p)) {
            continue;
        }
        if !include_construction && entity.base().construction {
            continue;
        }
        entities.push((
            entity_id,
            entity.type_name().to_string(),
            entity.base().construction,
            entity.base().invisible,
            entity.geometry_signature(&registry.inner).to_string(),
        ));
    }

    let mut parts: Vec<Py<PyAny>> = Vec::new();
    parts.push(
        PyTuple::new(py, entities.iter().map(|e| format!("{e:?}")))?
            .into_any()
            .unbind(),
    );
    if include_points {
        let points: Vec<(u64, f64, f64)> = registry
            .inner
            .point_ids()
            .into_iter()
            .filter(|id| !excluded_points.contains(id))
            .filter_map(|id| registry.inner.get_point(id))
            .map(|p| (p.id, (p.x * 1e6).round() / 1e6, (p.y * 1e6).round() / 1e6))
            .collect();
        parts.push(PyTuple::new(py, points.iter().map(|p| format!("{p:?}")))?.into_any().unbind());
    }
    Ok(PyTuple::new(py, parts)?.into_any().unbind())
}

fn collect_lines(
    py: Python<'_>,
    producers: &[Py<PyAny>],
    registry: &Bound<'_, PyAny>,
    position: (f64, f64),
    context: &Bound<'_, PyAny>,
    threshold: f64,
) -> PyResult<Vec<Py<PySnapLine>>> {
    let mut lines = Vec::new();
    for producer in producers {
        let result = producer.call_method1(
            py,
            "produce",
            (registry, position, context, threshold),
        )?;
        for item in result.bind(py).try_iter()? {
            lines.push(item?.extract::<Py<PySnapLine>>()?);
        }
    }
    Ok(lines)
}

fn collect_points(
    py: Python<'_>,
    producers: &[Py<PyAny>],
    registry: &Bound<'_, PyAny>,
    position: (f64, f64),
    context: &Bound<'_, PyAny>,
    threshold: f64,
) -> PyResult<Vec<Py<PySnapPoint>>> {
    let mut points = Vec::new();
    for producer in producers {
        let result = producer.call_method1(
            py,
            "produce_points",
            (registry, position, context, threshold),
        )?;
        for item in result.bind(py).try_iter()? {
            points.push(item?.extract::<Py<PySnapPoint>>()?);
        }
    }
    Ok(points)
}

#[gen_stub_pyclass]
#[pyclass(skip_from_py_object, module = "raygeo.sketch.snap.engine", name = "SnapEngine")]
pub struct PySnapEngine {
    pub producers: Vec<Py<PyAny>>,
    pub threshold: f64,
    pub index: PySnapLineIndex,
    pub last_query_pos: Option<(f64, f64)>,
    pub enabled: bool,
    pub cached_fingerprint: Option<String>,
    pub cached_full_fingerprint: Option<String>,
    pub cached_drag_context_key: Option<(HashSet<u64>, HashSet<u64>)>,
    pub last_primary: Option<Py<PySnapPoint>>,
}

impl Default for PySnapEngine {
    fn default() -> Self {
        Self {
            producers: Vec::new(),
            threshold: 5.0,
            index: PySnapLineIndex::default(),
            last_query_pos: None,
            enabled: true,
            cached_fingerprint: None,
            cached_full_fingerprint: None,
            cached_drag_context_key: None,
            last_primary: None,
        }
    }
}

type BestLines = (Option<Py<PySnapLine>>, Option<Py<PySnapLine>>);

#[gen_stub_pymethods]
#[pymethods]
impl PySnapEngine {
    #[new]
    #[pyo3(signature = (threshold=5.0))]
    fn new(threshold: f64) -> Self {
        Self {
            threshold,
            ..Self::default()
        }
    }

    #[classattr]
    const DEFAULT_THRESHOLD: f64 = 5.0;

    #[classattr]
    const PRIMARY_EXIT_FACTOR: f64 = 1.5;

    #[getter]
    fn enabled(&self) -> bool {
        self.enabled
    }
    #[setter]
    fn set_enabled(&mut self, value: bool) {
        self.enabled = value;
    }

    #[getter]
    fn threshold(&self) -> f64 {
        self.threshold
    }
    #[setter]
    fn set_threshold(&mut self, value: f64) {
        self.threshold = value;
    }

    #[getter]
    fn _producers(&self, py: Python<'_>) -> Vec<Py<PyAny>> {
        self.producers.iter().map(|p| p.clone_ref(py)).collect()
    }

    #[getter]
    fn _last_query_pos(&self) -> Option<(f64, f64)> {
        self.last_query_pos
    }

    #[getter]
    fn _index(&self) -> PySnapLineIndex {
        self.index.clone()
    }

    fn register_producer(&mut self, producer: Py<PyAny>) {
        self.producers.push(producer);
    }

    fn unregister_producer(&mut self, py: Python<'_>, producer: &Bound<'_, PyAny>) {
        self.producers.retain(|item| !item.bind(py).is(producer));
    }

    fn clear_producers(&mut self) {
        self.producers.clear();
    }

    #[pyo3(signature = (registry, drag_position, drag_context))]
    fn rebuild_index(
        &mut self,
        py: Python<'_>,
        registry: &Bound<'_, PyAny>,
        drag_position: (f64, f64),
        drag_context: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let full_key = geometry_key(
            py,
            registry,
            Some(drag_context),
            true,
            true,
        )?;
        let full_str = full_key.bind(py).repr()?.to_string();
        let context = drag_context.extract::<PyRef<'_, PyDragContext>>()?;
        let drag_key = (context.point_ids(), context.entity_ids());
        if self.cached_fingerprint.as_deref() == Some(full_str.as_str())
            && self.cached_drag_context_key.as_ref() == Some(&drag_key)
        {
            return Ok(());
        }

        self.index.clear();
        let ps: Vec<Py<PyAny>> = self
            .producers
            .iter()
            .map(|p| p.clone_ref(py))
            .collect();
        let lines = collect_lines(
            py,
            &ps,
            registry,
            drag_position,
            drag_context,
            self.threshold,
        )?;
        for line in lines {
            self.index.add_line(py, line);
        }
        self.last_query_pos = Some(drag_position);
        self.cached_fingerprint = Some(full_str);
        self.cached_full_fingerprint = Some(
            geometry_key(py, registry, None, true, true)?
                .bind(py)
                .repr()?
                .to_string(),
        );
        self.cached_drag_context_key = Some(drag_key);
        Ok(())
    }

    #[pyo3(signature = (registry, position, drag_context=None))]
    fn query(
        &mut self,
        py: Python<'_>,
        registry: &Bound<'_, PyAny>,
        position: (f64, f64),
        drag_context: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PySnapResult>> {
        if !self.enabled {
            return Py::new(py, PySnapResult::no_snap_value(position));
        }
        let default_context = Py::new(py, PyDragContext::default())?;
        let context_obj = match drag_context {
            Some(value) => value.clone().into_any(),
            None => default_context.bind(py).clone().into_any(),
        };
        self.rebuild_index(py, registry, position, &context_obj)?;
        self.cached_full_fingerprint = Some(
            geometry_key(py, registry, None, true, true)?
                .bind(py)
                .repr()?
                .to_string(),
        );
        let dynamic_lines = dynamic_lines(py, registry, &context_obj)?;

        let exit_radius = if self.last_primary.is_some() {
            self.threshold * 1.5
        } else {
            self.threshold
        };
        let context_ref = context_obj.extract::<PyRef<'_, PyDragContext>>()?;
        let _ = context_ref;
        let ps: Vec<Py<PyAny>> = self
            .producers
            .iter()
            .map(|p| p.clone_ref(py))
            .collect();
        let snap_points = collect_points(
            py,
            &ps,
            registry,
            position,
            &context_obj,
            exit_radius,
        )?;

        let (x, y) = position;
        if let Some((point, distance)) =
            self.find_nearest_snap_point(py, x, y, &snap_points)?
        {
            let crossing = self.find_crossing_lines(py, &point, &dynamic_lines)?;
            self.last_primary = Some(point.clone_ref(py));
            let borrowed = point.borrow(py);
            let result = PySnapResult {
                snapped: true,
                position: (borrowed.x, borrowed.y),
                snap_lines: crossing,
                snap_points: vec![point.clone_ref(py)],
                primary_snap_line: None,
                primary_snap_point: Some(point.clone_ref(py)),
                distance,
            };
            return Py::new(py, result);
        }
        self.last_primary = None;

        let (best_h, best_v) = self.find_best_lines(py, x, y, &dynamic_lines)?;
        if best_h.is_some() || best_v.is_some() {
            let snap_x = best_v
                .as_ref()
                .map(|v| v.borrow(py).coordinate)
                .unwrap_or(x);
            let snap_y = best_h
                .as_ref()
                .map(|v| v.borrow(py).coordinate)
                .unwrap_or(y);
            let mut lines = Vec::new();
            if let Some(line) = &best_h {
                lines.push(line.clone_ref(py));
            }
            if let Some(line) = &best_v {
                lines.push(line.clone_ref(py));
            }
            let dist = best_h
                .as_ref()
                .map(|v| v.borrow(py).distance_to(x, y))
                .unwrap_or(0.0)
                .max(
                    best_v
                        .as_ref()
                        .map(|v| v.borrow(py).distance_to(x, y))
                        .unwrap_or(0.0),
                );
            return Py::new(
                py,
                PySnapResult {
                    snapped: true,
                    position: (snap_x, snap_y),
                    snap_lines: lines,
                    snap_points: Vec::new(),
                    primary_snap_line: None,
                    primary_snap_point: None,
                    distance: dist,
                },
            );
        }

        Py::new(py, PySnapResult::no_snap_value(position))
    }

    #[pyo3(signature = (registry, position, drag_context=None))]
    fn get_visible_snap_lines(
        &mut self,
        py: Python<'_>,
        registry: &Bound<'_, PyAny>,
        position: (f64, f64),
        drag_context: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Vec<Py<PySnapLine>>> {
        if !self.enabled {
            return Ok(Vec::new());
        }
        let default_context = Py::new(py, PyDragContext::default())?;
        let context_obj = match drag_context {
            Some(value) => value.clone().into_any(),
            None => default_context.bind(py).clone().into_any(),
        };
        self.rebuild_index(py, registry, position, &context_obj)?;
        let mut lines = dynamic_lines(py, registry, &context_obj)?;
        for indexed in self.index.horizontal.iter().chain(self.index.vertical.iter()) {
            if let Some(line) = &indexed.snap_line {
                lines.push(line.clone_ref(py));
            }
        }
        Ok(lines)
    }

    fn is_snap_result_current(
        &self,
        py: Python<'_>,
        registry: &Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        let current = geometry_key(py, registry, None, true, true)?
            .bind(py)
            .repr()?
            .to_string();
        Ok(self.cached_full_fingerprint.as_deref() == Some(current.as_str()))
    }

    fn invalidate_index(&mut self) {
        self.cached_fingerprint = None;
        self.cached_full_fingerprint = None;
        self.cached_drag_context_key = None;
    }
}

impl PySnapEngine {
    fn find_best_lines(
        &self,
        py: Python<'_>,
        x: f64,
        y: f64,
        extra: &[Py<PySnapLine>],
    ) -> PyResult<BestLines> {
        let mut best_h = None;
        let mut best_h_dist = self.threshold;
        let mut best_v = None;
        let mut best_v_dist = self.threshold;
        for indexed in &self.index.horizontal {
            if let Some(line) = &indexed.snap_line {
                let dist = (y - indexed.coordinate).abs();
                if dist < best_h_dist {
                    best_h_dist = dist;
                    best_h = Some(line.clone_ref(py));
                }
            }
        }
        for indexed in &self.index.vertical {
            if let Some(line) = &indexed.snap_line {
                let dist = (x - indexed.coordinate).abs();
                if dist < best_v_dist {
                    best_v_dist = dist;
                    best_v = Some(line.clone_ref(py));
                }
            }
        }
        for line in extra {
            let borrowed = line.borrow(py);
            if borrowed.is_horizontal {
                let dist = (y - borrowed.coordinate).abs();
                if dist < best_h_dist {
                    best_h_dist = dist;
                    best_h = Some(line.clone_ref(py));
                }
            } else {
                let dist = (x - borrowed.coordinate).abs();
                if dist < best_v_dist {
                    best_v_dist = dist;
                    best_v = Some(line.clone_ref(py));
                }
            }
        }
        Ok((best_h, best_v))
    }

    fn find_nearest_snap_point(
        &mut self,
        py: Python<'_>,
        x: f64,
        y: f64,
        points: &[Py<PySnapPoint>],
    ) -> PyResult<Option<(Py<PySnapPoint>, f64)>> {
        let mut best: Option<Py<PySnapPoint>> = None;
        let mut best_dist = self.threshold;
        let mut best_priority = -1;
        for point in points {
            let p = point.borrow(py);
            let dist = ((x - p.x).powi(2) + (y - p.y).powi(2)).sqrt();
            let priority = p.line_type.priority_value();
            if dist > self.threshold {
                continue;
            }
            if priority > best_priority
                || (priority == best_priority && dist < best_dist)
            {
                best_dist = dist;
                best = Some(point.clone_ref(py));
                best_priority = priority;
            }
        }
        if best.is_some() {
            return Ok(best.map(|p| (p, best_dist)));
        }
        if let Some(last) = &self.last_primary {
            let exit_radius = self.threshold * 1.5;
            for point in points {
                let p = point.borrow(py);
                let last_ref = last.borrow(py);
                if line_type_key(p.line_type) != line_type_key(last_ref.line_type) {
                    continue;
                }
                if source_key(py, p.source.as_ref())
                    != source_key(py, last_ref.source.as_ref())
                {
                    continue;
                }
                let dist = ((x - p.x).powi(2) + (y - p.y).powi(2)).sqrt();
                if dist <= exit_radius {
                    return Ok(Some((point.clone_ref(py), dist)));
                }
            }
            self.last_primary = None;
        }
        Ok(None)
    }

    fn find_crossing_lines(
        &self,
        py: Python<'_>,
        snap_point: &Py<PySnapPoint>,
        extra: &[Py<PySnapLine>],
    ) -> PyResult<Vec<Py<PySnapLine>>> {
        let point = snap_point.borrow(py);
        let mut lines = Vec::new();
        for line in extra {
            push_crossing(py, &mut lines, line, point.x, point.y);
        }
        for indexed in self.index.horizontal.iter().chain(self.index.vertical.iter()) {
            if let Some(line) = &indexed.snap_line {
                push_crossing(py, &mut lines, line, point.x, point.y);
            }
        }
        Ok(lines)
    }
}

fn push_crossing(
    py: Python<'_>,
    lines: &mut Vec<Py<PySnapLine>>,
    line: &Py<PySnapLine>,
    x: f64,
    y: f64,
) {
    let borrowed = line.borrow(py);
    let cross = if borrowed.is_horizontal {
        (borrowed.coordinate - y).abs() < 1e-6
    } else {
        (borrowed.coordinate - x).abs() < 1e-6
    };
    if cross {
        lines.push(line.clone_ref(py));
    }
}

fn dynamic_lines(
    py: Python<'_>,
    registry: &Bound<'_, PyAny>,
    context: &Bound<'_, PyAny>,
) -> PyResult<Vec<Py<PySnapLine>>> {
    let registry = registry.extract::<PyRef<'_, PyEntityRegistry>>()?;
    let context = context.extract::<PyRef<'_, PyDragContext>>()?;
    let mut lines = Vec::new();
    for entity_id in &context.preview_ids() {
        let Some(entity) = registry.inner.get_entity(*entity_id) else {
            continue;
        };
        for point_id in entity.get_point_ids() {
            if context.is_point_dragged(point_id) {
                continue;
            }
            let Some(point) = registry.inner.get_point(point_id) else {
                continue;
            };
            lines.push(Py::new(
                py,
                PySnapLine {
                    is_horizontal: false,
                    coordinate: point.x,
                    line_type: PySnapLineType(PySnapLineType::ENTITY_POINT_VALUE),
                    source: None,
                },
            )?);
            lines.push(Py::new(
                py,
                PySnapLine {
                    is_horizontal: true,
                    coordinate: point.y,
                    line_type: PySnapLineType(PySnapLineType::ENTITY_POINT_VALUE),
                    source: None,
                },
            )?);
        }
    }
    Ok(lines)
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySnapLineProducer>()?;
    module.add_class::<PySnapEngine>()?;
    module.add_function(wrap_pyfunction!(geometry_key, module)?)?;
    Ok(())
}
