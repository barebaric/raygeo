//! Python bindings for sketch array definitions.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::python::sketch::registry::{
    value_from_dict, value_to_py, PyEntityRegistry,
};
use crate::python::sketch::types::EntityId;
use crate::sketch::arrays::{
    ArrayDefinition as CoreDefinition, CircularArray, CircularArrayStrategy,
    CurveAlongArray, CurveAlongArrayStrategy, InstancePlacement, PlacementKind,
};

type Members = Vec<(i64, Vec<EntityId>)>;
type Anchor = Option<((f64, f64), f64)>;

/// A rigid placement of one array instance.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "InstancePlacement",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyInstancePlacement {
    inner: InstancePlacement,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyInstancePlacement {
    /// Placement kind: ``rotation``, ``translation`` or
    /// ``curve_aligned``.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.inner.kind {
            PlacementKind::Rotation => "rotation",
            PlacementKind::Translation => "translation",
            PlacementKind::CurveAligned => "curve_aligned",
        }
    }

    #[getter]
    fn angle(&self) -> f64 {
        self.inner.angle
    }

    #[getter]
    fn center(&self) -> (f64, f64) {
        self.inner.center
    }

    #[getter]
    fn delta(&self) -> (f64, f64) {
        self.inner.delta
    }

    #[getter]
    fn target_center(&self) -> (f64, f64) {
        self.inner.target_center
    }

    /// Transforms an absolute point through the placement.
    fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        self.inner.transform_point(x, y)
    }

    /// Transforms a relative offset through the placement.
    fn transform_offset(&self, dx: f64, dy: f64) -> (f64, f64) {
        self.inner.transform_offset(dx, dy)
    }
}

/// Circular-array placement strategy.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "CircularArrayStrategy",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCircularArrayStrategy {
    inner: CircularArrayStrategy,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCircularArrayStrategy {
    #[new]
    #[pyo3(signature = (
        count = 6,
        rotate_copies = true,
        total_angle_deg = 360.0,
        center = (0.0, 0.0),
        radius = 0.0
    ))]
    fn new(
        count: u32,
        rotate_copies: bool,
        total_angle_deg: f64,
        center: (f64, f64),
        radius: f64,
    ) -> Self {
        PyCircularArrayStrategy {
            inner: CircularArrayStrategy {
                count,
                rotate_copies,
                total_angle_deg,
                center,
                radius,
            },
        }
    }

    #[getter]
    fn count(&self) -> u32 {
        self.inner.count
    }

    #[setter]
    fn set_count(&mut self, value: u32) {
        self.inner.count = value;
    }

    #[getter]
    fn rotate_copies(&self) -> bool {
        self.inner.rotate_copies
    }

    #[setter]
    fn set_rotate_copies(&mut self, value: bool) {
        self.inner.rotate_copies = value;
    }

    #[getter]
    fn total_angle_deg(&self) -> f64 {
        self.inner.total_angle_deg
    }

    #[setter]
    fn set_total_angle_deg(&mut self, value: f64) {
        self.inner.total_angle_deg = value;
    }

    #[getter]
    fn center(&self) -> (f64, f64) {
        self.inner.center
    }

    #[setter]
    fn set_center(&mut self, value: (f64, f64)) {
        self.inner.center = value;
    }

    #[getter]
    fn radius(&self) -> f64 {
        self.inner.radius
    }

    #[setter]
    fn set_radius(&mut self, value: f64) {
        self.inner.radius = value;
    }

    fn template_placement(
        &self,
        template_center: (f64, f64),
    ) -> PyInstancePlacement {
        PyInstancePlacement {
            inner: self.inner.template_placement(template_center),
        }
    }

    fn member_placements(
        &self,
        template_center: (f64, f64),
    ) -> Vec<PyInstancePlacement> {
        self.inner
            .member_placements(template_center)
            .into_iter()
            .map(|inner| PyInstancePlacement { inner })
            .collect()
    }
}

/// Curve-along placement strategy.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "CurveAlongArrayStrategy",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCurveAlongArrayStrategy {
    inner: CurveAlongArrayStrategy,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCurveAlongArrayStrategy {
    #[new]
    #[pyo3(signature = (
        count = 6,
        rotate_copies = true,
        path_entity_id = -1,
        align_to_tangent = true,
        offset_to_start = 0.0,
        spacing = 0.0
    ))]
    fn new(
        count: u32,
        rotate_copies: bool,
        path_entity_id: i64,
        align_to_tangent: bool,
        offset_to_start: f64,
        spacing: f64,
    ) -> Self {
        PyCurveAlongArrayStrategy {
            inner: CurveAlongArrayStrategy {
                count,
                rotate_copies,
                path_entity_id,
                align_to_tangent,
                offset_to_start,
                spacing,
            },
        }
    }

    #[getter]
    fn count(&self) -> u32 {
        self.inner.count
    }

    #[setter]
    fn set_count(&mut self, value: u32) {
        self.inner.count = value;
    }

    #[getter]
    fn rotate_copies(&self) -> bool {
        self.inner.rotate_copies
    }

    #[setter]
    fn set_rotate_copies(&mut self, value: bool) {
        self.inner.rotate_copies = value;
    }

    #[getter]
    fn path_entity_id(&self) -> i64 {
        self.inner.path_entity_id
    }

    #[setter]
    fn set_path_entity_id(&mut self, value: i64) {
        self.inner.path_entity_id = value;
    }

    #[getter]
    fn align_to_tangent(&self) -> bool {
        self.inner.align_to_tangent
    }

    #[setter]
    fn set_align_to_tangent(&mut self, value: bool) {
        self.inner.align_to_tangent = value;
    }

    #[getter]
    fn offset_to_start(&self) -> f64 {
        self.inner.offset_to_start
    }

    #[setter]
    fn set_offset_to_start(&mut self, value: f64) {
        self.inner.offset_to_start = value;
    }

    #[getter]
    fn spacing(&self) -> f64 {
        self.inner.spacing
    }

    #[setter]
    fn set_spacing(&mut self, value: f64) {
        self.inner.spacing = value;
    }

    fn template_placement(
        &self,
        template_center: (f64, f64),
        registry: PyRef<'_, PyEntityRegistry>,
    ) -> PyInstancePlacement {
        PyInstancePlacement {
            inner: self
                .inner
                .template_placement(template_center, &registry.inner),
        }
    }

    fn member_placements(
        &self,
        registry: PyRef<'_, PyEntityRegistry>,
    ) -> Vec<PyInstancePlacement> {
        self.inner
            .member_placements(&registry.inner)
            .into_iter()
            .map(|inner| PyInstancePlacement { inner })
            .collect()
    }
}

/// Persistent definition of a sketch array.
#[gen_stub_pyclass]
#[pyclass(
    module = "raygeo.sketch.arrays",
    name = "ArrayDefinition",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyArrayDefinition {
    inner: CoreDefinition,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyArrayDefinition {
    #[staticmethod]
    #[pyo3(signature = (
        uid,
        guide_circle_id,
        members = None,
        count = 6,
        total_angle_deg = 360.0,
        rotate_copies = true
    ))]
    fn circular(
        uid: String,
        guide_circle_id: EntityId,
        members: Option<Members>,
        count: u32,
        total_angle_deg: f64,
        rotate_copies: bool,
    ) -> Self {
        PyArrayDefinition {
            inner: CoreDefinition::Circular(CircularArray::new(
                uid,
                guide_circle_id.0,
                members
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(slot, eids)| {
                        (slot, eids.into_iter().map(|id| id.0).collect())
                    })
                    .collect(),
                count,
                total_angle_deg,
                rotate_copies,
            )),
        }
    }

    #[staticmethod]
    #[pyo3(signature = (
        uid,
        guide_circle_id,
        members = None,
        count = 6,
        rotate_copies = true,
        path_entity_id = -1,
        align_to_tangent = true,
        offset_to_start = 0.0,
        spacing = 0.0,
        template_anchor = None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn curve_along(
        uid: String,
        guide_circle_id: EntityId,
        members: Option<Members>,
        count: u32,
        rotate_copies: bool,
        path_entity_id: i64,
        align_to_tangent: bool,
        offset_to_start: f64,
        spacing: f64,
        template_anchor: Anchor,
    ) -> Self {
        PyArrayDefinition {
            inner: CoreDefinition::CurveAlong(CurveAlongArray::new(
                uid,
                guide_circle_id.0,
                members
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(slot, eids)| {
                        (slot, eids.into_iter().map(|id| id.0).collect())
                    })
                    .collect(),
                count,
                rotate_copies,
                path_entity_id,
                align_to_tangent,
                offset_to_start,
                spacing,
                template_anchor,
            )),
        }
    }

    /// Array mode: ``circular`` or ``curve_along``.
    #[getter]
    fn mode(&self) -> &'static str {
        self.inner.mode().as_str()
    }

    /// Unique array id.
    #[getter]
    fn uid(&self) -> String {
        self.inner.common().uid.clone()
    }

    #[setter]
    fn set_uid(&mut self, value: String) {
        self.inner.common_mut().uid = value;
    }

    /// Guide circle or guide path entity id.
    #[getter]
    fn guide_circle_id(&self) -> EntityId {
        EntityId(self.inner.common().guide_circle_id)
    }

    #[setter]
    fn set_guide_circle_id(&mut self, value: EntityId) {
        self.inner.common_mut().guide_circle_id = value.0;
    }

    /// Array members as ``(slot, [entity_id, ...])`` pairs.
    #[getter]
    fn members(&self) -> Vec<(i64, Vec<EntityId>)> {
        self.inner
            .common()
            .members
            .iter()
            .map(|(slot, eids)| {
                (*slot, eids.iter().copied().map(EntityId).collect())
            })
            .collect()
    }

    #[setter]
    fn set_members(&mut self, value: Members) {
        self.inner.common_mut().members = value
            .into_iter()
            .map(|(slot, eids)| {
                (slot, eids.into_iter().map(|id| id.0).collect())
            })
            .collect();
    }

    /// Standalone member points as ``(slot, [point_id, ...])`` pairs.
    #[getter]
    fn standalone_pids(&self) -> Vec<(i64, Vec<EntityId>)> {
        self.inner
            .common()
            .standalone_pids
            .iter()
            .map(|(slot, pids)| {
                (*slot, pids.iter().copied().map(EntityId).collect())
            })
            .collect()
    }

    #[setter]
    fn set_standalone_pids(&mut self, value: Members) {
        self.inner.common_mut().standalone_pids = value
            .into_iter()
            .map(|(slot, pids)| {
                (slot, pids.into_iter().map(|id| id.0).collect())
            })
            .collect();
    }

    /// Number of members.
    #[getter]
    fn count(&self) -> u32 {
        self.inner.common().count
    }

    #[setter]
    fn set_count(&mut self, value: u32) {
        self.inner.common_mut().count = value;
    }

    /// Whether copies are rotated with the guide.
    #[getter]
    fn rotate_copies(&self) -> bool {
        self.inner.common().rotate_copies
    }

    #[setter]
    fn set_rotate_copies(&mut self, value: bool) {
        self.inner.common_mut().rotate_copies = value;
    }

    /// Circular-array sweep angle.
    #[getter]
    fn total_angle_deg(&self) -> f64 {
        match &self.inner {
            CoreDefinition::Circular(a) => a.total_angle_deg,
            CoreDefinition::CurveAlong(_) => 360.0,
        }
    }

    #[setter]
    fn set_total_angle_deg(&mut self, value: f64) {
        if let CoreDefinition::Circular(a) = &mut self.inner {
            a.total_angle_deg = value;
        }
    }

    /// Curve-along guide path entity id.
    #[getter]
    fn path_entity_id(&self) -> i64 {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.path_entity_id,
            CoreDefinition::Circular(_) => -1,
        }
    }

    #[setter]
    fn set_path_entity_id(&mut self, value: i64) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.path_entity_id = value;
        }
    }

    /// Whether curve-along copies follow the tangent.
    #[getter]
    fn align_to_tangent(&self) -> bool {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.align_to_tangent,
            CoreDefinition::Circular(_) => true,
        }
    }

    #[setter]
    fn set_align_to_tangent(&mut self, value: bool) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.align_to_tangent = value;
        }
    }

    /// Curve-along start offset.
    #[getter]
    fn offset_to_start(&self) -> f64 {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.offset_to_start,
            CoreDefinition::Circular(_) => 0.0,
        }
    }

    #[setter]
    fn set_offset_to_start(&mut self, value: f64) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.offset_to_start = value;
        }
    }

    /// Curve-along spacing.
    #[getter]
    fn spacing(&self) -> f64 {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.spacing,
            CoreDefinition::Circular(_) => 0.0,
        }
    }

    #[setter]
    fn set_spacing(&mut self, value: f64) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.spacing = value;
        }
    }

    /// Curve-along template anchor.
    #[getter]
    fn template_anchor(&self) -> Anchor {
        match &self.inner {
            CoreDefinition::CurveAlong(a) => a.template_anchor,
            CoreDefinition::Circular(_) => None,
        }
    }

    #[setter]
    fn set_template_anchor(&mut self, value: Anchor) {
        if let CoreDefinition::CurveAlong(a) = &mut self.inner {
            a.template_anchor = value;
        }
    }

    /// Creates a copy of the definition.
    fn copy(&self) -> PyArrayDefinition {
        PyArrayDefinition {
            inner: self.inner.clone(),
        }
    }

    /// Serializes the definition.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let value = self.inner.to_json();
        let dict = PyDict::new(py);
        if let serde_json::Value::Object(map) = value {
            for (key, item) in map {
                dict.set_item(key, value_to_py(py, &item)?)?;
            }
        }
        Ok(dict)
    }

    /// Deserializes a definition.
    #[staticmethod]
    fn from_dict(dict: &Bound<'_, PyDict>) -> PyResult<Self> {
        let value = value_from_dict(dict)?;
        let inner = CoreDefinition::from_json(&value).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(
                "unsupported sketch array definition",
            )
        })?;
        Ok(PyArrayDefinition { inner })
    }

    /// Captures editable state for undo.
    fn snapshot<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        value_to_py(py, &self.inner.snapshot())
    }

    /// Restores editable state from a snapshot.
    fn restore(&mut self, state: &Bound<'_, PyAny>) -> PyResult<()> {
        let dict = state.cast::<PyDict>()?;
        let value = value_from_dict(dict)?;
        self.inner.restore(&value);
        Ok(())
    }

    fn __copy__(&self) -> PyArrayDefinition {
        self.copy()
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let arrays_mod = PyModule::new(py, "arrays")?;

    arrays_mod
        .setattr("__doc__", "Persistent definitions of sketch arrays.")?;
    arrays_mod.add_class::<PyArrayDefinition>()?;
    arrays_mod.add_class::<PyInstancePlacement>()?;
    arrays_mod.add_class::<PyCircularArrayStrategy>()?;
    arrays_mod.add_class::<PyCurveAlongArrayStrategy>()?;

    m.add_submodule(&arrays_mod)?;

    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("raygeo.sketch.arrays", &arrays_mod)?;

    Ok(())
}
