//! Snap result model and supporting types.

use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PySet, PyTuple};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::PyEntityRegistry;
use crate::sketch::snap::types as core;

#[gen_stub_pyclass]
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "raygeo.sketch.snap.types",
    name = "SnapLineType"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PySnapLineType(pub u8);

impl PySnapLineType {
    pub const ENTITY_POINT_VALUE: u8 = 0;
    pub const ON_ENTITY_VALUE: u8 = 1;
    pub const INTERSECTION_VALUE: u8 = 2;
    pub const MIDPOINT_VALUE: u8 = 3;
    pub const EQUIDISTANT_VALUE: u8 = 4;
    pub const TANGENT_VALUE: u8 = 5;
    pub const CENTER_VALUE: u8 = 6;

    pub fn priority_value(self) -> i32 {
        match self.0 {
            Self::ENTITY_POINT_VALUE => 100,
            Self::MIDPOINT_VALUE => 90,
            Self::ON_ENTITY_VALUE => 80,
            Self::INTERSECTION_VALUE => 70,
            Self::EQUIDISTANT_VALUE => 60,
            Self::TANGENT_VALUE => 40,
            Self::CENTER_VALUE => 30,
            _ => 0,
        }
    }

    pub fn name_value(self) -> &'static str {
        match self.0 {
            Self::ENTITY_POINT_VALUE => "ENTITY_POINT",
            Self::ON_ENTITY_VALUE => "ON_ENTITY",
            Self::INTERSECTION_VALUE => "INTERSECTION",
            Self::MIDPOINT_VALUE => "MIDPOINT",
            Self::EQUIDISTANT_VALUE => "EQUIDISTANT",
            Self::TANGENT_VALUE => "TANGENT",
            Self::CENTER_VALUE => "CENTER",
            _ => "UNKNOWN",
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySnapLineType {
    #[classattr]
    pub const ENTITY_POINT: PySnapLineType =
        PySnapLineType(PySnapLineType::ENTITY_POINT_VALUE);
    #[classattr]
    pub const ON_ENTITY: PySnapLineType =
        PySnapLineType(PySnapLineType::ON_ENTITY_VALUE);
    #[classattr]
    pub const INTERSECTION: PySnapLineType =
        PySnapLineType(PySnapLineType::INTERSECTION_VALUE);
    #[classattr]
    pub const MIDPOINT: PySnapLineType =
        PySnapLineType(PySnapLineType::MIDPOINT_VALUE);
    #[classattr]
    pub const EQUIDISTANT: PySnapLineType =
        PySnapLineType(PySnapLineType::EQUIDISTANT_VALUE);
    #[classattr]
    pub const TANGENT: PySnapLineType =
        PySnapLineType(PySnapLineType::TANGENT_VALUE);
    #[classattr]
    pub const CENTER: PySnapLineType =
        PySnapLineType(PySnapLineType::CENTER_VALUE);

    #[getter]
    fn priority(&self) -> i32 {
        self.priority_value()
    }

    #[getter]
    fn name(&self) -> &'static str {
        self.name_value()
    }

    #[getter]
    fn value(&self) -> u8 {
        self.0
    }

    fn __hash__(&self) -> u64 {
        self.0 as u64
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn __repr__(&self) -> String {
        format!("SnapLineType.{}", self.name_value())
    }

    fn __str__(&self) -> String {
        self.name_value().to_string()
    }
}

#[gen_stub_pyclass]
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "raygeo.sketch.snap.types",
    name = "SnapLineStyle"
)]
#[derive(Clone, PartialEq)]
pub struct PySnapLineStyle {
    pub color: [f64; 4],
    pub dash: Option<Vec<f64>>,
    pub line_width: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySnapLineStyle {
    #[new]
    #[pyo3(signature = (color=(0.0, 0.6, 1.0, 0.8), dash=None, line_width=1.0))]
    fn new(
        color: (f64, f64, f64, f64),
        dash: Option<Vec<f64>>,
        line_width: f64,
    ) -> Self {
        Self {
            color: [color.0, color.1, color.2, color.3],
            dash,
            line_width,
        }
    }

    #[getter]
    fn color(&self) -> (f64, f64, f64, f64) {
        (self.color[0], self.color[1], self.color[2], self.color[3])
    }

    #[getter]
    fn dash(&self, py: Python<'_>) -> PyResult<Option<Py<PyTuple>>> {
        match &self.dash {
            Some(values) => {
                Ok(Some(PyTuple::new(py, values.iter().copied())?.unbind()))
            }
            None => Ok(None),
        }
    }

    #[getter]
    fn line_width(&self) -> f64 {
        self.line_width
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.color == other.color
            && self.dash == other.dash
            && self.line_width == other.line_width
    }
}

fn style(value: core::SnapLineStyle) -> PySnapLineStyle {
    PySnapLineStyle {
        color: value.color,
        dash: value.dash,
        line_width: value.line_width,
    }
}

/// Builds the SNAP_LINE_STYLES dictionary from the canonical core
/// style table.
pub(crate) fn build_line_styles(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    let styles = PyDict::new(py);
    for (kind, value) in core::style_table() {
        styles.set_item(
            Py::new(py, PySnapLineType(kind.0))?,
            Py::new(py, style(value))?,
        )?;
    }
    Ok(styles)
}

#[gen_stub_pyclass]
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "raygeo.sketch.snap.types",
    name = "SnapPoint"
)]
#[derive(Clone)]
pub struct PySnapPoint {
    pub x: f64,
    pub y: f64,
    pub line_type: PySnapLineType,
    pub source: Option<Py<PyAny>>,
    pub spacing: Option<f64>,
    pub is_horizontal: bool,
    pub pattern_coords: Option<Vec<f64>>,
    pub axis_coord: Option<f64>,
}

impl PySnapPoint {
    /// The core snap point this object describes; the source id is
    /// taken from the source object's `id` attribute when present.
    pub(crate) fn to_core(&self, py: Python<'_>) -> core::SnapPoint {
        let source = self
            .source
            .as_ref()
            .and_then(|s| s.bind(py).getattr("id").ok()?.extract::<u64>().ok());
        core::SnapPoint {
            x: self.x,
            y: self.y,
            kind: core::SnapLineType(self.line_type.0),
            source,
            spacing: self.spacing,
            is_horizontal: self.is_horizontal,
            pattern_coords: self.pattern_coords.clone(),
            axis_coord: self.axis_coord,
        }
    }

    /// Materializes a core snap point with the given source object.
    pub(crate) fn from_core(
        py: Python<'_>,
        point: core::SnapPoint,
        source: Option<Py<PyAny>>,
    ) -> PyResult<Py<PySnapPoint>> {
        Py::new(
            py,
            PySnapPoint {
                x: point.x,
                y: point.y,
                line_type: PySnapLineType(point.kind.0),
                source,
                spacing: point.spacing,
                is_horizontal: point.is_horizontal,
                pattern_coords: point.pattern_coords,
                axis_coord: point.axis_coord,
            },
        )
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySnapPoint {
    #[new]
    #[pyo3(signature = (x, y, line_type, source=None, spacing=None, is_horizontal=false, pattern_coords=None, axis_coord=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        x: f64,
        y: f64,
        line_type: PyRef<'_, PySnapLineType>,
        source: Option<Py<PyAny>>,
        spacing: Option<f64>,
        is_horizontal: bool,
        pattern_coords: Option<Vec<f64>>,
        axis_coord: Option<f64>,
    ) -> Self {
        Self {
            x,
            y,
            line_type: *line_type,
            source,
            spacing,
            is_horizontal,
            pattern_coords,
            axis_coord,
        }
    }

    #[getter]
    fn x(&self) -> f64 {
        self.x
    }
    #[getter]
    fn y(&self) -> f64 {
        self.y
    }
    #[getter]
    fn line_type(&self) -> PySnapLineType {
        self.line_type
    }
    #[getter]
    fn source(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.source.as_ref().map(|s| s.clone_ref(py))
    }
    #[getter]
    fn spacing(&self) -> Option<f64> {
        self.spacing
    }
    #[getter]
    fn is_horizontal(&self) -> bool {
        self.is_horizontal
    }
    #[getter]
    fn pattern_coords(&self, py: Python<'_>) -> PyResult<Option<Py<PyTuple>>> {
        match &self.pattern_coords {
            Some(values) => {
                Ok(Some(PyTuple::new(py, values.iter().copied())?.unbind()))
            }
            None => Ok(None),
        }
    }
    #[getter]
    fn axis_coord(&self) -> Option<f64> {
        self.axis_coord
    }
    #[getter]
    fn pos(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    fn __repr__(&self) -> String {
        format!(
            "SnapPoint(x={}, y={}, line_type=SnapLineType.{})",
            self.x,
            self.y,
            self.line_type.name_value()
        )
    }
}

#[gen_stub_pyclass]
#[pyclass(
    frozen,
    skip_from_py_object,
    module = "raygeo.sketch.snap.types",
    name = "SnapLine"
)]
#[derive(Clone)]
pub struct PySnapLine {
    pub is_horizontal: bool,
    pub coordinate: f64,
    pub line_type: PySnapLineType,
    pub source: Option<Py<PyAny>>,
}

impl PySnapLine {
    /// Materializes a core snap line with the given source object.
    pub(crate) fn from_core(
        py: Python<'_>,
        line: core::SnapLine,
        source: Option<Py<PyAny>>,
    ) -> PyResult<Py<PySnapLine>> {
        Py::new(
            py,
            PySnapLine {
                is_horizontal: line.is_horizontal,
                coordinate: line.coordinate,
                line_type: PySnapLineType(line.kind.0),
                source,
            },
        )
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySnapLine {
    #[new]
    #[pyo3(signature = (is_horizontal, coordinate, line_type, source=None))]
    fn new(
        is_horizontal: bool,
        coordinate: f64,
        line_type: PyRef<'_, PySnapLineType>,
        source: Option<Py<PyAny>>,
    ) -> Self {
        Self {
            is_horizontal,
            coordinate,
            line_type: *line_type,
            source,
        }
    }

    #[getter]
    fn is_horizontal(&self) -> bool {
        self.is_horizontal
    }
    #[getter]
    fn coordinate(&self) -> f64 {
        self.coordinate
    }
    #[getter]
    fn line_type(&self) -> PySnapLineType {
        self.line_type
    }
    #[getter]
    fn source(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.source.as_ref().map(|s| s.clone_ref(py))
    }

    #[getter]
    fn style(&self) -> PySnapLineStyle {
        style_for(self.line_type)
    }

    pub(crate) fn distance_to(&self, x: f64, y: f64) -> f64 {
        if self.is_horizontal {
            (y - self.coordinate).abs()
        } else {
            (x - self.coordinate).abs()
        }
    }

    fn get_snap_position(&self, x: f64, y: f64) -> (f64, f64) {
        if self.is_horizontal {
            (x, self.coordinate)
        } else {
            (self.coordinate, y)
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "SnapLine(is_horizontal={}, coordinate={}, line_type=SnapLineType.{})",
            self.is_horizontal,
            self.coordinate,
            self.line_type.name_value()
        )
    }
}

#[gen_stub_pyclass]
#[pyclass(
    skip_from_py_object,
    module = "raygeo.sketch.snap.types",
    name = "SnapResult"
)]
#[derive(Clone)]
pub struct PySnapResult {
    pub snapped: bool,
    pub position: (f64, f64),
    pub snap_lines: Vec<Py<PySnapLine>>,
    pub snap_points: Vec<Py<PySnapPoint>>,
    pub primary_snap_line: Option<Py<PySnapLine>>,
    pub primary_snap_point: Option<Py<PySnapPoint>>,
    pub distance: f64,
}

#[gen_stub_pymethods]
#[pymethods]
impl PySnapResult {
    #[new]
    #[pyo3(signature = (snapped=false, position=(0.0, 0.0), snap_lines=None, snap_points=None, primary_snap_line=None, primary_snap_point=None, distance=f64::INFINITY))]
    fn new(
        snapped: bool,
        position: (f64, f64),
        snap_lines: Option<Vec<Py<PySnapLine>>>,
        snap_points: Option<Vec<Py<PySnapPoint>>>,
        primary_snap_line: Option<Py<PySnapLine>>,
        primary_snap_point: Option<Py<PySnapPoint>>,
        distance: f64,
    ) -> Self {
        Self {
            snapped,
            position,
            snap_lines: snap_lines.unwrap_or_default(),
            snap_points: snap_points.unwrap_or_default(),
            primary_snap_line,
            primary_snap_point,
            distance,
        }
    }

    #[classmethod]
    fn no_snap(
        _cls: &Bound<'_, pyo3::types::PyType>,
        position: (f64, f64),
    ) -> Self {
        Self::no_snap_value(position)
    }

    #[classmethod]
    fn from_snap_line(
        _cls: &Bound<'_, pyo3::types::PyType>,
        py: Python<'_>,
        snap_line: Py<PySnapLine>,
        original_pos: (f64, f64),
        distance: f64,
    ) -> Self {
        let position = snap_line
            .borrow(py)
            .get_snap_position(original_pos.0, original_pos.1);
        Self {
            snapped: true,
            position,
            snap_lines: vec![snap_line.clone_ref(py)],
            snap_points: Vec::new(),
            primary_snap_line: Some(snap_line),
            primary_snap_point: None,
            distance,
        }
    }

    #[classmethod]
    #[pyo3(signature = (snap_point, distance, snap_lines=None))]
    fn from_snap_point(
        _cls: &Bound<'_, pyo3::types::PyType>,
        py: Python<'_>,
        snap_point: Py<PySnapPoint>,
        distance: f64,
        snap_lines: Option<Vec<Py<PySnapLine>>>,
    ) -> Self {
        let position = {
            let point = snap_point.borrow(py);
            (point.x, point.y)
        };
        Self {
            snapped: true,
            position,
            snap_lines: snap_lines.unwrap_or_default(),
            snap_points: vec![snap_point.clone_ref(py)],
            primary_snap_line: None,
            primary_snap_point: Some(snap_point),
            distance,
        }
    }

    #[getter]
    fn snapped(&self) -> bool {
        self.snapped
    }
    #[getter]
    fn position(&self) -> (f64, f64) {
        self.position
    }
    #[getter]
    fn snap_lines(&self, py: Python<'_>) -> Vec<Py<PySnapLine>> {
        self.snap_lines.iter().map(|v| v.clone_ref(py)).collect()
    }
    #[getter]
    fn snap_points(&self, py: Python<'_>) -> Vec<Py<PySnapPoint>> {
        self.snap_points.iter().map(|v| v.clone_ref(py)).collect()
    }
    #[getter]
    fn primary_snap_line(&self, py: Python<'_>) -> Option<Py<PySnapLine>> {
        self.primary_snap_line.as_ref().map(|v| v.clone_ref(py))
    }
    #[getter]
    fn primary_snap_point(&self, py: Python<'_>) -> Option<Py<PySnapPoint>> {
        self.primary_snap_point.as_ref().map(|v| v.clone_ref(py))
    }
    #[getter]
    fn distance(&self) -> f64 {
        self.distance
    }

    #[setter]
    fn set_primary_snap_point(&mut self, value: Option<Py<PySnapPoint>>) {
        self.primary_snap_point = value;
    }

    #[setter]
    fn set_primary_snap_line(&mut self, value: Option<Py<PySnapLine>>) {
        self.primary_snap_line = value;
    }

    #[setter]
    fn set_snap_lines(&mut self, value: Vec<Py<PySnapLine>>) {
        self.snap_lines = value;
    }

    #[setter]
    fn set_snap_points(&mut self, value: Vec<Py<PySnapPoint>>) {
        self.snap_points = value;
    }

    #[setter]
    fn set_snapped(&mut self, value: bool) {
        self.snapped = value;
    }

    #[setter]
    fn set_position(&mut self, value: (f64, f64)) {
        self.position = value;
    }

    #[setter]
    fn set_distance(&mut self, value: f64) {
        self.distance = value;
    }
}

#[gen_stub_pyclass]
#[pyclass(
    skip_from_py_object,
    module = "raygeo.sketch.snap.types",
    name = "DragContext"
)]
#[derive(Clone)]
pub struct PyDragContext {
    pub dragged_point_ids: Py<PySet>,
    pub dragged_entity_ids: Py<PySet>,
    pub preview_entity_ids: Py<PySet>,
    pub initial_positions: HashMap<u64, (f64, f64)>,
    pub suppress_coincident_points: bool,
}

impl Default for PyDragContext {
    fn default() -> Self {
        Python::attach(|py| Self {
            dragged_point_ids: PySet::empty(py).unwrap().unbind(),
            dragged_entity_ids: PySet::empty(py).unwrap().unbind(),
            preview_entity_ids: PySet::empty(py).unwrap().unbind(),
            initial_positions: HashMap::new(),
            suppress_coincident_points: true,
        })
    }
}

impl PyDragContext {
    /// The core snap context this drag context describes.
    pub(crate) fn to_context(&self) -> core::SnapContext {
        core::SnapContext {
            dragged_point_ids: self.point_ids().into_iter().collect(),
            dragged_entity_ids: self.entity_ids().into_iter().collect(),
            preview_entity_ids: self.preview_ids().into_iter().collect(),
            suppress_coincident_points: self.suppress_coincident_points,
        }
    }

    pub(crate) fn point_ids(&self) -> HashSet<u64> {
        Python::attach(|py| {
            self.dragged_point_ids
                .bind(py)
                .iter()
                .filter_map(|v| v.extract::<u64>().ok())
                .collect()
        })
    }

    pub(crate) fn entity_ids(&self) -> HashSet<u64> {
        Python::attach(|py| {
            self.dragged_entity_ids
                .bind(py)
                .iter()
                .filter_map(|v| v.extract::<u64>().ok())
                .collect()
        })
    }

    pub(crate) fn preview_ids(&self) -> HashSet<u64> {
        Python::attach(|py| {
            self.preview_entity_ids
                .bind(py)
                .iter()
                .filter_map(|v| v.extract::<u64>().ok())
                .collect()
        })
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDragContext {
    #[new]
    #[pyo3(signature = (dragged_point_ids=None, dragged_entity_ids=None, preview_entity_ids=None, initial_positions=None, suppress_coincident_points=true))]
    fn new(
        py: Python<'_>,
        dragged_point_ids: Option<HashSet<u64>>,
        dragged_entity_ids: Option<HashSet<u64>>,
        preview_entity_ids: Option<HashSet<u64>>,
        initial_positions: Option<HashMap<u64, (f64, f64)>>,
        suppress_coincident_points: bool,
    ) -> PyResult<Self> {
        Ok(Self {
            dragged_point_ids: PySet::new(
                py,
                dragged_point_ids.unwrap_or_default().iter(),
            )?
            .unbind(),
            dragged_entity_ids: PySet::new(
                py,
                dragged_entity_ids.unwrap_or_default().iter(),
            )?
            .unbind(),
            preview_entity_ids: PySet::new(
                py,
                preview_entity_ids.unwrap_or_default().iter(),
            )?
            .unbind(),
            initial_positions: initial_positions.unwrap_or_default(),
            suppress_coincident_points,
        })
    }

    pub(crate) fn is_point_dragged(&self, point_id: u64) -> bool {
        self.point_ids().contains(&point_id)
    }

    pub(crate) fn is_entity_dragged(&self, entity_id: u64) -> bool {
        self.entity_ids().contains(&entity_id)
    }

    pub(crate) fn is_preview_entity(&self, entity_id: u64) -> bool {
        self.preview_ids().contains(&entity_id)
    }

    #[pyo3(signature = (x, y, registry, epsilon=1e-6))]
    fn coincides_with_dragged(
        &self,
        x: f64,
        y: f64,
        registry: &Bound<'_, PyAny>,
        epsilon: f64,
    ) -> PyResult<bool> {
        if !self.suppress_coincident_points {
            return Ok(false);
        }
        let registry = registry.extract::<PyRef<'_, PyEntityRegistry>>()?;
        for pid in &self.point_ids() {
            if let Some(point) = registry.inner.get_point(*pid) {
                if (point.x - x).abs() <= epsilon
                    && (point.y - y).abs() <= epsilon
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    #[getter]
    fn dragged_point_ids(&self, py: Python<'_>) -> Py<PySet> {
        self.dragged_point_ids.clone_ref(py)
    }
    #[getter]
    fn dragged_entity_ids(&self, py: Python<'_>) -> Py<PySet> {
        self.dragged_entity_ids.clone_ref(py)
    }
    #[getter]
    fn preview_entity_ids(&self, py: Python<'_>) -> Py<PySet> {
        self.preview_entity_ids.clone_ref(py)
    }
    #[getter]
    fn initial_positions(&self) -> HashMap<u64, (f64, f64)> {
        self.initial_positions.clone()
    }
    #[getter]
    fn suppress_coincident_points(&self) -> bool {
        self.suppress_coincident_points
    }
}

impl PySnapResult {
    pub(crate) fn no_snap_value(position: (f64, f64)) -> Self {
        Self {
            snapped: false,
            position,
            snap_lines: Vec::new(),
            snap_points: Vec::new(),
            primary_snap_line: None,
            primary_snap_point: None,
            distance: f64::INFINITY,
        }
    }
}

fn style_for(kind: PySnapLineType) -> PySnapLineStyle {
    style(core::style_for(core::SnapLineType(kind.0)))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySnapLineType>()?;
    module.add_class::<PySnapLineStyle>()?;
    module.add_class::<PySnapPoint>()?;
    module.add_class::<PySnapLine>()?;
    module.add_class::<PySnapResult>()?;
    module.add_class::<PyDragContext>()?;
    module.add("SNAP_LINE_STYLES", build_line_styles(module.py())?)?;
    Ok(())
}
